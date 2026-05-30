use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusPayload {
    pub version: String,
    pub platform: String,
    pub capabilities: Vec<String>,
}

pub fn status_payload() -> StatusPayload {
    StatusPayload {
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        capabilities: vec![
            "providers".to_string(),
            "mcp".to_string(),
            "prompts".to_string(),
            "skills".to_string(),
            "import-export".to_string(),
        ],
    }
}
