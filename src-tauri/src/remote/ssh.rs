use crate::error::AppError;
use crate::remote::types::{
    RemoteAuthMethod, RemoteCommandError, RemoteCommandResponse, RemoteConnectionSecret,
    RemoteHostProfile,
};
use serde::de::DeserializeOwned;
use std::process::Command;

const HELPER_INSTALL_REPO: &str = "https://github.com/xiaoY233/cc-switch";
const HELPER_RELEASE_REPO: &str = "xiaoY233/cc-switch";

fn build_ssh_base_args(profile: &RemoteHostProfile) -> Vec<String> {
    let mut args = vec![
        "-p".to_string(),
        profile.port.to_string(),
        "-o".to_string(),
        "ConnectTimeout=10".to_string(),
        "-o".to_string(),
        match &profile.auth_method {
            RemoteAuthMethod::Password => "BatchMode=no".to_string(),
            _ => "BatchMode=yes".to_string(),
        },
    ];

    match &profile.auth_method {
        RemoteAuthMethod::KeyFile { path } => {
            args.push("-i".to_string());
            args.push(path.clone());
        }
        RemoteAuthMethod::Password => {
            args.push("-o".to_string());
            args.push("PreferredAuthentications=password,keyboard-interactive".to_string());
            args.push("-o".to_string());
            args.push("PubkeyAuthentication=no".to_string());
        }
        RemoteAuthMethod::SshAgent => {}
    }

    args.push("--".to_string());
    args.push(format!("{}@{}", profile.username, profile.host));
    args
}

pub fn build_ssh_args(profile: &RemoteHostProfile, helper_args: &[String]) -> Vec<String> {
    let mut args = build_ssh_base_args(profile);

    let mut command = vec![
        shell_quote_helper_path(&profile.helper_path),
        "--json".to_string(),
    ];
    command.extend(helper_args.iter().map(|arg| shell_quote(arg)));
    args.push(command.join(" "));
    args
}

pub fn build_helper_install_args(profile: &RemoteHostProfile) -> Vec<String> {
    let mut args = build_ssh_base_args(profile);
    let helper_path = shell_quote_helper_path(&profile.helper_path);
    let repo = shell_quote(HELPER_INSTALL_REPO);
    let release_repo = shell_quote(HELPER_RELEASE_REPO);
    let command = format!(
        concat!(
            "set -e; ",
            "helper_path={helper_path}; ",
            "installed_path=\"$HOME/.local/bin/cc-switch-cli\"; ",
            "helper_dir=$(dirname \"$helper_path\"); ",
            "mkdir -p \"$helper_dir\" ~/.local/bin; ",
            "asset_os=$(uname -s); ",
            "case \"$asset_os\" in Linux) asset_os=Linux ;; Darwin) asset_os=macOS ;; *) asset_os= ;; esac; ",
            "asset_arch=$(uname -m); ",
            "case \"$asset_arch\" in x86_64|amd64) asset_arch=x86_64 ;; arm64|aarch64) asset_arch=arm64 ;; *) asset_arch= ;; esac; ",
            "if [ \"$asset_os\" = macOS ]; then asset_arch=universal; fi; ",
            "if [ -n \"$asset_os\" ] && [ -n \"$asset_arch\" ] && command -v curl >/dev/null 2>&1; then ",
            "api_url=https://api.github.com/repos/{release_repo}/releases/latest; ",
            "asset_pattern=\"cc-switch-cli-.*-${{asset_os}}-${{asset_arch}}$\"; ",
            "download_url=$(curl -fsSL \"$api_url\" | grep -E '\"browser_download_url\":' | sed -E 's/.*\"browser_download_url\": \"([^\"]+)\".*/\\1/' | grep -E \"$asset_pattern\" | head -1 || true); ",
            "if [ -n \"$download_url\" ]; then ",
            "helper_tmp=$(mktemp); ",
            "curl -fL \"$download_url\" -o \"$helper_tmp\" 1>&2; ",
            "chmod +x \"$helper_tmp\"; ",
            "mv \"$helper_tmp\" \"$helper_path\"; ",
            "\"$helper_path\" --json status; ",
            "exit 0; ",
            "fi; ",
            "fi; ",
            "if ! command -v cargo >/dev/null 2>&1; then ",
            "if [ -f \"$HOME/.cargo/env\" ]; then . \"$HOME/.cargo/env\"; fi; ",
            "fi; ",
            "if ! command -v cargo >/dev/null 2>&1; then ",
            "if command -v rustup >/dev/null 2>&1; then ",
            "rustup default stable 1>&2; ",
            "elif command -v curl >/dev/null 2>&1; then ",
            "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal 1>&2; ",
            ". \"$HOME/.cargo/env\"; ",
            "elif command -v wget >/dev/null 2>&1; then ",
            "wget -qO- https://sh.rustup.rs | sh -s -- -y --profile minimal 1>&2; ",
            ". \"$HOME/.cargo/env\"; ",
            "fi; ",
            "fi; ",
            "if ! command -v cargo >/dev/null 2>&1; then ",
            "echo 'Rust/Cargo is required to install cc-switch remote helper' >&2; ",
            "exit 127; ",
            "fi; ",
            "cargo install --git {repo} --bin cc-switch-cli --root ~/.local --locked 1>&2; ",
            "if [ \"$helper_path\" != \"$installed_path\" ]; then ",
            "ln -sf \"$installed_path\" \"$helper_path\"; ",
            "fi; ",
            "\"$helper_path\" --json status"
        ),
        helper_path = helper_path,
        repo = repo,
        release_repo = release_repo,
    );
    args.push(command);
    args
}

pub fn run_helper_json<T: DeserializeOwned>(
    profile: &RemoteHostProfile,
    helper_args: &[String],
    secret: Option<&RemoteConnectionSecret>,
) -> Result<T, AppError> {
    let stdout = run_ssh_command(profile, build_ssh_args(profile, helper_args), secret)?;
    parse_helper_json(&stdout)
}

pub fn install_helper_json<T: DeserializeOwned>(
    profile: &RemoteHostProfile,
    secret: Option<&RemoteConnectionSecret>,
) -> Result<T, AppError> {
    let stdout = run_ssh_command(profile, build_helper_install_args(profile), secret)?;
    parse_helper_json(&stdout)
}

fn run_ssh_command(
    profile: &RemoteHostProfile,
    args: Vec<String>,
    secret: Option<&RemoteConnectionSecret>,
) -> Result<String, AppError> {
    let mut command = Command::new("ssh");
    command.args(args);

    let _askpass = configure_password_auth(profile, secret, &mut command)?;
    let output = command
        .output()
        .map_err(|e| AppError::Message(format!("Failed to execute ssh: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AppError::Message(if stderr.is_empty() {
            format!("Remote ssh command failed with status {}", output.status)
        } else {
            stderr
        }));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| AppError::Message(format!("Remote helper returned invalid UTF-8: {e}")))
}

fn parse_helper_json<T: DeserializeOwned>(stdout: &str) -> Result<T, AppError> {
    let json_line = stdout
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(stdout)
        .trim();
    let envelope: RemoteCommandResponse<T> = serde_json::from_str(json_line)
        .map_err(|e| AppError::Message(format!("Remote helper returned invalid JSON: {e}")))?;

    if envelope.ok {
        envelope
            .data
            .ok_or_else(|| AppError::Message("Remote helper returned ok without data".to_string()))
    } else {
        let RemoteCommandError { code, message } = envelope.error.unwrap_or(RemoteCommandError {
            code: "remote_error".to_string(),
            message: "Remote helper command failed".to_string(),
        });
        Err(AppError::Message(format!("{code}: {message}")))
    }
}

#[cfg(unix)]
fn configure_password_auth(
    profile: &RemoteHostProfile,
    secret: Option<&RemoteConnectionSecret>,
    command: &mut Command,
) -> Result<Option<tempfile::NamedTempFile>, AppError> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    if !matches!(profile.auth_method, RemoteAuthMethod::Password) {
        return Ok(None);
    }

    let password = secret
        .and_then(|secret| secret.password.as_deref())
        .filter(|password| !password.is_empty())
        .ok_or_else(|| AppError::Message("Remote SSH password is required".to_string()))?;

    let mut askpass = tempfile::Builder::new()
        .prefix("cc-switch-ssh-askpass-")
        .tempfile()
        .map_err(|e| AppError::Message(format!("Failed to create ssh askpass helper: {e}")))?;
    askpass
        .write_all(b"#!/bin/sh\nprintf '%s' \"$CC_SWITCH_REMOTE_SSH_PASSWORD\"\n")
        .map_err(|e| AppError::Message(format!("Failed to write ssh askpass helper: {e}")))?;
    let mut perms = askpass
        .as_file()
        .metadata()
        .map_err(|e| AppError::Message(format!("Failed to inspect ssh askpass helper: {e}")))?
        .permissions();
    perms.set_mode(0o700);
    askpass
        .as_file()
        .set_permissions(perms)
        .map_err(|e| AppError::Message(format!("Failed to secure ssh askpass helper: {e}")))?;

    command.env("SSH_ASKPASS", askpass.path());
    command.env("SSH_ASKPASS_REQUIRE", "force");
    command.env("DISPLAY", "cc-switch");
    command.env("CC_SWITCH_REMOTE_SSH_PASSWORD", password);
    Ok(Some(askpass))
}

#[cfg(not(unix))]
fn configure_password_auth(
    profile: &RemoteHostProfile,
    _secret: Option<&RemoteConnectionSecret>,
    _command: &mut Command,
) -> Result<Option<()>, AppError> {
    if matches!(profile.auth_method, RemoteAuthMethod::Password) {
        return Err(AppError::Message(
            "Remote SSH password auth is only supported on Unix desktops".to_string(),
        ));
    }
    Ok(None)
}

fn shell_quote_helper_path(value: &str) -> String {
    if is_safe_unquoted_helper_path(value) {
        return value.to_string();
    }
    shell_quote(value)
}

fn is_safe_unquoted_helper_path(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:~".contains(c))
}

fn shell_quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:".contains(c))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}
