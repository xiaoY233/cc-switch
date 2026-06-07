use crate::remote::types::{RemoteCommandError, RemoteSessionState, RemoteSessionStatus};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

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
}
