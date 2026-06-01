use crate::error::AppError;
use crate::remote::types::{
    RemoteAuthMethod, RemoteCommandError, RemoteCommandResponse, RemoteConnectionSecret,
    RemoteHostProfile,
};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::process::Command;

const HELPER_RELEASE_REPO: &str = "xiaoY233/cc-switch";
const HELPER_RELEASE_TAG: &str = "remote-helper-latest";
const HELPER_RELEASE_REPO_ENV: &str = "CC_SWITCH_REMOTE_HELPER_RELEASE_REPO";
const HELPER_RELEASE_TAG_ENV: &str = "CC_SWITCH_REMOTE_HELPER_RELEASE_TAG";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteHelperInstallSource {
    pub release_repo: String,
    pub release_tag: String,
}

impl Default for RemoteHelperInstallSource {
    fn default() -> Self {
        Self {
            release_repo: HELPER_RELEASE_REPO.to_string(),
            release_tag: HELPER_RELEASE_TAG.to_string(),
        }
    }
}

impl RemoteHelperInstallSource {
    pub fn from_env() -> Self {
        let default = Self::default();
        Self {
            release_repo: env_string(HELPER_RELEASE_REPO_ENV).unwrap_or(default.release_repo),
            release_tag: env_string(HELPER_RELEASE_TAG_ENV).unwrap_or(default.release_tag),
        }
    }
}

fn build_ssh_base_args(profile: &RemoteHostProfile) -> Vec<String> {
    let mut args = vec![
        "-p".to_string(),
        profile.port.to_string(),
        "-o".to_string(),
        "ConnectTimeout=10".to_string(),
        "-o".to_string(),
        "StrictHostKeyChecking=accept-new".to_string(),
        "-o".to_string(),
        "NumberOfPasswordPrompts=1".to_string(),
        "-o".to_string(),
        "ControlMaster=auto".to_string(),
        "-o".to_string(),
        "ControlPersist=10m".to_string(),
        "-S".to_string(),
        control_socket_path(profile),
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

fn control_socket_path(profile: &RemoteHostProfile) -> String {
    let mut hasher = Sha256::new();
    hasher.update(profile.id.as_bytes());
    hasher.update(b"\0");
    hasher.update(profile.username.as_bytes());
    hasher.update(b"\0");
    hasher.update(profile.host.as_bytes());
    hasher.update(b"\0");
    hasher.update(profile.port.to_string().as_bytes());
    let digest = hasher.finalize();
    let short = digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    std::path::Path::new("/tmp")
        .join(format!("ccsw-{short}"))
        .to_string_lossy()
        .into_owned()
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
    build_helper_install_args_with_source(profile, &RemoteHelperInstallSource::from_env())
}

pub fn build_helper_install_args_with_source(
    profile: &RemoteHostProfile,
    source: &RemoteHelperInstallSource,
) -> Vec<String> {
    let mut args = build_ssh_base_args(profile);
    let helper_path = shell_quote_helper_path(&profile.helper_path);
    let release_repo = shell_quote(&source.release_repo);
    let release_tag = shell_quote(&source.release_tag);
    let command = format!(
        concat!(
            "set -e; ",
            "helper_path={helper_path}; ",
            "helper_dir=$(dirname \"$helper_path\"); ",
            "mkdir -p \"$helper_dir\" ~/.local/bin; ",
            "fetch_url_to_stdout() {{ ",
            "if command -v curl >/dev/null 2>&1 && curl -fsSL \"$1\"; then return 0; fi; ",
            "if command -v wget >/dev/null 2>&1 && wget -qO- \"$1\"; then return 0; fi; ",
            "return 1; ",
            "}}; ",
            "fetch_url_to_file() {{ ",
            "if command -v curl >/dev/null 2>&1 && curl -fsSL \"$1\" -o \"$2\"; then return 0; fi; ",
            "if command -v wget >/dev/null 2>&1 && wget -qO \"$2\" \"$1\"; then return 0; fi; ",
            "return 1; ",
            "}}; ",
            "verify_helper_status() {{ ",
            "status_output=$(\"$helper_path\" --json status 2>&1) || {{ ",
            "case \"$status_output\" in ",
            "*libgdk-3.so.0*|*libgtk-3.so.0*|*libwebkit2gtk*|*libayatana-appindicator*) ",
            "echo 'Downloaded remote helper is not compatible with this server: it depends on desktop GTK/WebKit libraries. Reinstall after the latest helper release is published.' >&2 ;; ",
            "*) echo \"Remote helper downloaded but failed to start: $status_output\" >&2 ;; ",
            "esac; ",
            "return 65; ",
            "}}; ",
            "status_json=$status_output; ",
            "printf '%s\\n' \"$status_json\"; ",
            "printf '%s\\n' \"$status_json\" | grep -q '\"providers\"' && ",
            "printf '%s\\n' \"$status_json\" | grep -q '\"openclaw\"' && ",
            "printf '%s\\n' \"$status_json\" | grep -q '\"mcp\"' && ",
            "printf '%s\\n' \"$status_json\" | grep -q '\"prompts\"' && ",
            "printf '%s\\n' \"$status_json\" | grep -q '\"skills\"' && ",
            "printf '%s\\n' \"$status_json\" | grep -q '\"sessions\"' && ",
            "printf '%s\\n' \"$status_json\" | grep -q '\"hermes-memory\"' && ",
            "printf '%s\\n' \"$status_json\" | grep -q '\"import-export\"' && return 0; ",
            "echo 'cc-switch remote helper is missing required capabilities; install a helper build that includes providers, openclaw, mcp, prompts, skills, sessions, hermes-memory, and import-export' >&2; ",
            "return 64; ",
            "}}; ",
            "try_release_asset_install() {{ ",
            "asset_os=$(uname -s); ",
            "case \"$asset_os\" in Linux) asset_os=Linux ;; Darwin) asset_os=macOS ;; *) asset_os= ;; esac; ",
            "asset_arch=$(uname -m); ",
            "case \"$asset_arch\" in x86_64|amd64) asset_arch=x86_64 ;; arm64|aarch64) asset_arch=arm64 ;; *) asset_arch= ;; esac; ",
            "if [ -z \"$asset_os\" ] || [ -z \"$asset_arch\" ]; then return 1; fi; ",
            "api_url=https://api.github.com/repos/{release_repo}/releases/tags/{release_tag}; ",
            "asset_pattern=\"cc-switch-cli-.*-${{asset_os}}-${{asset_arch}}$\"; ",
            "download_url=$(fetch_url_to_stdout \"$api_url\" | grep -E '\"browser_download_url\":' | sed -E 's/.*\"browser_download_url\": \"([^\"]+)\".*/\\1/' | grep -E \"$asset_pattern\" | tail -1 || true); ",
            "if [ -z \"$download_url\" ]; then return 1; fi; ",
            "helper_tmp=$(mktemp); ",
            "fetch_url_to_file \"$download_url\" \"$helper_tmp\" 1>&2; ",
            "chmod +x \"$helper_tmp\"; ",
            "mv \"$helper_tmp\" \"$helper_path\"; ",
            "return 0; ",
            "}}; ",
            "if try_release_asset_install; then ",
            "verify_helper_status; ",
            "exit 0; ",
            "fi; ",
            "echo 'No compatible cc-switch remote helper release asset found on GitHub release {release_tag}' >&2; ",
            "exit 1"
        ),
        helper_path = helper_path,
        release_repo = release_repo,
        release_tag = release_tag,
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
    let stdout = run_ssh_command(
        profile,
        build_helper_install_args_with_source(profile, &RemoteHelperInstallSource::from_env()),
        secret,
    )?;
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
            normalize_remote_stderr(&stderr)
        }));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| AppError::Message(format!("Remote helper returned invalid UTF-8: {e}")))
}

fn normalize_remote_stderr(stderr: &str) -> String {
    if stderr.contains("libgdk-3.so.0")
        || stderr.contains("libgtk-3.so.0")
        || stderr.contains("libwebkit2gtk")
        || stderr.contains("libayatana-appindicator")
    {
        "远程 Helper 不是纯 CLI 构建，依赖服务器上不存在的桌面 GTK/WebKit 库。请重新安装最新的远程 Helper。".to_string()
    } else {
        stderr.to_string()
    }
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
        if let Some(data) = envelope.data {
            Ok(data)
        } else {
            serde_json::from_value(serde_json::Value::Null).map_err(|_| {
                AppError::Message("Remote helper returned ok without data".to_string())
            })
        }
    } else {
        let RemoteCommandError { code, message } = envelope.error.unwrap_or(RemoteCommandError {
            code: "remote_error".to_string(),
            message: "Remote helper command failed".to_string(),
        });
        Err(AppError::Message(format!("{code}: {message}")))
    }
}

fn env_string(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(unix)]
fn configure_password_auth(
    profile: &RemoteHostProfile,
    secret: Option<&RemoteConnectionSecret>,
    command: &mut Command,
) -> Result<Option<tempfile::TempPath>, AppError> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    if !matches!(profile.auth_method, RemoteAuthMethod::Password) {
        return Ok(None);
    }

    let stored_secret = if secret
        .and_then(|secret| secret.password.as_deref())
        .filter(|password| !password.is_empty())
        .is_none()
    {
        crate::remote::store::load_profile_secret(&profile.id).ok()
    } else {
        None
    };
    let password = secret
        .and_then(|secret| secret.password.as_deref())
        .or_else(|| {
            stored_secret
                .as_ref()
                .and_then(|secret| secret.password.as_deref())
        })
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

    let askpass_path = askpass.into_temp_path();
    command.env("SSH_ASKPASS", &askpass_path);
    command.env("SSH_ASKPASS_REQUIRE", "force");
    command.env("DISPLAY", "cc-switch");
    command.env("CC_SWITCH_REMOTE_SSH_PASSWORD", password);
    Ok(Some(askpass_path))
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
