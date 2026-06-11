use crate::error::AppError;
use crate::remote::ssh::{build_ssh_serve_args, configure_password_auth_for_tokio};
use crate::remote::types::{
    RemoteCommandError, RemoteConnectionSecret, RemoteHostProfile, RemoteSessionState,
    RemoteSessionStatus,
};
use once_cell::sync::Lazy;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

// Tool installs and upgrades can legitimately take several minutes on remote
// servers. Timing out the helper session also tears down any in-process remote
// routing runtime, so keep this higher than normal UI request timeouts.
const REMOTE_SESSION_REQUEST_TIMEOUT: Duration = Duration::from_secs(15 * 60);

static REMOTE_SESSION_MANAGER: Lazy<RemoteSessionManager> =
    Lazy::new(RemoteSessionManager::default);

pub fn remote_session_manager() -> &'static RemoteSessionManager {
    &REMOTE_SESSION_MANAGER
}

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
    sessions: Arc<Mutex<HashMap<String, ManagedRemoteSession>>>,
}

struct ManagedRemoteSession {
    status: RemoteSessionStatus,
    executor: Option<Arc<dyn RemoteSessionExecutor>>,
}

trait RemoteSessionExecutor: Send + Sync {
    fn execute<'a>(
        &'a self,
        request_id: &'a str,
        command: &'a [String],
    ) -> Pin<Box<dyn Future<Output = Result<Value, RemoteSessionError>> + Send + 'a>>;

    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
}

impl RemoteSessionManager {
    pub async fn status(&self, profile_id: &str) -> RemoteSessionStatus {
        self.sessions
            .lock()
            .await
            .get(profile_id)
            .map(|session| session.status.clone())
            .unwrap_or(RemoteSessionStatus {
                profile_id: profile_id.to_string(),
                state: RemoteSessionState::Idle,
                last_error: None,
                active_request_id: None,
            })
    }

    #[allow(dead_code)]
    async fn set_status(&self, status: RemoteSessionStatus) {
        let mut sessions = self.sessions.lock().await;
        sessions
            .entry(status.profile_id.clone())
            .and_modify(|session| session.status = status.clone())
            .or_insert(ManagedRemoteSession {
                status,
                executor: None,
            });
    }

    pub async fn close(&self, profile_id: &str) -> bool {
        let session = self.sessions.lock().await.remove(profile_id);
        if let Some(session) = session {
            if let Some(executor) = session.executor {
                executor.close().await;
            }
            true
        } else {
            false
        }
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

        let executor = self
            .get_or_start_executor(&profile, secret.as_ref())
            .await?;
        self.set_status(RemoteSessionStatus {
            profile_id: profile.id.clone(),
            state: RemoteSessionState::Busy,
            last_error: None,
            active_request_id: Some(request_id.clone()),
        })
        .await;

        let result = tokio::time::timeout(
            REMOTE_SESSION_REQUEST_TIMEOUT,
            executor.execute(&request_id, &helper_args),
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
                let mut sessions = self.sessions.lock().await;
                sessions.insert(
                    profile.id.clone(),
                    ManagedRemoteSession {
                        status: RemoteSessionStatus {
                            profile_id: profile.id.clone(),
                            state: RemoteSessionState::Failed,
                            last_error: Some(error.to_string()),
                            active_request_id: None,
                        },
                        executor: None,
                    },
                );
            }
        }

        result
    }

    async fn get_or_start_executor(
        &self,
        profile: &RemoteHostProfile,
        secret: Option<&RemoteConnectionSecret>,
    ) -> Result<Arc<dyn RemoteSessionExecutor>, AppError> {
        if let Some(executor) = self
            .sessions
            .lock()
            .await
            .get(&profile.id)
            .and_then(|session| session.executor.clone())
        {
            return Ok(executor);
        }

        self.set_status(RemoteSessionStatus {
            profile_id: profile.id.clone(),
            state: RemoteSessionState::Connecting,
            last_error: None,
            active_request_id: None,
        })
        .await;

        let process = RemoteSessionProcess::start(profile, secret).await?;
        let executor: Arc<dyn RemoteSessionExecutor> = Arc::new(Mutex::new(process));
        let mut sessions = self.sessions.lock().await;
        sessions.insert(
            profile.id.clone(),
            ManagedRemoteSession {
                status: RemoteSessionStatus {
                    profile_id: profile.id.clone(),
                    state: RemoteSessionState::Ready,
                    last_error: None,
                    active_request_id: None,
                },
                executor: Some(executor.clone()),
            },
        );
        Ok(executor)
    }
}

struct RemoteSessionProcess {
    child: Child,
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
            child,
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

impl RemoteSessionExecutor for Mutex<RemoteSessionProcess> {
    fn execute<'a>(
        &'a self,
        request_id: &'a str,
        command: &'a [String],
    ) -> Pin<Box<dyn Future<Output = Result<Value, RemoteSessionError>> + Send + 'a>> {
        Box::pin(async move { self.lock().await.execute_value(request_id, command).await })
    }

    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let mut process = self.lock().await;
            let _ = process.child.kill().await;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    struct FakeExecutor {
        response: Value,
        execute_count: Arc<AtomicUsize>,
        closed: Arc<AtomicBool>,
    }

    impl FakeExecutor {
        fn new(response: Value) -> Self {
            Self {
                response,
                execute_count: Arc::new(AtomicUsize::new(0)),
                closed: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    impl RemoteSessionExecutor for FakeExecutor {
        fn execute<'a>(
            &'a self,
            _request_id: &'a str,
            _command: &'a [String],
        ) -> Pin<Box<dyn Future<Output = Result<Value, RemoteSessionError>> + Send + 'a>> {
            Box::pin(async move {
                self.execute_count.fetch_add(1, Ordering::SeqCst);
                Ok(self.response.clone())
            })
        }

        fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
            Box::pin(async move {
                self.closed.store(true, Ordering::SeqCst);
            })
        }
    }

    fn profile() -> RemoteHostProfile {
        RemoteHostProfile {
            id: "prod".to_string(),
            name: "Production".to_string(),
            host: "example.com".to_string(),
            port: 22,
            username: "ccswitch".to_string(),
            auth_method: crate::remote::types::RemoteAuthMethod::SshAgent,
            helper_path: "/usr/local/bin/cc-switch-helper".to_string(),
            created_at: 1,
            updated_at: 1,
        }
    }

    #[tokio::test]
    async fn execute_json_reuses_existing_executor() {
        let manager = RemoteSessionManager::default();
        let executor = Arc::new(FakeExecutor::new(serde_json::json!({"value": 42})));
        manager.sessions.lock().await.insert(
            "prod".to_string(),
            ManagedRemoteSession {
                status: RemoteSessionStatus {
                    profile_id: "prod".to_string(),
                    state: RemoteSessionState::Ready,
                    last_error: None,
                    active_request_id: None,
                },
                executor: Some(executor.clone()),
            },
        );

        let value: Value = manager
            .execute_json(profile(), None, vec!["status".to_string()])
            .await
            .expect("session value");

        assert_eq!(value, serde_json::json!({"value": 42}));
        assert_eq!(executor.execute_count.load(Ordering::SeqCst), 1);
        assert_eq!(
            manager.status("prod").await.state,
            RemoteSessionState::Ready
        );
    }

    #[tokio::test]
    async fn close_existing_session_closes_executor() {
        let manager = RemoteSessionManager::default();
        let executor = Arc::new(FakeExecutor::new(Value::Null));
        manager.sessions.lock().await.insert(
            "prod".to_string(),
            ManagedRemoteSession {
                status: RemoteSessionStatus {
                    profile_id: "prod".to_string(),
                    state: RemoteSessionState::Ready,
                    last_error: None,
                    active_request_id: None,
                },
                executor: Some(executor.clone()),
            },
        );

        assert!(manager.close("prod").await);
        assert!(executor.closed.load(Ordering::SeqCst));
        assert_eq!(manager.status("prod").await.state, RemoteSessionState::Idle);
    }
}
