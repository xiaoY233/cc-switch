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
fn ssh_args_terminate_options_before_destination() {
    let mut profile = profile();
    profile.username = "-oProxyCommand=bad".to_string();

    let args = build_ssh_args(&profile, &["status".to_string()]);
    let destination_index = args
        .iter()
        .position(|arg| arg == "-oProxyCommand=bad@example.com")
        .expect("destination");

    assert_eq!(args[destination_index - 1], "--");
}

#[test]
fn ssh_password_auth_uses_interactive_options_without_exposing_password() {
    let mut profile = profile();
    profile.auth_method = RemoteAuthMethod::Password;

    let args = build_ssh_args(&profile, &["status".to_string()]);
    let joined = args.join(" ");

    assert!(args.contains(&"BatchMode=no".to_string()));
    assert!(args.contains(&"PreferredAuthentications=password,keyboard-interactive".to_string()));
    assert!(args.contains(&"PubkeyAuthentication=no".to_string()));
    assert!(!joined.contains("password="));
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
fn ssh_command_escapes_single_quote_and_metacharacters_in_helper_path() {
    let mut profile = profile();
    profile.helper_path = "/tmp/cc switch'; rm -rf /".to_string();

    let args = build_ssh_args(&profile, &["status".to_string()]);

    assert_eq!(
        args.last().expect("remote command"),
        "'/tmp/cc switch'\\''; rm -rf /' --json status"
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
