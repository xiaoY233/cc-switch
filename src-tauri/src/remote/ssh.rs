use crate::error::AppError;
use crate::remote::types::{
    RemoteAuthMethod, RemoteCommandError, RemoteCommandResponse, RemoteConnectionSecret,
    RemoteHostProfile,
};
use serde::de::DeserializeOwned;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const HELPER_INSTALL_REPO: &str = "https://github.com/xiaoY233/cc-switch";
const HELPER_RELEASE_REPO: &str = "xiaoY233/cc-switch";
const HELPER_INSTALL_REPO_ENV: &str = "CC_SWITCH_REMOTE_HELPER_INSTALL_REPO";
const HELPER_INSTALL_BRANCH_ENV: &str = "CC_SWITCH_REMOTE_HELPER_INSTALL_BRANCH";
const HELPER_RELEASE_REPO_ENV: &str = "CC_SWITCH_REMOTE_HELPER_RELEASE_REPO";
const HELPER_LOCAL_SOURCE_DIR_ENV: &str = "CC_SWITCH_REMOTE_HELPER_LOCAL_SOURCE_DIR";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteHelperInstallSource {
    pub git_repo: String,
    pub git_branch: Option<String>,
    pub release_repo: String,
    pub local_source_dir: Option<PathBuf>,
}

impl Default for RemoteHelperInstallSource {
    fn default() -> Self {
        Self {
            git_repo: HELPER_INSTALL_REPO.to_string(),
            git_branch: None,
            release_repo: HELPER_RELEASE_REPO.to_string(),
            local_source_dir: None,
        }
    }
}

impl RemoteHelperInstallSource {
    pub fn from_env() -> Self {
        let default = Self::default();
        Self {
            git_repo: env_string(HELPER_INSTALL_REPO_ENV).unwrap_or(default.git_repo),
            git_branch: env_string(HELPER_INSTALL_BRANCH_ENV),
            release_repo: env_string(HELPER_RELEASE_REPO_ENV).unwrap_or(default.release_repo),
            local_source_dir: env_string(HELPER_LOCAL_SOURCE_DIR_ENV)
                .map(PathBuf::from)
                .filter(|path| is_valid_local_source_dir(path)),
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
    build_helper_install_args_with_source(profile, &RemoteHelperInstallSource::from_env())
}

pub fn build_helper_install_args_with_source(
    profile: &RemoteHostProfile,
    source: &RemoteHelperInstallSource,
) -> Vec<String> {
    let mut args = build_ssh_base_args(profile);
    let helper_path = shell_quote_helper_path(&profile.helper_path);
    let repo = shell_quote(&source.git_repo);
    let release_repo = shell_quote(&source.release_repo);
    let has_local_source = if source.local_source_dir.is_some() {
        "1"
    } else {
        "0"
    };
    let branch_args = source
        .git_branch
        .as_deref()
        .map(|branch| format!(" --branch {}", shell_quote(branch)))
        .unwrap_or_default();
    let command = format!(
        concat!(
            "set -e; ",
            "helper_path={helper_path}; ",
            "installed_path=\"$HOME/.local/bin/cc-switch-cli\"; ",
            "helper_dir=$(dirname \"$helper_path\"); ",
            "mkdir -p \"$helper_dir\" ~/.local/bin; ",
            "fetch_url_to_stdout() {{ ",
            "if command -v curl >/dev/null 2>&1 && curl -fsSL \"$1\"; then return 0; fi; ",
            "if command -v wget >/dev/null 2>&1 && wget -qO- \"$1\"; then return 0; fi; ",
            "return 1; ",
            "}}; ",
            "fetch_url_to_file() {{ ",
            "if command -v curl >/dev/null 2>&1 && curl -fL \"$1\" -o \"$2\"; then return 0; fi; ",
            "if command -v wget >/dev/null 2>&1 && wget -qO \"$2\" \"$1\"; then return 0; fi; ",
            "return 1; ",
            "}}; ",
            "verify_helper_status() {{ ",
            "status_json=$(\"$helper_path\" --json status); ",
            "printf '%s\\n' \"$status_json\"; ",
            "printf '%s\\n' \"$status_json\" | grep -q '\"providers\"' && ",
            "printf '%s\\n' \"$status_json\" | grep -q '\"openclaw\"' && ",
            "printf '%s\\n' \"$status_json\" | grep -q '\"mcp\"' && ",
            "printf '%s\\n' \"$status_json\" | grep -q '\"prompts\"' && ",
            "printf '%s\\n' \"$status_json\" | grep -q '\"skills\"' && ",
            "printf '%s\\n' \"$status_json\" | grep -q '\"import-export\"' && return 0; ",
            "echo 'cc-switch remote helper is missing required capabilities; install a helper build that includes providers, openclaw, mcp, prompts, skills, and import-export' >&2; ",
            "return 64; ",
            "}}; ",
            "link_installed_helper() {{ ",
            "if [ \"$helper_path\" != \"$installed_path\" ]; then ln -sf \"$installed_path\" \"$helper_path\"; fi; ",
            "}}; ",
            "try_release_asset_install() {{ ",
            "asset_os=$(uname -s); ",
            "case \"$asset_os\" in Linux) asset_os=Linux ;; Darwin) asset_os=macOS ;; *) asset_os= ;; esac; ",
            "asset_arch=$(uname -m); ",
            "case \"$asset_arch\" in x86_64|amd64) asset_arch=x86_64 ;; arm64|aarch64) asset_arch=arm64 ;; *) asset_arch= ;; esac; ",
            "if [ \"$asset_os\" = macOS ]; then asset_arch=universal; fi; ",
            "if [ -z \"$asset_os\" ] || [ -z \"$asset_arch\" ]; then return 1; fi; ",
            "api_url=https://api.github.com/repos/{release_repo}/releases/latest; ",
            "asset_pattern=\"cc-switch-cli-.*-${{asset_os}}-${{asset_arch}}$\"; ",
            "download_url=$(fetch_url_to_stdout \"$api_url\" | grep -E '\"browser_download_url\":' | sed -E 's/.*\"browser_download_url\": \"([^\"]+)\".*/\\1/' | grep -E \"$asset_pattern\" | head -1 || true); ",
            "if [ -z \"$download_url\" ]; then return 1; fi; ",
            "helper_tmp=$(mktemp); ",
            "fetch_url_to_file \"$download_url\" \"$helper_tmp\" 1>&2; ",
            "chmod +x \"$helper_tmp\"; ",
            "mv \"$helper_tmp\" \"$helper_path\"; ",
            "return 0; ",
            "}}; ",
            "finish_cargo_install() {{ ",
            "link_installed_helper; ",
            "verify_helper_status; ",
            "exit 0; ",
            "}}; ",
            "if ! command -v cargo >/dev/null 2>&1; then ",
            "if [ -f \"$HOME/.cargo/env\" ]; then . \"$HOME/.cargo/env\"; fi; ",
            "fi; ",
            "if ! command -v cargo >/dev/null 2>&1; then ",
            "if command -v rustup >/dev/null 2>&1; then ",
            "rustup default stable 1>&2; ",
            "else ",
            "rustup_tmp=$(mktemp); ",
            "fetch_url_to_file https://sh.rustup.rs \"$rustup_tmp\" 1>&2; ",
            "sh \"$rustup_tmp\" -y --profile minimal 1>&2; ",
            "rm -f \"$rustup_tmp\"; ",
            ". \"$HOME/.cargo/env\"; ",
            "fi; ",
            "fi; ",
            "if ! command -v cargo >/dev/null 2>&1; then ",
            "echo 'Rust/Cargo is required to install cc-switch remote helper' >&2; ",
            "exit 127; ",
            "fi; ",
            "if [ {has_local_source} = 1 ]; then ",
            "if command -v tar >/dev/null 2>&1; then ",
            "source_dir=$(mktemp -d); ",
            "if tar -xzf - -C \"$source_dir\" && cargo install --path \"$source_dir/src-tauri\" --bin cc-switch-cli --root ~/.local --locked --force 1>&2; then ",
            "rm -rf \"$source_dir\"; ",
            "finish_cargo_install; ",
            "fi; ",
            "rm -rf \"$source_dir\"; ",
            "echo 'Local source remote helper install failed; falling back to git install' >&2; ",
            "else ",
            "echo 'tar is required for local source remote helper install; falling back to git install' >&2; ",
            "fi; ",
            "fi; ",
            "if cargo install --git {repo}{branch_args} --bin cc-switch-cli --root ~/.local --locked --force 1>&2; then ",
            "finish_cargo_install; ",
            "fi; ",
            "echo 'Git remote helper install failed; falling back to release asset' >&2; ",
            "if try_release_asset_install; then ",
            "verify_helper_status; ",
            "exit 0; ",
            "fi; ",
            "echo 'No compatible cc-switch remote helper release asset found' >&2; ",
            "exit 1"
        ),
        helper_path = helper_path,
        repo = repo,
        release_repo = release_repo,
        branch_args = branch_args,
        has_local_source = has_local_source,
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
    let source = RemoteHelperInstallSource::from_env();
    let input = source
        .local_source_dir
        .as_deref()
        .map(create_local_source_archive)
        .transpose()?;
    let stdout = run_ssh_command_with_input(
        profile,
        build_helper_install_args_with_source(profile, &source),
        input.as_deref(),
        secret,
    )?;
    parse_helper_json(&stdout)
}

fn run_ssh_command(
    profile: &RemoteHostProfile,
    args: Vec<String>,
    secret: Option<&RemoteConnectionSecret>,
) -> Result<String, AppError> {
    run_ssh_command_with_input(profile, args, None, secret)
}

fn run_ssh_command_with_input(
    profile: &RemoteHostProfile,
    args: Vec<String>,
    input: Option<&[u8]>,
    secret: Option<&RemoteConnectionSecret>,
) -> Result<String, AppError> {
    let mut command = Command::new("ssh");
    command.args(args);

    let _askpass = configure_password_auth(profile, secret, &mut command)?;
    let output = if let Some(input) = input {
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|e| AppError::Message(format!("Failed to execute ssh: {e}")))?;
        {
            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| AppError::Message("Failed to open ssh stdin".to_string()))?;
            stdin
                .write_all(input)
                .map_err(|e| AppError::Message(format!("Failed to write ssh stdin: {e}")))?;
        }
        child
            .wait_with_output()
            .map_err(|e| AppError::Message(format!("Failed to wait for ssh: {e}")))?
    } else {
        command
            .output()
            .map_err(|e| AppError::Message(format!("Failed to execute ssh: {e}")))?
    };

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

fn is_valid_local_source_dir(path: &Path) -> bool {
    path.join("src-tauri").join("Cargo.toml").is_file()
        && path.join("src-tauri").join("src").is_dir()
}

fn create_local_source_archive(source_dir: &Path) -> Result<Vec<u8>, AppError> {
    if !is_valid_local_source_dir(source_dir) {
        return Err(AppError::Message(format!(
            "Invalid remote helper local source directory: {}",
            source_dir.display()
        )));
    }

    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut builder = tar::Builder::new(encoder);
    append_source_dir(&mut builder, source_dir, &source_dir.join("src-tauri"))?;
    let encoder = builder
        .into_inner()
        .map_err(|e| AppError::Message(format!("Failed to finish remote helper archive: {e}")))?;
    encoder
        .finish()
        .map_err(|e| AppError::Message(format!("Failed to compress remote helper archive: {e}")))
}

fn append_source_dir<W: Write>(
    builder: &mut tar::Builder<W>,
    root: &Path,
    path: &Path,
) -> Result<(), AppError> {
    for entry in std::fs::read_dir(path).map_err(|e| {
        AppError::Message(format!(
            "Failed to read remote helper source directory {}: {e}",
            path.display()
        ))
    })? {
        let entry = entry.map_err(|e| {
            AppError::Message(format!("Failed to inspect remote helper source entry: {e}"))
        })?;
        let entry_path = entry.path();
        let relative_path = entry_path.strip_prefix(root).map_err(|e| {
            AppError::Message(format!("Failed to build remote helper archive path: {e}"))
        })?;

        if should_skip_local_source_entry(relative_path) {
            continue;
        }

        let metadata = entry.metadata().map_err(|e| {
            AppError::Message(format!(
                "Failed to read remote helper source metadata {}: {e}",
                entry_path.display()
            ))
        })?;

        if metadata.is_dir() {
            builder
                .append_dir(relative_path, &entry_path)
                .map_err(|e| AppError::Message(format!("Failed to archive directory: {e}")))?;
            append_source_dir(builder, root, &entry_path)?;
        } else if metadata.is_file() {
            builder
                .append_path_with_name(&entry_path, relative_path)
                .map_err(|e| AppError::Message(format!("Failed to archive file: {e}")))?;
        }
    }

    Ok(())
}

fn should_skip_local_source_entry(relative_path: &Path) -> bool {
    relative_path
        .components()
        .any(|component| matches!(component.as_os_str().to_str(), Some("target" | ".git")))
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
