use cc_switch_lib::remote::{
    delete_profile_secret, load_profile_secret, load_profiles_from_path, save_profile_secret,
    save_profiles_to_path, RemoteAuthMethod, RemoteConnectionSecret, RemoteHostProfile,
};

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
fn remote_password_secret_round_trips_through_local_database_without_plaintext_profile_storage() {
    with_temp_home(|| {
        let profile = profile("prod", "10.0.0.10");
        let profile_dir = tempfile::tempdir().expect("profile dir");
        save_profiles_to_path(&profile_dir.path().join("remote-hosts.json"), &[profile])
            .expect("profile can still serialize without password");

        save_profile_secret(
            "prod",
            &RemoteConnectionSecret {
                password: Some("stored-password".to_string()),
            },
        )
        .expect("save secret");

        let loaded = load_profile_secret("prod").expect("load secret");
        assert_eq!(loaded.password.as_deref(), Some("stored-password"));

        delete_profile_secret("prod").expect("delete secret");
        let deleted = load_profile_secret("prod").expect("load deleted secret");
        assert_eq!(deleted.password, None);
    });
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
