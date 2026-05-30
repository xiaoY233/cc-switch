use crate::{AppState, AppType, Database, Provider, ProviderService};
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

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

pub fn list_providers(app: AppType) -> Result<serde_json::Value, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    let providers = ProviderService::list(&state, app).map_err(|e| e.to_string())?;
    let mut value = serde_json::to_value(providers).map_err(|e| e.to_string())?;
    redact_secret_values(&mut value);
    Ok(value)
}

pub fn current_provider(app: AppType) -> Result<String, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    ProviderService::current(&state, app).map_err(|e| e.to_string())
}

pub fn switch_provider(app: AppType, id: &str) -> Result<crate::services::SwitchResult, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    ProviderService::switch(&state, app, id).map_err(|e| e.to_string())
}

pub fn add_provider(app: AppType, provider_json: &str, add_to_live: bool) -> Result<bool, String> {
    let provider: Provider = serde_json::from_str(provider_json).map_err(|e| e.to_string())?;
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    ProviderService::add(&state, app, provider, add_to_live).map_err(|e| e.to_string())
}

pub fn update_provider(
    app: AppType,
    provider_json: &str,
    original_id: Option<&str>,
) -> Result<bool, String> {
    let provider: Provider = serde_json::from_str(provider_json).map_err(|e| e.to_string())?;
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    ProviderService::update(&state, app, original_id, provider).map_err(|e| e.to_string())
}

pub fn delete_provider(app: AppType, id: &str) -> Result<bool, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    ProviderService::delete(&state, app, id)
        .map(|_| true)
        .map_err(|e| e.to_string())
}

fn redact_secret_values(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if is_secret_key(key) {
                    *child = Value::String("[redacted]".to_string());
                } else {
                    redact_secret_values(child);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_secret_values(item);
            }
        }
        _ => {}
    }
}

fn is_secret_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    normalized == "key"
        || normalized.contains("api_key")
        || normalized.contains("apikey")
        || normalized.contains("token")
        || normalized.contains("secret")
        || normalized.contains("password")
        || normalized.contains("credential")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn provider_output_redacts_nested_secret_values() {
        let mut value = json!({
            "provider": {
                "settingsConfig": {
                    "env": {
                        "ANTHROPIC_AUTH_TOKEN": "sk-secret",
                        "ANTHROPIC_BASE_URL": "https://example.com",
                        "OPENAI_API_KEY": "sk-openai"
                    }
                },
                "meta": {
                    "usage_script": {
                        "accessToken": "user-token",
                        "baseUrl": "https://usage.example.com"
                    }
                }
            }
        });

        redact_secret_values(&mut value);

        assert_eq!(
            value["provider"]["settingsConfig"]["env"]["ANTHROPIC_AUTH_TOKEN"],
            "[redacted]"
        );
        assert_eq!(
            value["provider"]["settingsConfig"]["env"]["OPENAI_API_KEY"],
            "[redacted]"
        );
        assert_eq!(
            value["provider"]["meta"]["usage_script"]["accessToken"],
            "[redacted]"
        );
        assert_eq!(
            value["provider"]["settingsConfig"]["env"]["ANTHROPIC_BASE_URL"],
            "https://example.com"
        );
    }
}
