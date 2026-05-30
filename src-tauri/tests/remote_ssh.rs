use cc_switch_lib::remote::{build_ssh_args, RemoteAuthMethod, RemoteHostProfile};

fn profile() -> RemoteHostProfile {
    RemoteHostProfile {
        id: "dev".to_string(),
        name: "Dev".to_string(),
        host: "example.com".to_string(),
        port: 2222,
        username: "alice".to_string(),
        auth_method: RemoteAuthMethod::KeyFile {
            path: "/Users/alice/.ssh/id_ed25519".to_string(),
        },
        helper_path: "~/.local/bin/cc-switch".to_string(),
        created_at: 1,
        updated_at: 1,
    }
}

#[test]
fn ssh_args_include_port_identity_and_json_command() {
    let args = build_ssh_args(&profile(), &["status".to_string()]);
    assert_eq!(args[0], "-p");
    assert!(args.contains(&"2222".to_string()));
    assert!(args.contains(&"-i".to_string()));
    assert!(args.contains(&"/Users/alice/.ssh/id_ed25519".to_string()));
    assert!(args.contains(&"alice@example.com".to_string()));
    assert!(args.last().expect("remote command").contains("--json"));
}

#[test]
fn ssh_command_preserves_empty_and_space_helper_args() {
    let args = build_ssh_args(
        &profile(),
        &[
            "".to_string(),
            "two words".to_string(),
            "status".to_string(),
        ],
    );

    assert_eq!(
        args.last().expect("remote command"),
        "~/.local/bin/cc-switch --json '' 'two words' status"
    );
}

#[test]
fn ssh_command_quotes_helper_path_with_spaces() {
    let mut profile = profile();
    profile.helper_path = "/opt/cc switch/bin/cc-switch".to_string();

    let args = build_ssh_args(&profile, &["status".to_string()]);

    assert_eq!(
        args.last().expect("remote command"),
        "'/opt/cc switch/bin/cc-switch' --json status"
    );
}

#[test]
fn ssh_command_escapes_single_quote_and_metacharacters_in_helper_args() {
    let args = build_ssh_args(&profile(), &["it's; $(rm -rf /) && ok".to_string()]);

    assert_eq!(
        args.last().expect("remote command"),
        "~/.local/bin/cc-switch --json 'it'\\''s; $(rm -rf /) && ok'"
    );
}

#[test]
fn ssh_command_has_no_trailing_space_when_helper_args_are_empty() {
    let args = build_ssh_args(&profile(), &[]);

    assert_eq!(
        args.last().expect("remote command"),
        "~/.local/bin/cc-switch --json"
    );
}
