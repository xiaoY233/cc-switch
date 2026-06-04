use crate::app_config::{InstalledSkill, UnmanagedSkill};
use crate::prompt::Prompt;
use crate::services::skill::{
    DiscoverableSkill, ImportSkillSelection, SkillBackupEntry, SkillRepo, SkillService,
    SkillStorageLocation, SkillUninstallResult, SkillUpdateInfo,
};
use crate::services::ProviderSortUpdate;
use crate::{
    AppError, AppState, AppType, Database, McpServer, McpService, PromptService, Provider,
    ProviderService,
};
use indexmap::IndexMap;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::process::Command;
use std::sync::Arc;

const REDACTED_SECRET_SENTINEL: &str = "[redacted]";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusPayload {
    pub version: String,
    pub build: Option<String>,
    pub platform: String,
    pub arch: String,
    pub capabilities: Vec<String>,
}

pub fn status_payload() -> StatusPayload {
    StatusPayload {
        version: env!("CARGO_PKG_VERSION").to_string(),
        build: option_env!("CC_SWITCH_REMOTE_HELPER_BUILD")
            .filter(|value| !value.trim().is_empty() && *value != "unknown")
            .map(str::to_string),
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        capabilities: vec![
            "providers".to_string(),
            "openclaw".to_string(),
            "mcp".to_string(),
            "prompts".to_string(),
            "skills".to_string(),
            "sessions".to_string(),
            "hermes-memory".to_string(),
            "import-export".to_string(),
            "tools".to_string(),
            "settings".to_string(),
            "plugin".to_string(),
        ],
    }
}

pub fn get_settings() -> crate::settings::AppSettings {
    crate::settings::get_settings_for_frontend()
}

pub fn save_settings(settings_json: &str) -> Result<bool, String> {
    let settings: crate::settings::AppSettings =
        serde_json::from_str(settings_json).map_err(|e| e.to_string())?;
    crate::settings::update_settings(settings).map_err(|e| e.to_string())?;
    Ok(true)
}

pub fn migrate_skill_storage(
    target: &str,
) -> Result<crate::services::skill::MigrationResult, String> {
    let target = parse_skill_storage_location(target)?;
    let db = Arc::new(Database::init().map_err(|e| e.to_string())?);
    SkillService::migrate_storage(&db, target).map_err(|e| e.to_string())
}

pub fn apply_claude_plugin_config(official: &str) -> Result<bool, String> {
    let official = parse_bool(official)?;
    if official {
        crate::claude_plugin::clear_claude_config().map_err(|e| e.to_string())
    } else {
        crate::claude_plugin::write_claude_config().map_err(|e| e.to_string())
    }
}

pub fn set_claude_onboarding_skip(enabled: &str) -> Result<bool, String> {
    if parse_bool(enabled)? {
        crate::claude_mcp::set_has_completed_onboarding().map_err(|e| e.to_string())
    } else {
        crate::claude_mcp::clear_has_completed_onboarding().map_err(|e| e.to_string())
    }
}

fn parse_skill_storage_location(value: &str) -> Result<SkillStorageLocation, String> {
    match value {
        "cc_switch" => Ok(SkillStorageLocation::CcSwitch),
        "unified" => Ok(SkillStorageLocation::Unified),
        _ => Err(format!("Unsupported skill storage location: {value}")),
    }
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("Expected boolean true or false, got: {value}")),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatePayload {
    pub providers: serde_json::Value,
    pub current_provider_id: String,
}

const VALID_TOOLS: [&str; 6] = [
    "claude", "codex", "gemini", "opencode", "openclaw", "hermes",
];

#[derive(Debug, Clone, Serialize)]
pub struct ToolVersion {
    pub name: String,
    pub version: Option<String>,
    pub latest_version: Option<String>,
    pub error: Option<String>,
    pub installed_but_broken: bool,
    pub env_type: String,
    pub wsl_distro: Option<String>,
}

enum ToolLifecycleAction {
    Install,
    Update,
}

impl ToolLifecycleAction {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "install" => Ok(Self::Install),
            "update" => Ok(Self::Update),
            _ => Err(format!("Unsupported tool lifecycle action: {value}")),
        }
    }
}

pub fn tool_versions(tools_json: &str) -> Result<Vec<ToolVersion>, String> {
    let tools = parse_tool_list(tools_json)?;
    Ok(tools.into_iter().map(|tool| tool_version(&tool)).collect())
}

pub fn run_tool_lifecycle_action(tools_json: &str, action: &str) -> Result<(), String> {
    let tools = parse_tool_list(tools_json)?;
    let action = ToolLifecycleAction::parse(action)?;
    if tools.is_empty() {
        return Err("No supported tools selected".to_string());
    }

    let mut lines = vec!["set -e".to_string()];
    for tool in tools {
        let line = tool_lifecycle_command(&tool, &action)
            .ok_or_else(|| format!("Unsupported tool action target: {tool}"))?;
        lines.push(line);
    }

    let output = Command::new("bash")
        .arg("-lc")
        .arg(lines.join("\n"))
        .output()
        .map_err(|e| format!("Failed to start remote tool lifecycle command: {e}"))?;
    finish_command_output(&output)
}

fn parse_tool_list(tools_json: &str) -> Result<Vec<String>, String> {
    let requested: Option<Vec<String>> = if tools_json.trim().is_empty() || tools_json == "-" {
        None
    } else {
        Some(serde_json::from_str(tools_json).map_err(|e| e.to_string())?)
    };
    let requested: Option<HashSet<String>> = requested.map(|tools| tools.into_iter().collect());
    Ok(VALID_TOOLS
        .iter()
        .filter(|tool| {
            requested
                .as_ref()
                .map(|set| set.contains(**tool))
                .unwrap_or(true)
        })
        .map(|tool| (*tool).to_string())
        .collect())
}

fn tool_version(tool: &str) -> ToolVersion {
    let probe = Command::new("bash")
        .arg("-lc")
        .arg(format!("{tool} --version"))
        .output();

    let (version, error, installed_but_broken) = match probe {
        Ok(output) => classify_tool_version_output(&output),
        Err(error) => (None, Some(error.to_string()), false),
    };

    ToolVersion {
        name: tool.to_string(),
        version,
        latest_version: latest_tool_version(tool),
        error,
        installed_but_broken,
        env_type: std::env::consts::OS.to_string(),
        wsl_distro: None,
    }
}

fn classify_tool_version_output(
    output: &std::process::Output,
) -> (Option<String>, Option<String>, bool) {
    if output.status.success() {
        let stdout = decode_command_output(&output.stdout);
        let stderr = decode_command_output(&output.stderr);
        let raw = if stdout.trim().is_empty() {
            stderr.trim()
        } else {
            stdout.trim()
        };
        return (extract_semver(raw), None, false);
    }

    let detail = command_error_detail(output);
    if output.status.code() == Some(127) || detail.trim().is_empty() {
        (
            None,
            Some("not installed or not executable".to_string()),
            false,
        )
    } else {
        (None, Some(detail), true)
    }
}

fn latest_tool_version(tool: &str) -> Option<String> {
    let command = match tool {
        "claude" => "npm view @anthropic-ai/claude-code version --silent",
        "codex" => "npm view @openai/codex version --silent",
        "gemini" => "npm view @google/gemini-cli version --silent",
        "opencode" => "npm view opencode-ai version --silent",
        "openclaw" => "npm view openclaw version --silent",
        "hermes" => {
            "python3 -m pip index versions hermes-agent 2>/dev/null | head -n 1 | sed -n 's/.*(\\([^)]*\\)).*/\\1/p'"
        }
        _ => return None,
    };
    let output = Command::new("bash").arg("-lc").arg(command).output().ok()?;
    if !output.status.success() {
        return None;
    }
    extract_semver(decode_command_output(&output.stdout).trim())
}

fn tool_lifecycle_command(tool: &str, action: &ToolLifecycleAction) -> Option<String> {
    match action {
        ToolLifecycleAction::Install => match tool {
            "claude" => Some(installer_with_npm_fallback(
                "bash -c 'tmp=$(mktemp) && curl -fsSL https://claude.ai/install.sh -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'",
                "npm i -g @anthropic-ai/claude-code@latest",
            )),
            "codex" => Some("npm i -g @openai/codex@latest".to_string()),
            "gemini" => Some("npm i -g @google/gemini-cli@latest".to_string()),
            "opencode" => Some(installer_with_npm_fallback(
                "bash -c 'tmp=$(mktemp) && curl -fsSL https://opencode.ai/install -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'",
                "npm i -g opencode-ai@latest",
            )),
            "openclaw" => Some("npm i -g openclaw@latest".to_string()),
            "hermes" => Some("bash -c 'tmp=$(mktemp) && curl -fsSL https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install.sh -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'".to_string()),
            _ => None,
        },
        ToolLifecycleAction::Update => match tool {
            "claude" => Some("claude update || npm i -g @anthropic-ai/claude-code@latest".to_string()),
            "codex" => Some("codex update || npm i -g @openai/codex@latest".to_string()),
            "gemini" => Some("npm i -g @google/gemini-cli@latest".to_string()),
            "opencode" => Some("opencode upgrade || npm i -g opencode-ai@latest".to_string()),
            "openclaw" => Some("openclaw update --yes || npm i -g openclaw@latest".to_string()),
            "hermes" => Some("hermes update || bash -c 'tmp=$(mktemp) && curl -fsSL https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install.sh -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'".to_string()),
            _ => None,
        },
    }
}

fn installer_with_npm_fallback(installer: &str, npm: &str) -> String {
    format!("{installer} || {npm}")
}

fn extract_semver(raw: &str) -> Option<String> {
    raw.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '+'))
        .find_map(|token| {
            let normalized = token.trim_start_matches(['v', 'V']);
            let core = normalized.split(['-', '+']).next().unwrap_or(normalized);
            let parts: Vec<_> = core.split('.').collect();
            if parts.len() >= 3 && parts.iter().take(3).all(|part| part.parse::<u64>().is_ok()) {
                Some(normalized.to_string())
            } else {
                None
            }
        })
}

fn finish_command_output(output: &std::process::Output) -> Result<(), String> {
    if output.status.success() {
        return Ok(());
    }
    Err(command_error_detail(output))
}

fn command_error_detail(output: &std::process::Output) -> String {
    let stderr = decode_command_output(&output.stderr);
    let stdout = decode_command_output(&output.stdout);
    let raw = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    let detail = last_lines(raw, 8);
    if detail.is_empty() {
        format!("Command failed with status {}", output.status)
    } else {
        detail
    }
}

fn last_lines(text: &str, n: usize) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines.drain(..start);
    lines.join("\n")
}

fn decode_command_output(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

pub fn list_sessions() -> Result<Vec<crate::session_manager::SessionMeta>, String> {
    Ok(crate::session_manager::scan_sessions())
}

pub fn session_messages(
    provider_id: &str,
    source_path: &str,
) -> Result<Vec<crate::session_manager::SessionMessage>, String> {
    crate::session_manager::load_messages(provider_id, source_path)
}

pub fn delete_session(
    provider_id: &str,
    session_id: &str,
    source_path: &str,
) -> Result<bool, String> {
    crate::session_manager::delete_session(provider_id, session_id, source_path)
}

pub fn delete_sessions(
    items_json: &str,
) -> Result<Vec<crate::session_manager::DeleteSessionOutcome>, String> {
    let items: Vec<crate::session_manager::DeleteSessionRequest> =
        serde_json::from_str(items_json).map_err(|e| e.to_string())?;
    Ok(crate::session_manager::delete_sessions(&items))
}

fn parse_hermes_memory_kind(kind: &str) -> Result<crate::hermes_config::MemoryKind, String> {
    serde_json::from_value(json!(kind)).map_err(|e| e.to_string())
}

pub fn get_hermes_memory(kind: &str) -> Result<String, String> {
    let kind = parse_hermes_memory_kind(kind)?;
    crate::hermes_config::read_memory(kind).map_err(|e| e.to_string())
}

pub fn set_hermes_memory(kind: &str, content: &str) -> Result<(), String> {
    let kind = parse_hermes_memory_kind(kind)?;
    crate::hermes_config::write_memory(kind, content).map_err(|e| e.to_string())
}

pub fn get_hermes_memory_limits() -> Result<crate::hermes_config::HermesMemoryLimits, String> {
    crate::hermes_config::read_memory_limits().map_err(|e| e.to_string())
}

pub fn set_hermes_memory_enabled(
    kind: &str,
    enabled: bool,
) -> Result<crate::hermes_config::HermesWriteOutcome, String> {
    let kind = parse_hermes_memory_kind(kind)?;
    crate::hermes_config::set_memory_enabled(kind, enabled).map_err(|e| e.to_string())
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

pub fn provider_state(app: AppType) -> Result<ProviderStatePayload, String> {
    Ok(ProviderStatePayload {
        providers: list_providers(app.clone())?,
        current_provider_id: current_provider(app)?,
    })
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
        _ => import_default_config_internal(&state, app).or_else(live_config_missing_as_false),
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
            && claude_provider_models_are_claude_safe(provider)
        {
            meta.claude_desktop_mode = Some(crate::provider::ClaudeDesktopMode::Direct);
        } else if let Some(routes) = suggested_claude_desktop_routes(provider) {
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

fn import_default_config_internal(state: &AppState, app_type: AppType) -> Result<bool, AppError> {
    let imported = ProviderService::import_default_config(state, app_type.clone())?;

    if imported {
        if state
            .db
            .should_auto_extract_config_snippet(app_type.as_str())?
        {
            match ProviderService::extract_common_config_snippet(state, app_type.clone()) {
                Ok(snippet) if !snippet.is_empty() && snippet != "{}" => {
                    let _ = state
                        .db
                        .set_config_snippet(app_type.as_str(), Some(snippet));
                    let _ = state
                        .db
                        .set_config_snippet_cleared(app_type.as_str(), false);
                }
                _ => {}
            }
        }

        ProviderService::migrate_legacy_common_config_usage_if_needed(state, app_type)?;
    }

    Ok(imported)
}

fn claude_provider_models_are_claude_safe(provider: &Provider) -> bool {
    let Some(env) = provider
        .settings_config
        .get("env")
        .and_then(|value| value.as_object())
    else {
        return true;
    };

    [
        "ANTHROPIC_MODEL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
    ]
    .into_iter()
    .filter_map(|key| env.get(key).and_then(|value| value.as_str()))
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .all(crate::claude_desktop_config::is_claude_safe_model_id)
}

fn suggested_claude_desktop_routes(
    provider: &Provider,
) -> Option<std::collections::HashMap<String, crate::provider::ClaudeDesktopModelRoute>> {
    let env = provider
        .settings_config
        .get("env")
        .and_then(|value| value.as_object())?;
    let mut routes = std::collections::HashMap::new();
    let supports_1m_default = !matches!(
        provider
            .meta
            .as_ref()
            .and_then(|meta| meta.provider_type.as_deref()),
        Some("github_copilot") | Some("codex_oauth")
    );

    fn add_route(
        routes: &mut std::collections::HashMap<String, crate::provider::ClaudeDesktopModelRoute>,
        env: &serde_json::Map<String, serde_json::Value>,
        route_key: &str,
        env_key: &str,
        supports_1m_default: bool,
    ) {
        let Some(raw_model) = env
            .get(env_key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return;
        };

        let marker = crate::claude_desktop_config::ONE_M_CONTEXT_MARKER.as_bytes();
        let raw_bytes = raw_model.as_bytes();
        let has_1m_marker = raw_bytes.len() >= marker.len()
            && raw_bytes[raw_bytes.len() - marker.len()..].eq_ignore_ascii_case(marker);
        let stripped_model = if has_1m_marker {
            raw_model[..raw_model.len() - marker.len()].trim_end()
        } else {
            raw_model
        };
        if stripped_model.is_empty() {
            return;
        }

        let effective_supports_1m = supports_1m_default || has_1m_marker;
        let explicit_label_override = env
            .get(&format!("{env_key}_NAME"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let label_override = explicit_label_override.clone().or_else(|| {
            (!crate::claude_desktop_config::is_claude_safe_model_id(stripped_model))
                .then(|| stripped_model.to_string())
        });

        let should_overwrite = |existing: Option<&str>| {
            existing.is_none()
                || explicit_label_override.is_some()
                || existing == Some(stripped_model)
        };

        let merge_into = |existing: &mut crate::provider::ClaudeDesktopModelRoute| {
            let merged = existing.supports_1m.unwrap_or(false) || effective_supports_1m;
            existing.supports_1m = Some(merged);
            if should_overwrite(existing.label_override.as_deref()) {
                existing.label_override = label_override.clone();
            }
        };

        if let Some(existing) = routes
            .values_mut()
            .find(|existing| existing.model == stripped_model)
        {
            merge_into(existing);
            return;
        }

        routes
            .entry(route_key.to_string())
            .and_modify(merge_into)
            .or_insert_with(|| crate::provider::ClaudeDesktopModelRoute {
                model: stripped_model.to_string(),
                label_override,
                supports_1m: Some(effective_supports_1m),
            });
    }

    for spec in crate::claude_desktop_config::DEFAULT_PROXY_ROUTES {
        add_route(
            &mut routes,
            env,
            spec.route_id,
            spec.env_key,
            supports_1m_default,
        );
    }

    if routes.is_empty() {
        let primary_route = crate::claude_desktop_config::DEFAULT_PROXY_ROUTES[0].route_id;
        add_route(
            &mut routes,
            env,
            primary_route,
            "ANTHROPIC_MODEL",
            supports_1m_default,
        );
    }

    (!routes.is_empty()).then_some(routes)
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
    #[cfg(unix)]
    fn tool_version_command_not_found_is_not_installed_not_broken() {
        use std::os::unix::process::ExitStatusExt;

        let output = std::process::Output {
            status: std::process::ExitStatus::from_raw(127 << 8),
            stdout: Vec::new(),
            stderr: b"bash: line 1: claude: command not found\n".to_vec(),
        };

        let (version, error, installed_but_broken) = classify_tool_version_output(&output);

        assert_eq!(version, None);
        assert_eq!(error.as_deref(), Some("not installed or not executable"));
        assert!(!installed_but_broken);
    }

    #[test]
    #[cfg(unix)]
    fn tool_version_non_127_failure_is_installed_but_broken() {
        use std::os::unix::process::ExitStatusExt;

        let output = std::process::Output {
            status: std::process::ExitStatus::from_raw(1 << 8),
            stdout: Vec::new(),
            stderr: b"node: version too old\n".to_vec(),
        };

        let (version, error, installed_but_broken) = classify_tool_version_output(&output);

        assert_eq!(version, None);
        assert_eq!(error.as_deref(), Some("node: version too old"));
        assert!(installed_but_broken);
    }

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
