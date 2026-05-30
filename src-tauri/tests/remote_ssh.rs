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
