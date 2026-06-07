use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliEnvelope<T: Serialize> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<CliError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CliError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CliServeRequest {
    pub id: String,
    pub command: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CliServeResponse {
    pub id: String,
    pub ok: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<CliError>,
}

pub fn ok<T: Serialize>(data: T) -> CliEnvelope<T> {
    CliEnvelope {
        ok: true,
        data: Some(data),
        error: None,
    }
}

pub fn err<T: Serialize>(code: &str, message: impl Into<String>) -> CliEnvelope<T> {
    CliEnvelope {
        ok: false,
        data: None,
        error: Some(CliError {
            code: code.to_string(),
            message: message.into(),
        }),
    }
}

pub fn serve_ok(id: String, data: serde_json::Value) -> CliServeResponse {
    CliServeResponse {
        id,
        ok: true,
        data: Some(data),
        error: None,
    }
}

pub fn serve_err(id: String, code: &str, message: impl Into<String>) -> CliServeResponse {
    CliServeResponse {
        id,
        ok: false,
        data: None,
        error: Some(CliError {
            code: code.to_string(),
            message: message.into(),
        }),
    }
}
