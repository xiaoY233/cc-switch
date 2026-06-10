#![cfg(feature = "desktop")]

use serial_test::serial;

fn with_temp_home<T>(run: impl FnOnce() -> T) -> T {
    let temp = tempfile::tempdir().expect("temp dir");
    let old_test_home = std::env::var_os("CC_SWITCH_TEST_HOME");
    let old_home = std::env::var_os("HOME");
    std::env::set_var("CC_SWITCH_TEST_HOME", temp.path());
    std::env::set_var("HOME", temp.path());

    let result = run();

    match old_test_home {
        Some(value) => std::env::set_var("CC_SWITCH_TEST_HOME", value),
        None => std::env::remove_var("CC_SWITCH_TEST_HOME"),
    }
    match old_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

    result
}

#[test]
fn status_returns_stable_json_envelope() {
    let response = cc_switch_lib::cli::run(&["status".to_string()]);

    assert_eq!(response["ok"], true);
    assert_eq!(response["data"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(response["data"]["platform"], std::env::consts::OS);
    assert_eq!(
        response["data"]["capabilities"],
        serde_json::json!(cc_switch_lib::remote_capabilities::REMOTE_HELPER_CAPABILITIES)
    );
    assert!(response["error"].is_null());
}

#[test]
fn status_advertises_session_capability() {
    let response = cc_switch_lib::cli::run(&["status".to_string()]);
    let capabilities = response
        .get("data")
        .and_then(|data| data.get("capabilities"))
        .and_then(|value| value.as_array())
        .expect("status capabilities");

    assert!(
        capabilities
            .iter()
            .any(|value| value.as_str() == Some("session")),
        "remote helper status must advertise persistent session support"
    );
}

#[test]
#[serial]
fn settings_round_trip_through_json_cli() {
    let (save_response, get_response) = with_temp_home(|| {
        let settings_json = serde_json::json!({
            "showInTray": true,
            "minimizeToTrayOnClose": true,
            "useAppWindowControls": false,
            "enableClaudePluginIntegration": true,
            "skipClaudeOnboarding": true,
            "launchOnStartup": false,
            "silentStartup": false,
            "enableLocalProxy": false,
            "visibleApps": {
                "claude": true,
                "claude-desktop": false,
                "codex": true,
                "gemini": false,
                "opencode": true,
                "openclaw": true,
                "hermes": true
            },
            "skillSyncMethod": "auto",
            "skillStorageLocation": "unified"
        })
        .to_string();

        let save_response =
            cc_switch_lib::cli::run(&["settings".to_string(), "save".to_string(), settings_json]);
        let get_response = cc_switch_lib::cli::run(&["settings".to_string(), "get".to_string()]);
        (save_response, get_response)
    });

    assert_eq!(save_response["ok"], true);
    assert!(save_response["error"].is_null());
    assert_eq!(get_response["ok"], true);
    assert_eq!(get_response["data"]["visibleApps"]["claude-desktop"], false);
    assert_eq!(get_response["data"]["visibleApps"]["hermes"], true);
    assert_eq!(get_response["data"]["skillStorageLocation"], "unified");
    assert_eq!(get_response["data"]["enableClaudePluginIntegration"], true);
    assert_eq!(get_response["data"]["skipClaudeOnboarding"], true);
}

#[test]
fn status_accepts_json_flag_for_remote_invocation() {
    let response = cc_switch_lib::cli::run(&["--json".to_string(), "status".to_string()]);

    assert_eq!(response["ok"], true);
    assert_eq!(response["data"]["version"], env!("CARGO_PKG_VERSION"));
    assert!(response["error"].is_null());
}

#[test]
#[serial]
fn openclaw_default_model_round_trips_through_json_cli() {
    let (set_response, get_response) = with_temp_home(|| {
        let model_json =
            r#"{"primary":"provider-1/gpt-4.1","fallbacks":["provider-1/gpt-4.1-mini"]}"#;
        let set_response = cc_switch_lib::cli::run(&[
            "openclaw".to_string(),
            "set-default-model".to_string(),
            model_json.to_string(),
        ]);
        let get_response =
            cc_switch_lib::cli::run(&["openclaw".to_string(), "get-default-model".to_string()]);
        (set_response, get_response)
    });

    assert_eq!(set_response["ok"], true);
    assert!(set_response["error"].is_null());
    assert_eq!(get_response["ok"], true);
    assert_eq!(get_response["data"]["primary"], "provider-1/gpt-4.1");
    assert_eq!(
        get_response["data"]["fallbacks"],
        serde_json::json!(["provider-1/gpt-4.1-mini"])
    );
}

#[test]
#[serial]
fn openclaw_sections_round_trip_through_json_cli() {
    let (set_env, get_env, set_tools, get_tools, set_agents, get_agents) = with_temp_home(|| {
        let env_json = r#"{"vars":{"NODE_ENV":"remote"}}"#;
        let tools_json = r#"{"profile":"coding","allow":["Bash(*)"]}"#;
        let agents_json = r#"{"workspace":"~/remote","timeoutSeconds":300}"#;

        let set_env = cc_switch_lib::cli::run(&[
            "openclaw".to_string(),
            "set-env".to_string(),
            env_json.to_string(),
        ]);
        let get_env = cc_switch_lib::cli::run(&["openclaw".to_string(), "get-env".to_string()]);
        let set_tools = cc_switch_lib::cli::run(&[
            "openclaw".to_string(),
            "set-tools".to_string(),
            tools_json.to_string(),
        ]);
        let get_tools = cc_switch_lib::cli::run(&["openclaw".to_string(), "get-tools".to_string()]);
        let set_agents = cc_switch_lib::cli::run(&[
            "openclaw".to_string(),
            "set-agents-defaults".to_string(),
            agents_json.to_string(),
        ]);
        let get_agents =
            cc_switch_lib::cli::run(&["openclaw".to_string(), "get-agents-defaults".to_string()]);
        (
            set_env, get_env, set_tools, get_tools, set_agents, get_agents,
        )
    });

    assert_eq!(set_env["ok"], true);
    assert_eq!(get_env["ok"], true);
    assert_eq!(get_env["data"]["vars"]["NODE_ENV"], "remote");
    assert_eq!(set_tools["ok"], true);
    assert_eq!(get_tools["ok"], true);
    assert_eq!(get_tools["data"]["profile"], "coding");
    assert_eq!(get_tools["data"]["allow"], serde_json::json!(["Bash(*)"]));
    assert_eq!(set_agents["ok"], true);
    assert_eq!(get_agents["ok"], true);
    assert_eq!(get_agents["data"]["workspace"], "~/remote");
    assert_eq!(get_agents["data"]["timeoutSeconds"], 300);
}

#[test]
#[serial]
fn hermes_memory_round_trips_through_json_cli() {
    let (set_response, get_response, limits_response) = with_temp_home(|| {
        let set_response = cc_switch_lib::cli::run(&[
            "hermes".to_string(),
            "memory".to_string(),
            "set".to_string(),
            "memory".to_string(),
            "remote helper memory".to_string(),
        ]);
        let get_response = cc_switch_lib::cli::run(&[
            "hermes".to_string(),
            "memory".to_string(),
            "get".to_string(),
            "memory".to_string(),
        ]);
        let limits_response = cc_switch_lib::cli::run(&[
            "hermes".to_string(),
            "memory".to_string(),
            "limits".to_string(),
        ]);
        (set_response, get_response, limits_response)
    });

    assert_eq!(set_response["ok"], true);
    assert!(set_response["error"].is_null());
    assert_eq!(get_response["ok"], true);
    assert_eq!(get_response["data"], "remote helper memory");
    assert_eq!(limits_response["ok"], true);
    assert_eq!(limits_response["data"]["memory"], 2200);
}

#[test]
#[serial]
fn providers_import_returns_stable_json_envelope() {
    let response = with_temp_home(|| {
        cc_switch_lib::cli::run(&[
            "providers".to_string(),
            "import".to_string(),
            "claude".to_string(),
        ])
    });

    assert_eq!(response["ok"], true);
    assert!(response["error"].is_null());
    assert_eq!(response["data"], false);
}

#[test]
#[serial]
fn providers_sort_returns_stable_json_envelope() {
    let response = with_temp_home(|| {
        cc_switch_lib::cli::run(&[
            "providers".to_string(),
            "sort".to_string(),
            "claude".to_string(),
            r#"[{"id":"provider-a","sortIndex":0}]"#.to_string(),
        ])
    });

    assert_eq!(response["ok"], true);
    assert!(response["error"].is_null());
    assert_eq!(response["data"], true);
}

#[test]
#[serial]
fn providers_update_preserves_existing_secret_when_payload_contains_redacted_sentinel() {
    let (list_response, update_response, stored_token) = with_temp_home(|| {
        let add_response = cc_switch_lib::cli::run(&[
            "providers".to_string(),
            "add".to_string(),
            "claude".to_string(),
            r#"{"id":"remote-provider","name":"Remote Provider","settingsConfig":{"env":{"ANTHROPIC_AUTH_TOKEN":"sk-original","ANTHROPIC_BASE_URL":"https://api.anthropic.com"}}}"#
                .to_string(),
            "false".to_string(),
        ]);
        assert_eq!(add_response["ok"], true);

        let list_response = cc_switch_lib::cli::run(&[
            "providers".to_string(),
            "list".to_string(),
            "claude".to_string(),
        ]);
        assert_eq!(list_response["ok"], true, "{list_response}");
        let mut provider = list_response["data"]["remote-provider"].clone();
        assert_eq!(provider["id"], "remote-provider", "{list_response}");
        assert_eq!(
            provider["settingsConfig"]["env"]["ANTHROPIC_AUTH_TOKEN"],
            "[redacted]"
        );

        provider["name"] = serde_json::Value::String("Remote Secret Renamed".to_string());
        let update_response = cc_switch_lib::cli::run(&[
            "providers".to_string(),
            "update".to_string(),
            "claude".to_string(),
            serde_json::to_string(&provider).expect("provider json"),
            "-".to_string(),
        ]);

        let stored = cc_switch_lib::Database::init()
            .expect("database")
            .get_provider_by_id("remote-provider", "claude")
            .expect("load provider")
            .expect("stored provider");
        let stored_token = stored.settings_config["env"]["ANTHROPIC_AUTH_TOKEN"]
            .as_str()
            .expect("stored token")
            .to_string();

        (list_response, update_response, stored_token)
    });

    assert_eq!(list_response["ok"], true);
    assert_eq!(update_response["ok"], true);
    assert_eq!(stored_token, "sk-original");
}

#[test]
#[serial]
fn import_export_round_trips_database_sql_through_json_cli() {
    let (export_response, import_response) = with_temp_home(|| {
        let seed_response = cc_switch_lib::cli::run(&[
            "providers".to_string(),
            "add".to_string(),
            "claude".to_string(),
            r#"{"id":"remote-seed","name":"Remote Seed","settingsConfig":{"env":{"ANTHROPIC_AUTH_TOKEN":"test-key"}}}"#
                .to_string(),
            "false".to_string(),
        ]);
        assert_eq!(seed_response["ok"], true);
        let export_response =
            cc_switch_lib::cli::run(&["import-export".to_string(), "export-sql".to_string()]);
        let sql = export_response["data"]
            .as_str()
            .expect("exported SQL string");
        let encoded = {
            use base64::{engine::general_purpose::STANDARD, Engine as _};
            STANDARD.encode(sql)
        };
        let import_response = cc_switch_lib::cli::run(&[
            "import-export".to_string(),
            "import-sql-b64".to_string(),
            encoded,
        ]);
        (export_response, import_response)
    });

    assert_eq!(export_response["ok"], true);
    assert!(export_response["error"].is_null());
    assert!(export_response["data"]
        .as_str()
        .expect("exported SQL string")
        .starts_with("-- CC Switch SQLite 导出"));
    assert_eq!(import_response["ok"], true);
    assert!(import_response["error"].is_null());
    assert!(import_response["data"]["success"]
        .as_bool()
        .unwrap_or(false));
    assert!(import_response["data"]["backupId"].is_string());
}

#[test]
fn unsupported_command_returns_stable_error_envelope() {
    let response = cc_switch_lib::cli::run(&["unknown".to_string()]);

    assert_eq!(response["ok"], false);
    assert!(response["data"].is_null());
    assert_eq!(response["error"]["code"], "unsupported_command");
    assert_eq!(
        response["error"]["message"],
        "Supported commands: status, providers, universal-providers, routing-config, sessions, hermes, openclaw, mcp, prompts, skills, import-export, tools, settings, plugin"
    );
}
