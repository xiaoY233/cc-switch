use cc_switch_lib::remote::{
    build_helper_install_args, build_helper_install_args_with_source, build_ssh_args,
    run_helper_json, RemoteAuthMethod, RemoteConnectionSecret, RemoteHelperInstallSource,
    RemoteHostProfile,
};
#[cfg(unix)]
use serial_test::serial;
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

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
        helper_path: "~/.local/bin/cc-switch-remote-helper".to_string(),
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
fn ssh_args_accept_new_host_keys_without_disabling_changed_host_protection() {
    let args = build_ssh_args(&profile(), &["status".to_string()]);

    assert!(args.contains(&"StrictHostKeyChecking=accept-new".to_string()));
    assert!(args.contains(&"NumberOfPasswordPrompts=1".to_string()));
    assert!(!args.contains(&"StrictHostKeyChecking=no".to_string()));
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
#[cfg(unix)]
#[serial]
fn password_auth_runs_ssh_with_askpass_secret_without_command_line_secret() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ssh_path = dir.path().join("ssh");
    fs::write(
        &ssh_path,
        r#"#!/bin/sh
set -eu
test "${SSH_ASKPASS_REQUIRE:-}" = "force"
test "${DISPLAY:-}" = "cc-switch"
test "$("$SSH_ASKPASS")" = "unit-test-secret"
test "${CC_SWITCH_REMOTE_SSH_PASSWORD:-}" = "unit-test-secret"
printf '%s\n' '{"ok":true,"data":{"version":"test","platform":"linux","capabilities":["providers"]},"error":null}'
"#,
    )
    .expect("write fake ssh");
    let mut permissions = fs::metadata(&ssh_path)
        .expect("fake ssh metadata")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&ssh_path, permissions).expect("chmod fake ssh");

    let old_path = std::env::var_os("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", dir.path().display(), old_path.to_string_lossy());
    std::env::set_var("PATH", new_path);

    let mut profile = profile();
    profile.auth_method = RemoteAuthMethod::Password;
    let secret = RemoteConnectionSecret {
        password: Some("unit-test-secret".to_string()),
    };

    let status: serde_json::Value =
        run_helper_json(&profile, &["status".to_string()], Some(&secret)).expect("helper status");

    std::env::set_var("PATH", old_path);

    assert_eq!(status["version"], "test");
}

#[test]
#[cfg(unix)]
#[serial]
fn helper_json_accepts_ok_null_data_for_void_commands() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ssh_path = dir.path().join("ssh");
    fs::write(
        &ssh_path,
        r#"#!/bin/sh
set -eu
printf '%s\n' '{"ok":true,"data":null,"error":null}'
"#,
    )
    .expect("write fake ssh");
    let mut permissions = fs::metadata(&ssh_path)
        .expect("fake ssh metadata")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&ssh_path, permissions).expect("chmod fake ssh");

    let old_path = std::env::var_os("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", dir.path().display(), old_path.to_string_lossy());
    std::env::set_var("PATH", new_path);

    let result =
        run_helper_json::<()>(&profile(), &["mcp".to_string(), "upsert".to_string()], None);

    std::env::set_var("PATH", old_path);

    assert!(result.is_ok());
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
        "~/.local/bin/cc-switch-remote-helper --json '' 'two words' status"
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
        "~/.local/bin/cc-switch-remote-helper --json 'it'\\''s; $(rm -rf /) && ok'"
    );
}

#[test]
fn ssh_command_has_no_trailing_space_when_helper_args_are_empty() {
    let args = build_ssh_args(&profile(), &[]);

    assert_eq!(
        args.last().expect("remote command"),
        "~/.local/bin/cc-switch-remote-helper --json"
    );
}

#[test]
fn helper_install_args_install_cli_and_link_configured_helper_path() {
    let args = build_helper_install_args(&profile());
    let remote_command = args.last().expect("remote command");

    assert!(args.contains(&"alice@example.com".to_string()));
    assert!(remote_command.contains("api.github.com/repos/xiaoY233/cc-switch/releases/latest"));
    assert!(remote_command.contains("fetch_url_to_stdout()"));
    assert!(remote_command.contains("wget -qO- \"$1\""));
    assert!(remote_command.contains("fetch_url_to_file()"));
    assert!(remote_command.contains("wget -qO \"$2\" \"$1\""));
    assert!(remote_command.contains("verify_helper_status()"));
    assert!(remote_command.contains("grep -q '\"openclaw\"'"));
    assert!(remote_command.contains("cc-switch remote helper is missing required capabilities"));
    assert!(remote_command.contains("cc-switch-cli-.*-${asset_os}-${asset_arch}"));
    assert!(remote_command.contains("fetch_url_to_file \"$download_url\" \"$helper_tmp\""));
    assert!(remote_command.contains("rustup.rs"));
    assert!(remote_command.contains(". \"$HOME/.cargo/env\""));
    assert!(remote_command.contains("Rust/Cargo is required to install cc-switch remote helper"));
    assert!(remote_command.contains("cargo install --git https://github.com/xiaoY233/cc-switch"));
    assert!(remote_command.contains("--bin cc-switch-cli"));
    assert!(remote_command.contains("--force"));
    assert!(remote_command.contains("installed_path=\"$HOME/.local/bin/cc-switch-cli\""));
    assert!(remote_command.contains("ln -sf \"$installed_path\" \"$helper_path\""));
    assert!(remote_command.contains("verify_helper_status"));
}

#[test]
fn helper_install_defaults_to_git_build_before_release_asset_fallback() {
    let source = RemoteHelperInstallSource::default();
    assert_eq!(source.local_source_dir, None);

    let args = build_helper_install_args_with_source(&profile(), &source);
    let remote_command = args.last().expect("remote command");
    let git_index = remote_command
        .find("if cargo install --git https://github.com/xiaoY233/cc-switch")
        .expect("git install command");
    let release_index = remote_command
        .find("if try_release_asset_install; then")
        .expect("release fallback");

    assert!(git_index < release_index);
    assert!(
        remote_command.contains("Git remote helper install failed; falling back to release asset")
    );
}

#[test]
fn helper_install_args_quote_configured_helper_path() {
    let mut profile = profile();
    profile.helper_path = "/tmp/cc switch'; rm -rf /".to_string();

    let args = build_helper_install_args(&profile);

    assert!(args
        .last()
        .expect("remote command")
        .contains("helper_path='/tmp/cc switch'\\''; rm -rf /'"));
}

#[test]
fn helper_install_args_accept_custom_source_branch() {
    let source = RemoteHelperInstallSource {
        git_repo: "https://github.com/acme/cc-switch".to_string(),
        git_branch: Some("remote-helper".to_string()),
        release_repo: "acme/cc-switch".to_string(),
        local_source_dir: None,
    };

    let args = build_helper_install_args_with_source(&profile(), &source);
    let remote_command = args.last().expect("remote command");

    assert!(remote_command.contains("api.github.com/repos/acme/cc-switch/releases/latest"));
    assert!(remote_command.contains(
        "cargo install --git https://github.com/acme/cc-switch --branch remote-helper --bin cc-switch-cli"
    ));
}

#[test]
fn helper_install_args_accept_local_source_dir_before_git_fallback() {
    let source = RemoteHelperInstallSource {
        git_repo: "https://github.com/acme/cc-switch".to_string(),
        git_branch: Some("remote-helper".to_string()),
        release_repo: "acme/cc-switch".to_string(),
        local_source_dir: Some("/Users/alice/cc-switch".into()),
    };

    let args = build_helper_install_args_with_source(&profile(), &source);
    let remote_command = args.last().expect("remote command");

    assert!(remote_command.contains("source_dir=$(mktemp -d)"));
    assert!(remote_command.contains("tar -xzf - -C \"$source_dir\""));
    assert!(remote_command.contains("cargo install --path \"$source_dir/src-tauri\" --bin cc-switch-cli --root ~/.local --locked --force"));
    assert!(remote_command.contains("rm -rf \"$source_dir\""));
    assert!(remote_command.contains("cargo install --git https://github.com/acme/cc-switch --branch remote-helper --bin cc-switch-cli"));
}

#[test]
#[cfg(unix)]
#[serial]
fn install_helper_streams_local_source_archive_to_ssh_stdin() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source_dir = dir.path().join("source");
    fs::create_dir_all(source_dir.join("src-tauri/src")).expect("source dirs");
    fs::write(
        source_dir.join("src-tauri/Cargo.toml"),
        "[package]\nname='unit'\n",
    )
    .expect("write cargo toml");
    fs::write(source_dir.join("src-tauri/src/lib.rs"), "").expect("write lib");

    let stdin_path = dir.path().join("stdin.tgz");
    let ssh_path = dir.path().join("ssh");
    fs::write(
        &ssh_path,
        format!(
            r#"#!/bin/sh
set -eu
cat > "{}"
test -s "{}"
printf '%s\n' '{{"ok":true,"data":{{"version":"test","platform":"linux","capabilities":["providers","openclaw","mcp","prompts","skills","import-export"]}},"error":null}}'
"#,
            stdin_path.display(),
            stdin_path.display()
        ),
    )
    .expect("write fake ssh");
    let mut permissions = fs::metadata(&ssh_path)
        .expect("fake ssh metadata")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&ssh_path, permissions).expect("chmod fake ssh");

    let old_path = std::env::var_os("PATH").unwrap_or_default();
    let old_source = std::env::var_os("CC_SWITCH_REMOTE_HELPER_LOCAL_SOURCE_DIR");
    let new_path = format!("{}:{}", dir.path().display(), old_path.to_string_lossy());
    std::env::set_var("PATH", new_path);
    std::env::set_var("CC_SWITCH_REMOTE_HELPER_LOCAL_SOURCE_DIR", &source_dir);

    let status: serde_json::Value =
        cc_switch_lib::remote::install_helper_json(&profile(), None).expect("install helper");

    std::env::set_var("PATH", old_path);
    match old_source {
        Some(value) => std::env::set_var("CC_SWITCH_REMOTE_HELPER_LOCAL_SOURCE_DIR", value),
        None => std::env::remove_var("CC_SWITCH_REMOTE_HELPER_LOCAL_SOURCE_DIR"),
    }

    assert_eq!(status["version"], "test");
    assert!(fs::metadata(stdin_path).expect("stdin archive").len() > 0);
}
