use crate::remote::{build_ssh_args, validate_profile, RemoteHostProfile};

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
