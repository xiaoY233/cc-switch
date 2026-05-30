use cc_switch_lib::remote::{
    load_profiles_from_path, save_profiles_to_path, RemoteAuthMethod, RemoteHostProfile,
};

fn profile(id: &str, host: &str) -> RemoteHostProfile {
    RemoteHostProfile {
        id: id.to_string(),
        name: format!("{id} server"),
        host: host.to_string(),
        port: 22,
        username: "deploy".to_string(),
        auth_method: RemoteAuthMethod::KeyFile {
            path: "~/.ssh/id_ed25519".to_string(),
        },
        helper_path: "~/.local/bin/cc-switch".to_string(),
        created_at: 1,
        updated_at: 1,
    }
}

#[test]
fn remote_profile_keeps_secrets_out_of_local_state() {
    let profile = profile("prod", "10.0.0.10");

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

#[test]
fn remote_profiles_round_trip_through_store_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("remote-hosts.json");
    let profiles = vec![profile("prod", "10.0.0.10"), profile("dev", "10.0.0.11")];

    save_profiles_to_path(&path, &profiles).expect("save profiles");
    let loaded = load_profiles_from_path(&path).expect("load profiles");

    assert_eq!(loaded, profiles);
}

#[test]
fn missing_remote_profile_store_loads_as_empty_list() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("remote-hosts.json");

    let loaded = load_profiles_from_path(&path).expect("load missing profiles");

    assert!(loaded.is_empty());
}
