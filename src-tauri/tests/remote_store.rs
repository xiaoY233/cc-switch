use cc_switch_lib::remote::{RemoteAuthMethod, RemoteHostProfile};

#[test]
fn remote_profile_keeps_secrets_out_of_local_state() {
    let profile = RemoteHostProfile {
        id: "prod".to_string(),
        name: "Production".to_string(),
        host: "10.0.0.10".to_string(),
        port: 22,
        username: "deploy".to_string(),
        auth_method: RemoteAuthMethod::KeyFile {
            path: "~/.ssh/id_ed25519".to_string(),
        },
        helper_path: "~/.local/bin/cc-switch".to_string(),
        created_at: 1,
        updated_at: 1,
    };

    let serialized = serde_json::to_string(&profile).expect("serialize profile");
    assert!(!serialized.contains("api_key"));
    assert!(!serialized.contains("providerSecret"));
    assert!(serialized.contains("id_ed25519"));
}
