use crate::app_config::{InstalledSkill, McpServer, UnmanagedSkill};
use crate::prompt::Prompt;
use crate::provider::Provider;
use crate::remote::{
    build_helper_install_args, build_ssh_args, delete_profile, install_helper_json, load_profiles,
    run_helper_json, upsert_profile, validate_profile, RemoteCapability, RemoteConnectionSecret,
    RemoteHealth, RemoteHostProfile, RemotePlatform,
};
use crate::services::skill::{
    DiscoverableSkill, ImportSkillSelection, SkillBackupEntry, SkillRepo, SkillUninstallResult,
    SkillUpdateInfo,
};
use crate::services::SwitchResult;
use indexmap::IndexMap;

#[tauri::command]
pub fn remote_list_profiles() -> Result<Vec<RemoteHostProfile>, String> {
    load_profiles().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_save_profile(profile: RemoteHostProfile) -> Result<RemoteHostProfile, String> {
    upsert_profile(profile).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_delete_profile(id: String) -> Result<bool, String> {
    delete_profile(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_validate_profile(profile: RemoteHostProfile) -> Result<bool, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn remote_build_status_command(profile: RemoteHostProfile) -> Result<Vec<String>, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    Ok(build_ssh_args(&profile, &["status".to_string()]))
}

#[tauri::command]
pub fn remote_build_helper_install_command(
    profile: RemoteHostProfile,
) -> Result<Vec<String>, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    Ok(build_helper_install_args(&profile))
}

#[tauri::command]
pub fn remote_parse_helper_response(raw: String) -> Result<serde_json::Value, String> {
    serde_json::from_str(&raw).map_err(|e| format!("Invalid helper JSON: {e}"))
}

#[tauri::command]
pub fn remote_check_health(
    profile: RemoteHostProfile,
    secret: Option<RemoteConnectionSecret>,
) -> Result<RemoteHealth, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    let status: serde_json::Value =
        run_helper_json(&profile, &["status".to_string()], secret.as_ref())
            .map_err(|e| e.to_string())?;

    Ok(remote_health_from_status(status))
}

#[tauri::command]
pub fn remote_install_helper(
    profile: RemoteHostProfile,
    secret: Option<RemoteConnectionSecret>,
) -> Result<RemoteHealth, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    let status: serde_json::Value =
        install_helper_json(&profile, secret.as_ref()).map_err(|e| e.to_string())?;

    Ok(remote_health_from_status(status))
}

fn remote_health_from_status(status: serde_json::Value) -> RemoteHealth {
    RemoteHealth {
        reachable: true,
        helper_installed: true,
        helper_version: status
            .get("version")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        platform: status
            .get("platform")
            .and_then(|value| value.as_str())
            .map(parse_remote_platform),
        capabilities: status
            .get("capabilities")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str())
                    .filter_map(parse_remote_capability)
                    .collect()
            })
            .unwrap_or_default(),
        last_error: None,
    }
}

fn parse_remote_platform(value: &str) -> RemotePlatform {
    match value {
        "linux" => RemotePlatform::Linux,
        "macos" => RemotePlatform::Macos,
        _ => RemotePlatform::Unknown,
    }
}

fn parse_remote_capability(value: &str) -> Option<RemoteCapability> {
    match value {
        "providers" => Some(RemoteCapability::Providers),
        "openclaw" => Some(RemoteCapability::Openclaw),
        "mcp" => Some(RemoteCapability::Mcp),
        "prompts" => Some(RemoteCapability::Prompts),
        "skills" => Some(RemoteCapability::Skills),
        "import-export" => Some(RemoteCapability::ImportExport),
        _ => None,
    }
}

#[tauri::command]
pub fn remote_get_providers(
    profile: RemoteHostProfile,
    app: String,
    secret: Option<RemoteConnectionSecret>,
) -> Result<IndexMap<String, Provider>, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["providers".to_string(), "list".to_string(), app],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_get_current_provider(
    profile: RemoteHostProfile,
    app: String,
    secret: Option<RemoteConnectionSecret>,
) -> Result<String, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["providers".to_string(), "current".to_string(), app],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_switch_provider(
    profile: RemoteHostProfile,
    app: String,
    id: String,
    secret: Option<RemoteConnectionSecret>,
) -> Result<SwitchResult, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["providers".to_string(), "switch".to_string(), app, id],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_add_provider(
    profile: RemoteHostProfile,
    app: String,
    provider: Provider,
    #[allow(non_snake_case)] addToLive: Option<bool>,
    secret: Option<RemoteConnectionSecret>,
) -> Result<bool, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    let provider_json = serde_json::to_string(&provider).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &[
            "providers".to_string(),
            "add".to_string(),
            app,
            provider_json,
            addToLive.unwrap_or(true).to_string(),
        ],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_update_provider(
    profile: RemoteHostProfile,
    app: String,
    provider: Provider,
    #[allow(non_snake_case)] originalId: Option<String>,
    secret: Option<RemoteConnectionSecret>,
) -> Result<bool, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    let provider_json = serde_json::to_string(&provider).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &[
            "providers".to_string(),
            "update".to_string(),
            app,
            provider_json,
            originalId.unwrap_or_else(|| "-".to_string()),
        ],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_delete_provider(
    profile: RemoteHostProfile,
    app: String,
    id: String,
    secret: Option<RemoteConnectionSecret>,
) -> Result<bool, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["providers".to_string(), "delete".to_string(), app, id],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_set_openclaw_default_model(
    profile: RemoteHostProfile,
    model: crate::openclaw_config::OpenClawDefaultModel,
    secret: Option<RemoteConnectionSecret>,
) -> Result<crate::openclaw_config::OpenClawWriteOutcome, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    let model_json = serde_json::to_string(&model).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &[
            "openclaw".to_string(),
            "set-default-model".to_string(),
            model_json,
        ],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_get_mcp_servers(
    profile: RemoteHostProfile,
    secret: Option<RemoteConnectionSecret>,
) -> Result<IndexMap<String, McpServer>, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["mcp".to_string(), "list".to_string()],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_upsert_mcp_server(
    profile: RemoteHostProfile,
    server: McpServer,
    secret: Option<RemoteConnectionSecret>,
) -> Result<(), String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    let server_json = serde_json::to_string(&server).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["mcp".to_string(), "upsert".to_string(), server_json],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_delete_mcp_server(
    profile: RemoteHostProfile,
    id: String,
    secret: Option<RemoteConnectionSecret>,
) -> Result<bool, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["mcp".to_string(), "delete".to_string(), id],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_toggle_mcp_app(
    profile: RemoteHostProfile,
    #[allow(non_snake_case)] serverId: String,
    app: String,
    enabled: bool,
    secret: Option<RemoteConnectionSecret>,
) -> Result<(), String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &[
            "mcp".to_string(),
            "toggle".to_string(),
            serverId,
            app,
            enabled.to_string(),
        ],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_import_mcp_from_apps(
    profile: RemoteHostProfile,
    secret: Option<RemoteConnectionSecret>,
) -> Result<usize, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["mcp".to_string(), "import".to_string()],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_get_prompts(
    profile: RemoteHostProfile,
    app: String,
    secret: Option<RemoteConnectionSecret>,
) -> Result<IndexMap<String, Prompt>, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["prompts".to_string(), "list".to_string(), app],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_upsert_prompt(
    profile: RemoteHostProfile,
    app: String,
    id: String,
    prompt: Prompt,
    secret: Option<RemoteConnectionSecret>,
) -> Result<(), String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    let prompt_json = serde_json::to_string(&prompt).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &[
            "prompts".to_string(),
            "upsert".to_string(),
            app,
            id,
            prompt_json,
        ],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_delete_prompt(
    profile: RemoteHostProfile,
    app: String,
    id: String,
    secret: Option<RemoteConnectionSecret>,
) -> Result<(), String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["prompts".to_string(), "delete".to_string(), app, id],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_enable_prompt(
    profile: RemoteHostProfile,
    app: String,
    id: String,
    secret: Option<RemoteConnectionSecret>,
) -> Result<(), String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["prompts".to_string(), "enable".to_string(), app, id],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_import_prompt_from_file(
    profile: RemoteHostProfile,
    app: String,
    secret: Option<RemoteConnectionSecret>,
) -> Result<String, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["prompts".to_string(), "import".to_string(), app],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_get_current_prompt_file_content(
    profile: RemoteHostProfile,
    app: String,
    secret: Option<RemoteConnectionSecret>,
) -> Result<Option<String>, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["prompts".to_string(), "current".to_string(), app],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_get_installed_skills(
    profile: RemoteHostProfile,
    secret: Option<RemoteConnectionSecret>,
) -> Result<Vec<InstalledSkill>, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["skills".to_string(), "installed".to_string()],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_get_skill_backups(
    profile: RemoteHostProfile,
    secret: Option<RemoteConnectionSecret>,
) -> Result<Vec<SkillBackupEntry>, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["skills".to_string(), "backups".to_string()],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_delete_skill_backup(
    profile: RemoteHostProfile,
    #[allow(non_snake_case)] backupId: String,
    secret: Option<RemoteConnectionSecret>,
) -> Result<bool, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["skills".to_string(), "delete-backup".to_string(), backupId],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_install_skill_unified(
    profile: RemoteHostProfile,
    skill: DiscoverableSkill,
    #[allow(non_snake_case)] currentApp: String,
    secret: Option<RemoteConnectionSecret>,
) -> Result<InstalledSkill, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    let skill_json = serde_json::to_string(&skill).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &[
            "skills".to_string(),
            "install".to_string(),
            skill_json,
            currentApp,
        ],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_uninstall_skill_unified(
    profile: RemoteHostProfile,
    id: String,
    secret: Option<RemoteConnectionSecret>,
) -> Result<SkillUninstallResult, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["skills".to_string(), "uninstall".to_string(), id],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_restore_skill_backup(
    profile: RemoteHostProfile,
    #[allow(non_snake_case)] backupId: String,
    #[allow(non_snake_case)] currentApp: String,
    secret: Option<RemoteConnectionSecret>,
) -> Result<InstalledSkill, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &[
            "skills".to_string(),
            "restore".to_string(),
            backupId,
            currentApp,
        ],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_toggle_skill_app(
    profile: RemoteHostProfile,
    id: String,
    app: String,
    enabled: bool,
    secret: Option<RemoteConnectionSecret>,
) -> Result<bool, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &[
            "skills".to_string(),
            "toggle".to_string(),
            id,
            app,
            enabled.to_string(),
        ],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_scan_unmanaged_skills(
    profile: RemoteHostProfile,
    secret: Option<RemoteConnectionSecret>,
) -> Result<Vec<UnmanagedSkill>, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["skills".to_string(), "scan-unmanaged".to_string()],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_import_skills_from_apps(
    profile: RemoteHostProfile,
    imports: Vec<ImportSkillSelection>,
    secret: Option<RemoteConnectionSecret>,
) -> Result<Vec<InstalledSkill>, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    let imports_json = serde_json::to_string(&imports).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["skills".to_string(), "import".to_string(), imports_json],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_discover_available_skills(
    profile: RemoteHostProfile,
    secret: Option<RemoteConnectionSecret>,
) -> Result<Vec<DiscoverableSkill>, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["skills".to_string(), "discover".to_string()],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_check_skill_updates(
    profile: RemoteHostProfile,
    secret: Option<RemoteConnectionSecret>,
) -> Result<Vec<SkillUpdateInfo>, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["skills".to_string(), "check-updates".to_string()],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_update_skill(
    profile: RemoteHostProfile,
    id: String,
    secret: Option<RemoteConnectionSecret>,
) -> Result<InstalledSkill, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["skills".to_string(), "update".to_string(), id],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_get_skill_repos(
    profile: RemoteHostProfile,
    secret: Option<RemoteConnectionSecret>,
) -> Result<Vec<SkillRepo>, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["skills".to_string(), "repos".to_string()],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_add_skill_repo(
    profile: RemoteHostProfile,
    repo: SkillRepo,
    secret: Option<RemoteConnectionSecret>,
) -> Result<bool, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    let repo_json = serde_json::to_string(&repo).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["skills".to_string(), "add-repo".to_string(), repo_json],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_remove_skill_repo(
    profile: RemoteHostProfile,
    owner: String,
    name: String,
    secret: Option<RemoteConnectionSecret>,
) -> Result<bool, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    run_helper_json(
        &profile,
        &["skills".to_string(), "remove-repo".to_string(), owner, name],
        secret.as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::{RemoteAuthMethod, RemoteHostProfile};

    fn valid_profile() -> RemoteHostProfile {
        RemoteHostProfile {
            id: "prod".to_string(),
            name: "Production".to_string(),
            host: "example.com".to_string(),
            port: 22,
            username: "ccswitch".to_string(),
            auth_method: RemoteAuthMethod::SshAgent,
            helper_path: "/usr/local/bin/cc-switch-helper".to_string(),
            created_at: 1,
            updated_at: 1,
        }
    }

    #[test]
    fn validates_remote_profile() {
        assert!(remote_validate_profile(valid_profile()).unwrap());
    }

    #[test]
    fn builds_status_command_after_validation() {
        let args = remote_build_status_command(valid_profile()).unwrap();

        assert_eq!(
            args,
            vec![
                "-p",
                "22",
                "-o",
                "ConnectTimeout=10",
                "-o",
                "BatchMode=yes",
                "--",
                "ccswitch@example.com",
                "/usr/local/bin/cc-switch-helper --json status",
            ]
        );
    }

    #[test]
    fn builds_helper_install_command_after_validation() {
        let args = remote_build_helper_install_command(valid_profile()).unwrap();
        let command = args.last().expect("remote command");

        assert!(command.contains("rustup.rs"));
        assert!(command.contains("cargo install --git"));
        assert!(command.contains("\"$helper_path\" --json status"));
    }

    #[test]
    fn rejects_invalid_helper_json_with_context() {
        let err = remote_parse_helper_response("{".to_string()).unwrap_err();

        assert!(err.starts_with("Invalid helper JSON: "));
    }
}
