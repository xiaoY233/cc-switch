use crate::app_config::{InstalledSkill, UnmanagedSkill};
use crate::prompt::Prompt;
use crate::services::skill::{
    DiscoverableSkill, ImportSkillSelection, SkillBackupEntry, SkillRepo, SkillService,
    SkillUninstallResult, SkillUpdateInfo,
};
use crate::services::ProviderSortUpdate;
use crate::{
    AppError, AppState, AppType, Database, McpServer, McpService, PromptService, Provider,
    ProviderService,
};
use indexmap::IndexMap;
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;

const REDACTED_SECRET_SENTINEL: &str = "[redacted]";

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
            "openclaw".to_string(),
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
    redact_provider_map_secret_values(&mut value);
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
    let mut provider: Provider = serde_json::from_str(provider_json).map_err(|e| e.to_string())?;
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    let provider_id = original_id.unwrap_or(provider.id.as_str());
    if let Some(existing_provider) = state
        .db
        .get_provider_by_id(provider_id, app.as_str())
        .map_err(|e| e.to_string())?
    {
        restore_redacted_secret_values(&existing_provider, &mut provider)
            .map_err(|e| e.to_string())?;
    }
    ProviderService::update(&state, app, original_id, provider).map_err(|e| e.to_string())
}

pub fn delete_provider(app: AppType, id: &str) -> Result<bool, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    ProviderService::delete(&state, app, id)
        .map(|_| true)
        .map_err(|e| e.to_string())
}

pub fn sort_providers(app: AppType, updates_json: &str) -> Result<bool, String> {
    let updates: Vec<ProviderSortUpdate> =
        serde_json::from_str(updates_json).map_err(|e| e.to_string())?;
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    ProviderService::update_sort_order(&state, app, updates).map_err(|e| e.to_string())
}

pub fn import_providers(app: AppType) -> Result<bool, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    match app {
        AppType::OpenCode => crate::services::provider::import_opencode_providers_from_live(&state)
            .map(|count| count > 0)
            .or_else(live_config_missing_as_false),
        AppType::OpenClaw => crate::services::provider::import_openclaw_providers_from_live(&state)
            .map(|count| count > 0)
            .or_else(live_config_missing_as_false),
        AppType::Hermes => crate::services::provider::import_hermes_providers_from_live(&state)
            .map(|count| count > 0)
            .or_else(live_config_missing_as_false),
        AppType::ClaudeDesktop => import_claude_desktop_providers_from_claude(&state),
        _ => crate::commands::import_default_config_internal(&state, app)
            .or_else(live_config_missing_as_false),
    }
}

pub fn export_database_sql() -> Result<String, String> {
    let db = Database::init().map_err(|e| e.to_string())?;
    db.export_sql_string().map_err(|e| e.to_string())
}

pub fn import_database_sql_b64(encoded_sql: &str) -> Result<Value, String> {
    let sql_bytes = {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        STANDARD.decode(encoded_sql).map_err(|e| e.to_string())?
    };
    let sql = String::from_utf8(sql_bytes).map_err(|e| e.to_string())?;
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let backup_id = db.import_sql_string(&sql).map_err(|e| e.to_string())?;
    let sync_warning = {
        let state = AppState::new(db);
        ProviderService::sync_current_to_live(&state)
            .and_then(|_| crate::settings::reload_settings())
            .err()
            .map(|e| e.to_string())
    };

    let mut payload = json!({
        "success": true,
        "message": "SQL imported successfully",
        "backupId": backup_id
    });
    if let Some(warning) = sync_warning {
        payload["warning"] = Value::String(warning);
    }
    Ok(payload)
}

fn live_config_missing_as_false(error: AppError) -> Result<bool, String> {
    match &error {
        AppError::Localized { key, .. }
            if matches!(
                *key,
                "claude.live.missing"
                    | "codex.live.missing"
                    | "gemini.live.missing"
                    | "opencode.config.missing"
                    | "openclaw.config.missing"
                    | "hermes.config.missing"
            ) =>
        {
            Ok(false)
        }
        _ => Err(error.to_string()),
    }
}

fn import_claude_desktop_providers_from_claude(state: &AppState) -> Result<bool, String> {
    let claude_providers = state
        .db
        .get_all_providers(AppType::Claude.as_str())
        .map_err(|e| e.to_string())?;
    let existing_ids = state
        .db
        .get_provider_ids(AppType::ClaudeDesktop.as_str())
        .map_err(|e| e.to_string())?;

    let mut imported = 0usize;
    for provider in claude_providers.values() {
        if existing_ids.contains(&provider.id) {
            continue;
        }

        let mut desktop_provider = provider.clone();
        desktop_provider.in_failover_queue = false;
        let meta = desktop_provider.meta.get_or_insert_with(Default::default);

        if crate::claude_desktop_config::is_compatible_direct_provider(provider)
            && crate::commands::claude_provider_models_are_claude_safe(provider)
        {
            meta.claude_desktop_mode = Some(crate::provider::ClaudeDesktopMode::Direct);
        } else if let Some(routes) = crate::commands::suggested_claude_desktop_routes(provider) {
            meta.claude_desktop_mode = Some(crate::provider::ClaudeDesktopMode::Proxy);
            meta.claude_desktop_model_routes = routes;
        } else {
            continue;
        }

        state
            .db
            .save_provider(AppType::ClaudeDesktop.as_str(), &desktop_provider)
            .map_err(|e| e.to_string())?;
        imported += 1;
    }

    let _ = state.db.ensure_official_seed_by_id(
        crate::database::CLAUDE_DESKTOP_OFFICIAL_PROVIDER_ID,
        AppType::ClaudeDesktop,
    );

    Ok(imported > 0)
}

pub fn get_openclaw_default_model(
) -> Result<Option<crate::openclaw_config::OpenClawDefaultModel>, String> {
    crate::openclaw_config::get_default_model().map_err(|e| e.to_string())
}

pub fn set_openclaw_default_model(
    model_json: &str,
) -> Result<crate::openclaw_config::OpenClawWriteOutcome, String> {
    let model: crate::openclaw_config::OpenClawDefaultModel =
        serde_json::from_str(model_json).map_err(|e| e.to_string())?;
    crate::openclaw_config::set_default_model(&model).map_err(|e| e.to_string())
}

pub fn get_openclaw_env() -> Result<crate::openclaw_config::OpenClawEnvConfig, String> {
    crate::openclaw_config::get_env_config().map_err(|e| e.to_string())
}

pub fn set_openclaw_env(
    env_json: &str,
) -> Result<crate::openclaw_config::OpenClawWriteOutcome, String> {
    let env: crate::openclaw_config::OpenClawEnvConfig =
        serde_json::from_str(env_json).map_err(|e| e.to_string())?;
    crate::openclaw_config::set_env_config(&env).map_err(|e| e.to_string())
}

pub fn get_openclaw_tools() -> Result<crate::openclaw_config::OpenClawToolsConfig, String> {
    crate::openclaw_config::get_tools_config().map_err(|e| e.to_string())
}

pub fn set_openclaw_tools(
    tools_json: &str,
) -> Result<crate::openclaw_config::OpenClawWriteOutcome, String> {
    let tools: crate::openclaw_config::OpenClawToolsConfig =
        serde_json::from_str(tools_json).map_err(|e| e.to_string())?;
    crate::openclaw_config::set_tools_config(&tools).map_err(|e| e.to_string())
}

pub fn get_openclaw_agents_defaults(
) -> Result<Option<crate::openclaw_config::OpenClawAgentsDefaults>, String> {
    crate::openclaw_config::get_agents_defaults().map_err(|e| e.to_string())
}

pub fn set_openclaw_agents_defaults(
    defaults_json: &str,
) -> Result<crate::openclaw_config::OpenClawWriteOutcome, String> {
    let defaults: crate::openclaw_config::OpenClawAgentsDefaults =
        serde_json::from_str(defaults_json).map_err(|e| e.to_string())?;
    crate::openclaw_config::set_agents_defaults(&defaults).map_err(|e| e.to_string())
}

pub fn list_mcp_servers() -> Result<IndexMap<String, McpServer>, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    McpService::get_all_servers(&state).map_err(|e| e.to_string())
}

pub fn upsert_mcp_server(server_json: &str) -> Result<(), String> {
    let server: McpServer = serde_json::from_str(server_json).map_err(|e| e.to_string())?;
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    McpService::upsert_server(&state, server).map_err(|e| e.to_string())
}

pub fn delete_mcp_server(id: &str) -> Result<bool, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    McpService::delete_server(&state, id).map_err(|e| e.to_string())
}

pub fn toggle_mcp_app(server_id: &str, app: AppType, enabled: bool) -> Result<(), String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    McpService::toggle_app(&state, server_id, app, enabled).map_err(|e| e.to_string())
}

pub fn import_mcp_from_apps() -> Result<usize, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    let mut total = 0;
    total += McpService::import_from_claude(&state).unwrap_or(0);
    total += McpService::import_from_codex(&state).unwrap_or(0);
    total += McpService::import_from_gemini(&state).unwrap_or(0);
    total += McpService::import_from_opencode(&state).unwrap_or(0);
    total += McpService::import_from_hermes(&state).unwrap_or(0);
    Ok(total)
}

pub fn list_prompts(app: AppType) -> Result<IndexMap<String, Prompt>, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    PromptService::get_prompts(&state, app).map_err(|e| e.to_string())
}

pub fn upsert_prompt(app: AppType, id: &str, prompt_json: &str) -> Result<(), String> {
    let prompt: Prompt = serde_json::from_str(prompt_json).map_err(|e| e.to_string())?;
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    PromptService::upsert_prompt(&state, app, id, prompt).map_err(|e| e.to_string())
}

pub fn delete_prompt(app: AppType, id: &str) -> Result<(), String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    PromptService::delete_prompt(&state, app, id).map_err(|e| e.to_string())
}

pub fn enable_prompt(app: AppType, id: &str) -> Result<(), String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    PromptService::enable_prompt(&state, app, id).map_err(|e| e.to_string())
}

pub fn import_prompt_from_file(app: AppType) -> Result<String, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    PromptService::import_from_file(&state, app).map_err(|e| e.to_string())
}

pub fn current_prompt_file_content(app: AppType) -> Result<Option<String>, String> {
    PromptService::get_current_file_content(app).map_err(|e| e.to_string())
}

pub fn list_installed_skills() -> Result<Vec<InstalledSkill>, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    SkillService::get_all_installed(&db).map_err(|e| e.to_string())
}

pub fn list_skill_backups() -> Result<Vec<SkillBackupEntry>, String> {
    SkillService::list_backups().map_err(|e| e.to_string())
}

pub fn delete_skill_backup(backup_id: &str) -> Result<bool, String> {
    SkillService::delete_backup(backup_id).map_err(|e| e.to_string())?;
    Ok(true)
}

pub fn install_skill_unified(
    skill_json: &str,
    current_app: AppType,
) -> Result<InstalledSkill, String> {
    let skill: DiscoverableSkill = serde_json::from_str(skill_json).map_err(|e| e.to_string())?;
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let service = SkillService::new();
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    runtime
        .block_on(service.install(&db, &skill, &current_app))
        .map_err(|e| e.to_string())
}

pub fn uninstall_skill_unified(id: &str) -> Result<SkillUninstallResult, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    SkillService::uninstall(&db, id).map_err(|e| e.to_string())
}

pub fn restore_skill_backup(
    backup_id: &str,
    current_app: AppType,
) -> Result<InstalledSkill, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    SkillService::restore_from_backup(&db, backup_id, &current_app).map_err(|e| e.to_string())
}

pub fn toggle_skill_app(id: &str, app: AppType, enabled: bool) -> Result<bool, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    SkillService::toggle_app(&db, id, &app, enabled).map_err(|e| e.to_string())?;
    Ok(true)
}

pub fn scan_unmanaged_skills() -> Result<Vec<UnmanagedSkill>, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    SkillService::scan_unmanaged(&db).map_err(|e| e.to_string())
}

pub fn import_skills_from_apps(imports_json: &str) -> Result<Vec<InstalledSkill>, String> {
    let imports: Vec<ImportSkillSelection> =
        serde_json::from_str(imports_json).map_err(|e| e.to_string())?;
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    SkillService::import_from_apps(&db, imports).map_err(|e| e.to_string())
}

pub fn discover_available_skills() -> Result<Vec<DiscoverableSkill>, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let repos = db.get_skill_repos().map_err(|e| e.to_string())?;
    let service = SkillService::new();
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    runtime
        .block_on(service.discover_available(repos))
        .map_err(|e| e.to_string())
}

pub fn check_skill_updates() -> Result<Vec<SkillUpdateInfo>, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let service = SkillService::new();
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    runtime
        .block_on(service.check_updates(&db))
        .map_err(|e| e.to_string())
}

pub fn update_skill(id: &str) -> Result<InstalledSkill, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    let service = SkillService::new();
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    runtime
        .block_on(service.update_skill(&db, id))
        .map_err(|e| e.to_string())
}

pub fn list_skill_repos() -> Result<Vec<SkillRepo>, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    db.get_skill_repos().map_err(|e| e.to_string())
}

pub fn add_skill_repo(repo_json: &str) -> Result<bool, String> {
    let repo: SkillRepo = serde_json::from_str(repo_json).map_err(|e| e.to_string())?;
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    db.save_skill_repo(&repo).map_err(|e| e.to_string())?;
    Ok(true)
}

pub fn remove_skill_repo(owner: &str, name: &str) -> Result<bool, String> {
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    db.delete_skill_repo(owner, name)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

fn redact_provider_map_secret_values(value: &mut Value) {
    match value {
        Value::Object(providers) => {
            for provider in providers.values_mut() {
                redact_secret_values(provider);
            }
        }
        _ => redact_secret_values(value),
    }
}

fn redact_secret_values(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if is_secret_key(key) {
                    *child = Value::String(REDACTED_SECRET_SENTINEL.to_string());
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

fn restore_redacted_secret_values(
    existing_provider: &Provider,
    provider: &mut Provider,
) -> Result<(), serde_json::Error> {
    let existing = serde_json::to_value(existing_provider)?;
    let mut incoming = serde_json::to_value(&provider)?;
    merge_redacted_secret_values(&existing, &mut incoming);
    *provider = serde_json::from_value(incoming)?;
    Ok(())
}

fn merge_redacted_secret_values(existing: &Value, incoming: &mut Value) {
    match (existing, incoming) {
        (Value::Object(existing_map), Value::Object(incoming_map)) => {
            for (key, incoming_child) in incoming_map.iter_mut() {
                if is_secret_key(key) && is_redacted_sentinel(incoming_child) {
                    if let Some(existing_child) = existing_map.get(key) {
                        *incoming_child = existing_child.clone();
                    }
                } else if let Some(existing_child) = existing_map.get(key) {
                    merge_redacted_secret_values(existing_child, incoming_child);
                }
            }
        }
        (Value::Array(existing_items), Value::Array(incoming_items)) => {
            for (existing_child, incoming_child) in existing_items.iter().zip(incoming_items) {
                merge_redacted_secret_values(existing_child, incoming_child);
            }
        }
        _ => {}
    }
}

fn is_redacted_sentinel(value: &Value) -> bool {
    value.as_str() == Some(REDACTED_SECRET_SENTINEL)
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
            REDACTED_SECRET_SENTINEL
        );
        assert_eq!(
            value["provider"]["settingsConfig"]["env"]["OPENAI_API_KEY"],
            REDACTED_SECRET_SENTINEL
        );
        assert_eq!(
            value["provider"]["meta"]["usage_script"]["accessToken"],
            REDACTED_SECRET_SENTINEL
        );
        assert_eq!(
            value["provider"]["settingsConfig"]["env"]["ANTHROPIC_BASE_URL"],
            "https://example.com"
        );
    }

    #[test]
    fn provider_map_redaction_does_not_treat_provider_ids_as_secret_keys() {
        let mut value = json!({
            "secret-provider": {
                "id": "secret-provider",
                "settingsConfig": {
                    "env": {
                        "ANTHROPIC_AUTH_TOKEN": "sk-secret",
                        "ANTHROPIC_BASE_URL": "https://example.com"
                    }
                }
            }
        });

        redact_provider_map_secret_values(&mut value);

        assert_eq!(value["secret-provider"]["id"], "secret-provider");
        assert_eq!(
            value["secret-provider"]["settingsConfig"]["env"]["ANTHROPIC_AUTH_TOKEN"],
            REDACTED_SECRET_SENTINEL
        );
        assert_eq!(
            value["secret-provider"]["settingsConfig"]["env"]["ANTHROPIC_BASE_URL"],
            "https://example.com"
        );
    }
}
