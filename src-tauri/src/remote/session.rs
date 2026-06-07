use crate::error::AppError;
use crate::remote::ssh::{build_ssh_serve_args, configure_password_auth_for_tokio};
use crate::remote::types::{
    RemoteCommandError, RemoteConnectionSecret, RemoteHostProfile, RemoteSessionState,
    RemoteSessionStatus,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

const REMOTE_SESSION_REQUEST_TIMEOUT: Duration = Duration::from_secs(45);

#[cfg(unix)]
type PasswordAuthGuard = tempfile::TempPath;

#[cfg(not(unix))]
type PasswordAuthGuard = ();

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoteSessionRequestLine<'a> {
    id: &'a str,
    command: &'a [String],
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSessionResponseLine {
    pub id: String,
    pub ok: bool,
    pub data: Option<Value>,
    pub error: Option<RemoteCommandError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteSessionError {
    InvalidJson(String),
    MissingData,
    CommandFailed(RemoteCommandError),
    Io(String),
    Timeout,
    Closed,
}

impl std::fmt::Display for RemoteSessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(message) => {
                write!(f, "Remote session returned invalid JSON: {message}")
            }
            Self::MissingData => write!(f, "Remote session returned ok without data"),
            Self::CommandFailed(error) => write!(f, "{}: {}", error.code, error.message),
            Self::Io(message) => write!(f, "Remote session I/O failed: {message}"),
            Self::Timeout => write!(f, "Remote session command timed out"),
            Self::Closed => write!(
                f,
                "Remote helper session closed before returning a response"
            ),
        }
    }
}

impl std::error::Error for RemoteSessionError {}

pub fn parse_session_response_line(
    line: &str,
) -> Result<RemoteSessionResponseLine, RemoteSessionError> {
    serde_json::from_str(line).map_err(|error| RemoteSessionError::InvalidJson(error.to_string()))
}

pub fn build_session_request_line(
    id: &str,
    command: &[String],
) -> Result<String, RemoteSessionError> {
    serde_json::to_string(&RemoteSessionRequestLine { id, command })
        .map_err(|error| RemoteSessionError::InvalidJson(error.to_string()))
}

#[derive(Default)]
pub struct RemoteSessionManager {
    sessions: Arc<Mutex<HashMap<String, RemoteSessionStatus>>>,
}

impl RemoteSessionManager {
    pub async fn status(&self, profile_id: &str) -> RemoteSessionStatus {
        self.sessions
            .lock()
            .await
            .get(profile_id)
            .cloned()
            .unwrap_or(RemoteSessionStatus {
                profile_id: profile_id.to_string(),
                state: RemoteSessionState::Idle,
                last_error: None,
                active_request_id: None,
            })
    }

    #[allow(dead_code)]
    async fn set_status(&self, status: RemoteSessionStatus) {
        self.sessions
            .lock()
            .await
            .insert(status.profile_id.clone(), status);
    }

    pub async fn execute_json<T>(
        &self,
        profile: RemoteHostProfile,
        secret: Option<RemoteConnectionSecret>,
        helper_args: Vec<String>,
    ) -> Result<T, AppError>
    where
        T: DeserializeOwned + Send + 'static,
    {
        let value = self.execute_value(profile, secret, helper_args).await?;
        serde_json::from_value(value).map_err(|error| {
            AppError::Message(format!("Remote session returned invalid data: {error}"))
        })
    }

    async fn execute_value(
        &self,
        profile: RemoteHostProfile,
        secret: Option<RemoteConnectionSecret>,
        helper_args: Vec<String>,
    ) -> Result<Value, AppError> {
        let request_id = uuid::Uuid::new_v4().to_string();
        self.set_status(RemoteSessionStatus {
            profile_id: profile.id.clone(),
            state: RemoteSessionState::Connecting,
            last_error: None,
            active_request_id: Some(request_id.clone()),
        })
        .await;

        let mut process = RemoteSessionProcess::start(&profile, secret.as_ref()).await?;
        self.set_status(RemoteSessionStatus {
            profile_id: profile.id.clone(),
            state: RemoteSessionState::Busy,
            last_error: None,
            active_request_id: Some(request_id.clone()),
        })
        .await;

        let result = tokio::time::timeout(
            REMOTE_SESSION_REQUEST_TIMEOUT,
            process.execute_value(&request_id, &helper_args),
        )
        .await
        .map_err(|_| AppError::Message(RemoteSessionError::Timeout.to_string()))?
        .map_err(|error| AppError::Message(error.to_string()));

        match &result {
            Ok(_) => {
                self.set_status(RemoteSessionStatus {
                    profile_id: profile.id.clone(),
                    state: RemoteSessionState::Ready,
                    last_error: None,
                    active_request_id: None,
                })
                .await;
            }
            Err(error) => {
                self.set_status(RemoteSessionStatus {
                    profile_id: profile.id.clone(),
                    state: RemoteSessionState::Failed,
                    last_error: Some(error.to_string()),
                    active_request_id: None,
                })
                .await;
            }
        }

        result
    }
}

struct RemoteSessionProcess {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    _askpass: Option<PasswordAuthGuard>,
}

impl RemoteSessionProcess {
    async fn start(
        profile: &RemoteHostProfile,
        secret: Option<&RemoteConnectionSecret>,
    ) -> Result<Self, AppError> {
        let mut command = Command::new("ssh");
        command.args(build_ssh_serve_args(profile));
        let askpass = configure_password_auth_for_tokio(profile, secret, &mut command)?;
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|error| {
            AppError::Message(format!("Failed to start remote helper session: {error}"))
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            AppError::Message("Remote helper session stdin unavailable".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            AppError::Message("Remote helper session stdout unavailable".to_string())
        })?;

        Ok(Self {
            _child: child,
            stdin,
            stdout: BufReader::new(stdout),
            _askpass: askpass,
        })
    }

    async fn execute_value(
        &mut self,
        request_id: &str,
        command: &[String],
    ) -> Result<Value, RemoteSessionError> {
        let mut request = build_session_request_line(request_id, command)?;
        request.push('\n');
        self.stdin
            .write_all(request.as_bytes())
            .await
            .map_err(|error| RemoteSessionError::Io(error.to_string()))?;
        self.stdin
            .flush()
            .await
            .map_err(|error| RemoteSessionError::Io(error.to_string()))?;

        let mut line = String::new();
        let read = self
            .stdout
            .read_line(&mut line)
            .await
            .map_err(|error| RemoteSessionError::Io(error.to_string()))?;
        if read == 0 {
            return Err(RemoteSessionError::Closed);
        }

        let response = parse_session_response_line(line.trim())?;
        if response.id != request_id {
            return Err(RemoteSessionError::Io(format!(
                "Remote session response id mismatch: expected {request_id}, got {}",
                response.id
            )));
        }
        if response.ok {
            response.data.ok_or(RemoteSessionError::MissingData)
        } else {
            Err(RemoteSessionError::CommandFailed(response.error.unwrap_or(
                RemoteCommandError {
                    code: "remote_error".to_string(),
                    message: "Remote helper command failed".to_string(),
                },
            )))
        }
    }
}
