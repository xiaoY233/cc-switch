#[test]
fn status_returns_stable_json_envelope() {
    let response = cc_switch_lib::cli::run(&["status".to_string()]);

    assert_eq!(response["ok"], true);
    assert_eq!(response["data"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(response["data"]["platform"], std::env::consts::OS);
    assert_eq!(
        response["data"]["capabilities"],
        serde_json::json!(["providers", "mcp", "prompts", "skills", "import-export"])
    );
    assert!(response["error"].is_null());
}

#[test]
fn unsupported_command_returns_stable_error_envelope() {
    let response = cc_switch_lib::cli::run(&["unknown".to_string()]);

    assert_eq!(response["ok"], false);
    assert!(response["data"].is_null());
    assert_eq!(response["error"]["code"], "unsupported_command");
    assert_eq!(response["error"]["message"], "Supported command: status");
}
