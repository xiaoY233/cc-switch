use crate::provider::Provider;
use crate::remote::{
    build_ssh_args, delete_profile, load_profiles, run_helper_json, upsert_profile,
    validate_profile, RemoteCapability, RemoteConnectionSecret, RemoteHealth, RemoteHostProfile,
    RemotePlatform,
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

    Ok(RemoteHealth {
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
    })
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
    fn rejects_invalid_helper_json_with_context() {
        let err = remote_parse_helper_response("{".to_string()).unwrap_err();

        assert!(err.starts_with("Invalid helper JSON: "));
    }
}
