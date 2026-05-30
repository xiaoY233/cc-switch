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

    let serialized = serde_json::to_value(&profile).expect("serialize profile");
    let object = serialized
        .as_object()
        .expect("remote profile serializes to an object");

    let mut keys = object.keys().map(String::as_str).collect::<Vec<_>>();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "authMethod",
            "createdAt",
            "helperPath",
            "host",
            "id",
            "name",
            "port",
            "updatedAt",
            "username",
        ],
        "local remote profiles should only store connection metadata"
    );

    assert_eq!(
        object.get("authMethod"),
        Some(&serde_json::json!({
            "type": "keyFile",
            "path": "~/.ssh/id_ed25519",
        }))
    );

    for secret_key in [
        "apiKey",
        "api_key",
        "providerSecret",
        "providerSecrets",
        "providerApiKey",
    ] {
        assert_eq!(object.get(secret_key), None);
    }
}
