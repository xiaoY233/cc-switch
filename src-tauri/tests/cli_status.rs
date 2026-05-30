use serial_test::serial;

#[test]
fn status_returns_stable_json_envelope() {
    let response = cc_switch_lib::cli::run(&["status".to_string()]);

    assert_eq!(response["ok"], true);
    assert_eq!(response["data"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(response["data"]["platform"], std::env::consts::OS);
    assert_eq!(
        response["data"]["capabilities"],
        serde_json::json!([
            "providers",
            "openclaw",
            "mcp",
            "prompts",
            "skills",
            "import-export"
        ])
    );
    assert!(response["error"].is_null());
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
    let temp = tempfile::tempdir().expect("temp dir");
    let old_test_home = std::env::var_os("CC_SWITCH_TEST_HOME");
    let old_home = std::env::var_os("HOME");
    std::env::set_var("CC_SWITCH_TEST_HOME", temp.path());
    std::env::set_var("HOME", temp.path());

    let model_json = r#"{"primary":"provider-1/gpt-4.1","fallbacks":["provider-1/gpt-4.1-mini"]}"#;
    let set_response = cc_switch_lib::cli::run(&[
        "openclaw".to_string(),
        "set-default-model".to_string(),
        model_json.to_string(),
    ]);
    let get_response =
        cc_switch_lib::cli::run(&["openclaw".to_string(), "get-default-model".to_string()]);

    match old_test_home {
        Some(value) => std::env::set_var("CC_SWITCH_TEST_HOME", value),
        None => std::env::remove_var("CC_SWITCH_TEST_HOME"),
    }
    match old_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

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
    let temp = tempfile::tempdir().expect("temp dir");
    let old_test_home = std::env::var_os("CC_SWITCH_TEST_HOME");
    let old_home = std::env::var_os("HOME");
    std::env::set_var("CC_SWITCH_TEST_HOME", temp.path());
    std::env::set_var("HOME", temp.path());

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

    match old_test_home {
        Some(value) => std::env::set_var("CC_SWITCH_TEST_HOME", value),
        None => std::env::remove_var("CC_SWITCH_TEST_HOME"),
    }
    match old_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }

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
fn unsupported_command_returns_stable_error_envelope() {
    let response = cc_switch_lib::cli::run(&["unknown".to_string()]);

    assert_eq!(response["ok"], false);
    assert!(response["data"].is_null());
    assert_eq!(response["error"]["code"], "unsupported_command");
    assert_eq!(
        response["error"]["message"],
        "Supported commands: status, providers, openclaw, mcp, prompts, skills"
    );
}
