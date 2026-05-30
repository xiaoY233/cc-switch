use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliEnvelope<T: Serialize> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<CliError>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliError {
    pub code: String,
    pub message: String,
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
