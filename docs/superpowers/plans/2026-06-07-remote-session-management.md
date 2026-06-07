# Remote Session Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace normal remote UI operations that launch one SSH process per action with a persistent SSH-backed helper session.

**Architecture:** Add a newline-delimited JSON-RPC `serve` mode to the remote helper, then add a Tauri `RemoteSessionManager` that owns one SSH child process per remote host and routes existing remote helper command vectors through it. Keep existing one-shot SSH commands for helper install, diagnostics, and compatibility, while keeping the frontend `remoteApi` shape stable.

**Tech Stack:** Rust/Tauri 2, serde JSON envelopes, stdio over SSH, Tokio process/io/time/sync, React 18, TanStack Query, existing remote helper command dispatcher.

---

## File Structure

- Modify `src-tauri/Cargo.toml`: add Tokio `process` and `io-util` features for async SSH child process handling.
- Modify `src-tauri/src/cli/types.rs`: add serializable/deserializable serve request/response types.
- Modify `src-tauri/src/cli/commands.rs`: advertise the `session` capability in helper status.
- Modify `src-tauri/src/cli/mod.rs`: expose a shared command dispatcher and route `serve`.
- Create `src-tauri/src/cli/serve.rs`: helper stdin/stdout serve loop and testable single-line handler.
- Modify `src-tauri/src/bin/cc-switch-cli.rs`: avoid printing an extra envelope after `serve` exits.
- Modify `src-tauri/src/remote/types.rs`: add remote session status and helper-session capability enum value.
- Modify `src-tauri/src/remote/mod.rs`: export the session module and new types.
- Create `src-tauri/src/remote/session.rs`: Tauri-side SSH session transport and manager.
- Modify `src-tauri/src/remote/ssh.rs`: expose one-shot execution for fallback, add serve SSH args, keep install path one-shot.
- Modify `src-tauri/src/commands/remote.rs`: route normal remote helper commands through the session manager and add session status/close commands.
- Modify `src-tauri/src/lib.rs`: manage `RemoteSessionManager` as Tauri state.
- Modify `src/lib/api/remote.ts`: add transport status types and `getSessionStatus`/`closeSession`.
- Modify `src/lib/query/remote.ts`: add host-scoped transport status query keys.
- Modify `src/components/remote/RemoteHealthPanel.tsx`: show helper upgrade requirement when `session` is missing.
- Modify `src/App.tsx`: read active remote session status once and pass it to remote-capable panels.
- Modify `src/components/sessions/SessionManagerPage.tsx`, `src/components/skills/SkillsPage.tsx`, `src/components/mcp/UnifiedMcpPanel.tsx`, `src/components/prompts/PromptPanel.tsx`, and existing tool/settings panels that already receive `target={managementTarget}`: show panel-local loading and stale data without changing their local behavior.
- Add Rust tests under `src-tauri/tests/remote_session.rs` and update existing remote CLI/SSH tests.
- Add frontend tests under `tests/components` and `src/lib/query`.

## Execution Notes

- Start from a clean branch or commit the existing unrelated remote settings and SSH fixes before implementing this plan.
- Keep commits small and scoped by task.
- Do not remove the one-shot SSH path until session mode is fully verified.
- Do not modify local provider, MCP, prompt, skill, settings, or session business logic to support transport changes.

## Task 1: Helper Serve Protocol Types and Status Capability

**Files:**
- Modify: `src-tauri/src/cli/types.rs`
- Modify: `src-tauri/src/cli/commands.rs`
- Test: `src-tauri/tests/cli_status.rs`

- [ ] **Step 1: Write failing status capability test**

Add this test to `src-tauri/tests/cli_status.rs`:

```rust
use cc_switch_lib::cli;

#[test]
fn status_advertises_session_capability() {
    let response = cli::run(&["status".to_string()]);
    let capabilities = response
        .get("data")
        .and_then(|data| data.get("capabilities"))
        .and_then(|value| value.as_array())
        .expect("status capabilities");

    assert!(
        capabilities
            .iter()
            .any(|value| value.as_str() == Some("session")),
        "remote helper status must advertise persistent session support"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test cli_status status_advertises_session_capability
```

Expected: FAIL because `session` is not in the helper capability list.

- [ ] **Step 3: Add serve protocol types**

Modify `src-tauri/src/cli/types.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliEnvelope<T: Serialize> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<CliError>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CliServeRequest {
    pub id: String,
    pub command: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CliServeResponse {
    pub id: String,
    pub ok: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<CliError>,
}

pub fn ok<T: Serialize>(data: T) -> CliEnvelope<T> {
    CliEnvelope {
        ok: true,
        data: Some(data),
        error: None,
    }
}

pub fn err<T: Serialize>(code: &str, message: impl Into<String>) -> CliEnvelope<T> {
    CliEnvelope {
        ok: false,
        data: None,
        error: Some(CliError {
            code: code.to_string(),
            message: message.into(),
        }),
    }
}

pub fn serve_ok(id: String, data: serde_json::Value) -> CliServeResponse {
    CliServeResponse {
        id,
        ok: true,
        data: Some(data),
        error: None,
    }
}

pub fn serve_err(id: String, code: &str, message: impl Into<String>) -> CliServeResponse {
    CliServeResponse {
        id,
        ok: false,
        data: None,
        error: Some(CliError {
            code: code.to_string(),
            message: message.into(),
        }),
    }
}
```

- [ ] **Step 4: Add session capability**

Modify `status_payload()` in `src-tauri/src/cli/commands.rs` and add `"session"` to the capabilities vector:

```rust
capabilities: vec![
    "providers".to_string(),
    "openclaw".to_string(),
    "mcp".to_string(),
    "prompts".to_string(),
    "skills".to_string(),
    "sessions".to_string(),
    "hermes-memory".to_string(),
    "import-export".to_string(),
    "tools".to_string(),
    "settings".to_string(),
    "plugin".to_string(),
    "session".to_string(),
],
```

- [ ] **Step 5: Run test to verify it passes**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test cli_status status_advertises_session_capability
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/cli/types.rs src-tauri/src/cli/commands.rs src-tauri/tests/cli_status.rs
git commit -m "feat(remote): advertise helper session capability"
```

## Task 2: Helper `--json serve` Mode

**Files:**
- Create: `src-tauri/src/cli/serve.rs`
- Modify: `src-tauri/src/cli/mod.rs`
- Modify: `src-tauri/src/bin/cc-switch-cli.rs`
- Test: `src-tauri/src/cli/serve.rs`

- [ ] **Step 1: Create failing serve handler tests**

Create `src-tauri/src/cli/serve.rs` with tests first:

```rust
use crate::cli::types::{self, CliServeRequest, CliServeResponse};
use serde_json::Value;
use std::io::{self, BufRead, Write};

pub fn handle_line(line: &str) -> CliServeResponse {
    let request: CliServeRequest = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(error) => {
            return types::serve_err(
                "invalid".to_string(),
                "invalid_request",
                format!("Invalid serve request JSON: {error}"),
            );
        }
    };

    let data = super::run_command(&request.command);
    if data.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        types::serve_ok(
            request.id,
            data.get("data").cloned().unwrap_or(Value::Null),
        )
    } else {
        let error = data.get("error").cloned().unwrap_or(Value::Null);
        let code = error
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or("remote_error");
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Remote helper command failed");
        types::serve_err(request.id, code, message)
    }
}

pub fn run_stdio() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = handle_line(&line);
        serde_json::to_writer(&mut stdout, &response)?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_line_dispatches_status() {
        let response = handle_line(r#"{"id":"1","command":["status"]}"#);

        assert_eq!(response.id, "1");
        assert!(response.ok);
        assert!(response.data.expect("data").get("capabilities").is_some());
    }

    #[test]
    fn handle_line_preserves_unsupported_command_error() {
        let response = handle_line(r#"{"id":"2","command":["missing"]}"#);

        assert_eq!(response.id, "2");
        assert!(!response.ok);
        assert_eq!(
            response.error.expect("error").code,
            "unsupported_command"
        );
    }

    #[test]
    fn handle_line_rejects_invalid_json() {
        let response = handle_line("{");

        assert_eq!(response.id, "invalid");
        assert!(!response.ok);
        assert_eq!(response.error.expect("error").code, "invalid_request");
    }
}
```

- [ ] **Step 2: Run tests to verify compile failure**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml cli::serve
```

Expected: FAIL because `cli::serve` is not exported and `run_command` does not exist.

- [ ] **Step 3: Route serve mode and expose dispatcher**

Modify `src-tauri/src/cli/mod.rs`:

```rust
pub mod commands;
pub mod serve;
pub mod types;

use serde_json::Value;

pub enum CliRunResult {
    Json(Value),
    Served,
}

pub fn run(args: &[String]) -> Value {
    match run_entry(args) {
        CliRunResult::Json(value) => value,
        CliRunResult::Served => serde_json::to_value(types::ok(())).expect("serialize serve end"),
    }
}

pub fn run_entry(args: &[String]) -> CliRunResult {
    let args = normalize_args(args);
    if args == ["serve"] {
        return match serve::run_stdio() {
            Ok(()) => CliRunResult::Served,
            Err(error) => CliRunResult::Json(
                serde_json::to_value(types::err::<()>("serve_failed", error.to_string()))
                    .expect("serialize serve error"),
            ),
        };
    }
    CliRunResult::Json(run_command(&args))
}
```

In the same file, keep the current `normalize_args` function unchanged:

```rust
fn normalize_args(args: &[String]) -> Vec<String> {
    args.iter()
        .filter(|arg| arg.as_str() != "--json")
        .cloned()
        .collect()
}
```

Then mechanically rename the existing dispatcher function:

```rust
fn run_normalized(args: &[String]) -> Value {
```

to:

```rust
pub(crate) fn run_command(args: &[String]) -> Value {
```

Do not rewrite the match arms inside that function. The complete first lines after the rename should be:

```rust
pub(crate) fn run_command(args: &[String]) -> Value {
    match args {
        [cmd] if cmd == "status" => serde_json::to_value(types::ok(commands::status_payload()))
            .expect("serialize status response"),
```

After this mechanical rename, there should be no remaining references to `run_normalized`.

- [ ] **Step 4: Avoid extra output after serve exits**

Modify `src-tauri/src/bin/cc-switch-cli.rs`:

```rust
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(feature = "desktop")]
    let response = cc_switch_lib::cli::run_entry(&args);
    #[cfg(not(feature = "desktop"))]
    let response = cli::run_entry(&args);

    match response {
        #[cfg(feature = "desktop")]
        cc_switch_lib::cli::CliRunResult::Json(value) => {
            println!("{}", serde_json::to_string(&value).expect("serialize CLI response"));
        }
        #[cfg(feature = "desktop")]
        cc_switch_lib::cli::CliRunResult::Served => {}
        #[cfg(not(feature = "desktop"))]
        cli::CliRunResult::Json(value) => {
            println!("{}", serde_json::to_string(&value).expect("serialize CLI response"));
        }
        #[cfg(not(feature = "desktop"))]
        cli::CliRunResult::Served => {}
    }
}
```

- [ ] **Step 5: Run serve tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml cli::serve
cargo test --manifest-path src-tauri/Cargo.toml --test cli_status
```

Expected: PASS.

- [ ] **Step 6: Manual smoke test helper serve**

Run:

```bash
printf '%s\n' '{"id":"1","command":["status"]}' '{"id":"2","command":["missing"]}' | cargo run --manifest-path src-tauri/Cargo.toml --bin cc-switch-remote-helper --no-default-features -- --json serve
```

Expected:

```json
{"id":"1","ok":true,"data":{"version":"...","build":...,"platform":"...","arch":"...","capabilities":[...]}}
{"id":"2","ok":false,"data":null,"error":{"code":"unsupported_command","message":"Supported commands: ..."}}
```

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/cli/mod.rs src-tauri/src/cli/serve.rs src-tauri/src/bin/cc-switch-cli.rs
git commit -m "feat(remote): add helper serve mode"
```

## Task 3: Remote Session Types and SSH Serve Args

**Files:**
- Modify: `src-tauri/src/remote/types.rs`
- Modify: `src-tauri/src/remote/ssh.rs`
- Modify: `src-tauri/src/remote/mod.rs`
- Test: `src-tauri/tests/remote_ssh.rs`

- [ ] **Step 1: Write failing SSH serve args test**

Add to `src-tauri/tests/remote_ssh.rs`:

```rust
#[test]
fn ssh_serve_args_start_helper_in_serve_mode() {
    let args = cc_switch_lib::remote::build_ssh_serve_args(&profile());

    assert!(args.contains(&"alice@example.com".to_string()));
    assert_eq!(
        args.last().expect("remote command"),
        "~/.local/bin/cc-switch-remote-helper --json serve"
    );
    assert!(args.contains(&"ControlMaster=no".to_string()));
    assert!(!args.iter().any(|arg| arg == "-S"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test remote_ssh ssh_serve_args_start_helper_in_serve_mode
```

Expected: FAIL because `build_ssh_serve_args` is missing.

- [ ] **Step 3: Add remote session status types**

Modify `src-tauri/src/remote/types.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSessionStatus {
    pub profile_id: String,
    pub state: RemoteSessionState,
    pub last_error: Option<String>,
    pub active_request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RemoteSessionState {
    Idle,
    Connecting,
    Ready,
    Busy,
    Reconnecting,
    Failed,
    Closed,
}
```

Also add the capability enum value:

```rust
Session,
```

inside `RemoteCapability`.

- [ ] **Step 4: Add SSH serve args**

Modify `src-tauri/src/remote/ssh.rs`:

```rust
pub fn build_ssh_serve_args(profile: &RemoteHostProfile) -> Vec<String> {
    let mut args = build_ssh_base_args(profile);
    args.push(format!(
        "{} --json serve",
        shell_quote_helper_path(&profile.helper_path)
    ));
    args
}
```

- [ ] **Step 5: Export new symbols**

Modify `src-tauri/src/remote/mod.rs`:

```rust
pub use ssh::{
    build_helper_install_args, build_helper_install_args_with_source, build_ssh_args,
    build_ssh_serve_args, install_helper_json, run_helper_json, RemoteHelperInstallSource,
};

pub use types::{
    RemoteAuthMethod, RemoteCapability, RemoteCommandError, RemoteCommandRequest,
    RemoteCommandResponse, RemoteConnectionSecret, RemoteHealth, RemoteHostProfile, RemotePlatform,
    RemoteSessionState, RemoteSessionStatus,
};
```

- [ ] **Step 6: Run SSH tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test remote_ssh ssh_serve_args_start_helper_in_serve_mode
cargo test --manifest-path src-tauri/Cargo.toml --test remote_ssh
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/remote/types.rs src-tauri/src/remote/ssh.rs src-tauri/src/remote/mod.rs src-tauri/tests/remote_ssh.rs
git commit -m "feat(remote): add SSH serve transport args"
```

## Task 4: Tauri Remote Session Manager

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/remote/session.rs`
- Modify: `src-tauri/src/remote/mod.rs`
- Test: `src-tauri/tests/remote_session.rs`

- [ ] **Step 1: Add Tokio features**

Modify `src-tauri/Cargo.toml`:

```toml
tokio = { version = "1", features = ["macros", "rt-multi-thread", "time", "sync", "process", "io-util"] }
```

- [ ] **Step 2: Write session protocol parser tests**

Create `src-tauri/tests/remote_session.rs`:

```rust
use cc_switch_lib::remote::{
    parse_session_response_line, RemoteCommandError, RemoteSessionError,
};
use serde_json::json;

#[test]
fn parses_session_success_response() {
    let response = parse_session_response_line(
        r#"{"id":"1","ok":true,"data":{"value":42},"error":null}"#,
    )
    .expect("response");

    assert_eq!(response.id, "1");
    assert!(response.ok);
    assert_eq!(response.data, Some(json!({"value":42})));
}

#[test]
fn parses_session_error_response() {
    let response = parse_session_response_line(
        r#"{"id":"2","ok":false,"data":null,"error":{"code":"unsupported_command","message":"no"}}"#,
    )
    .expect("response");

    assert_eq!(response.id, "2");
    assert!(!response.ok);
    assert_eq!(
        response.error,
        Some(RemoteCommandError {
            code: "unsupported_command".to_string(),
            message: "no".to_string(),
        })
    );
}

#[test]
fn invalid_session_response_is_classified() {
    let error = parse_session_response_line("{").unwrap_err();

    assert!(matches!(error, RemoteSessionError::InvalidJson(_)));
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test remote_session
```

Expected: FAIL because session module and parser are missing.

- [ ] **Step 4: Implement session response model and parser**

Create `src-tauri/src/remote/session.rs`:

```rust
use crate::error::AppError;
use crate::remote::ssh::build_ssh_serve_args;
use crate::remote::types::{
    RemoteCommandError, RemoteCommandResponse, RemoteConnectionSecret, RemoteHostProfile,
    RemoteSessionState, RemoteSessionStatus,
};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::Mutex;

const REMOTE_SESSION_REQUEST_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSessionResponseLine {
    pub id: String,
    pub ok: bool,
    pub data: Option<Value>,
    pub error: Option<RemoteCommandError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteSessionError {
    InvalidJson(String),
    MissingData,
    CommandFailed(RemoteCommandError),
    Io(String),
    Timeout,
    Closed,
}

impl std::fmt::Display for RemoteSessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(message) => write!(f, "Remote session returned invalid JSON: {message}"),
            Self::MissingData => write!(f, "Remote session returned ok without data"),
            Self::CommandFailed(error) => write!(f, "{}: {}", error.code, error.message),
            Self::Io(message) => write!(f, "Remote session I/O failed: {message}"),
            Self::Timeout => write!(f, "Remote session command timed out"),
            Self::Closed => write!(f, "Remote session is closed"),
        }
    }
}

impl std::error::Error for RemoteSessionError {}

pub fn parse_session_response_line(line: &str) -> Result<RemoteSessionResponseLine, RemoteSessionError> {
    serde_json::from_str(line).map_err(|error| RemoteSessionError::InvalidJson(error.to_string()))
}
```

This step intentionally adds only the parser and error types first.

- [ ] **Step 5: Export session parser and errors**

Modify `src-tauri/src/remote/mod.rs`:

```rust
pub mod session;

pub use session::{
    parse_session_response_line, RemoteSessionError, RemoteSessionManager, RemoteSessionResponseLine,
};
```

Use temporary empty structs to satisfy exports before the full manager step:

```rust
pub struct RemoteSessionManager;
```

inside `src-tauri/src/remote/session.rs`.

- [ ] **Step 6: Run parser tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test remote_session
```

Expected: PASS for parser tests.

- [ ] **Step 7: Write manager state tests**

Append to `src-tauri/tests/remote_session.rs`:

```rust
use cc_switch_lib::remote::{RemoteSessionManager, RemoteSessionState};

#[tokio::test]
async fn new_manager_reports_idle_for_unknown_profile() {
    let manager = RemoteSessionManager::default();
    let status = manager.status("missing").await;

    assert_eq!(status.profile_id, "missing");
    assert_eq!(status.state, RemoteSessionState::Idle);
    assert_eq!(status.last_error, None);
}
```

- [ ] **Step 8: Implement manager status shell**

Replace the temporary manager in `src-tauri/src/remote/session.rs` with:

```rust
#[derive(Default)]
pub struct RemoteSessionManager {
    sessions: Arc<Mutex<HashMap<String, RemoteSessionStatus>>>,
}

impl RemoteSessionManager {
    pub async fn status(&self, profile_id: &str) -> RemoteSessionStatus {
        self.sessions
            .lock()
            .await
            .get(profile_id)
            .cloned()
            .unwrap_or(RemoteSessionStatus {
                profile_id: profile_id.to_string(),
                state: RemoteSessionState::Idle,
                last_error: None,
                active_request_id: None,
            })
    }

    async fn set_status(&self, status: RemoteSessionStatus) {
        self.sessions
            .lock()
            .await
            .insert(status.profile_id.clone(), status);
    }
}
```

- [ ] **Step 9: Run manager state test**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test remote_session new_manager_reports_idle_for_unknown_profile
```

Expected: PASS.

- [ ] **Step 10: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/remote/session.rs src-tauri/src/remote/mod.rs src-tauri/tests/remote_session.rs
git commit -m "feat(remote): add session manager foundation"
```

## Task 5: Session Execution Through SSH Child Process

**Files:**
- Modify: `src-tauri/src/remote/session.rs`
- Test: `src-tauri/tests/remote_session.rs`

- [ ] **Step 1: Write command serialization test**

Add to `src-tauri/tests/remote_session.rs`:

```rust
use cc_switch_lib::remote::build_session_request_line;

#[test]
fn session_request_line_serializes_command_vector() {
    let line = build_session_request_line("req-1", &["settings".to_string(), "get".to_string()])
        .expect("request line");

    assert_eq!(
        line,
        r#"{"id":"req-1","command":["settings","get"]}"#
    );
}
```

- [ ] **Step 2: Implement request serialization**

Add to `src-tauri/src/remote/session.rs`:

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoteSessionRequestLine<'a> {
    id: &'a str,
    command: &'a [String],
}

pub fn build_session_request_line(id: &str, command: &[String]) -> Result<String, RemoteSessionError> {
    serde_json::to_string(&RemoteSessionRequestLine { id, command })
        .map_err(|error| RemoteSessionError::InvalidJson(error.to_string()))
}
```

Export it from `src-tauri/src/remote/mod.rs`:

```rust
pub use session::{
    build_session_request_line, parse_session_response_line, RemoteSessionError,
    RemoteSessionManager, RemoteSessionResponseLine,
};
```

- [ ] **Step 3: Run serialization test**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test remote_session session_request_line_serializes_command_vector
```

Expected: PASS.

- [ ] **Step 4: Implement session process skeleton**

Add to `src-tauri/src/remote/session.rs`:

```rust
struct RemoteSessionProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
}

impl RemoteSessionProcess {
    async fn start(
        profile: &RemoteHostProfile,
        secret: Option<&RemoteConnectionSecret>,
    ) -> Result<Self, AppError> {
        let mut command = Command::new("ssh");
        command.args(build_ssh_serve_args(profile));
        crate::remote::ssh::configure_password_auth_for_tokio(profile, secret, &mut command)?;
        command.stdin(std::process::Stdio::piped());
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        let mut child = command
            .spawn()
            .map_err(|error| AppError::Message(format!("Failed to start remote helper session: {error}")))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AppError::Message("Remote helper session stdin unavailable".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::Message("Remote helper session stdout unavailable".to_string()))?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }
}
```

This requires Task 6 to expose password-auth environment setup for Tokio command.

- [ ] **Step 5: Implement one-at-a-time execute on process**

Add to `RemoteSessionProcess`:

```rust
async fn execute_value(
    &mut self,
    request_id: &str,
    command: &[String],
) -> Result<Value, RemoteSessionError> {
    let mut request = build_session_request_line(request_id, command)?;
    request.push('\n');
    self.stdin
        .write_all(request.as_bytes())
        .await
        .map_err(|error| RemoteSessionError::Io(error.to_string()))?;
    self.stdin
        .flush()
        .await
        .map_err(|error| RemoteSessionError::Io(error.to_string()))?;

    let mut line = String::new();
    let read = self
        .stdout
        .read_line(&mut line)
        .await
        .map_err(|error| RemoteSessionError::Io(error.to_string()))?;
    if read == 0 {
        return Err(RemoteSessionError::Closed);
    }

    let response = parse_session_response_line(line.trim())?;
    if response.id != request_id {
        return Err(RemoteSessionError::Io(format!(
            "Remote session response id mismatch: expected {request_id}, got {}",
            response.id
        )));
    }
    if response.ok {
        response.data.ok_or(RemoteSessionError::MissingData)
    } else {
        Err(RemoteSessionError::CommandFailed(
            response.error.unwrap_or(RemoteCommandError {
                code: "remote_error".to_string(),
                message: "Remote helper command failed".to_string(),
            }),
        ))
    }
}
```

- [ ] **Step 6: Implement typed manager execute**

Add to `RemoteSessionManager`:

```rust
pub async fn execute_json<T>(
    &self,
    profile: RemoteHostProfile,
    secret: Option<RemoteConnectionSecret>,
    helper_args: Vec<String>,
) -> Result<T, AppError>
where
    T: DeserializeOwned + Send + 'static,
{
    let value = self.execute_value(profile, secret, helper_args).await?;
    serde_json::from_value(value)
        .map_err(|error| AppError::Message(format!("Remote session returned invalid data: {error}")))
}

async fn execute_value(
    &self,
    profile: RemoteHostProfile,
    secret: Option<RemoteConnectionSecret>,
    helper_args: Vec<String>,
) -> Result<Value, AppError> {
    let request_id = uuid::Uuid::new_v4().to_string();
    self.set_status(RemoteSessionStatus {
        profile_id: profile.id.clone(),
        state: RemoteSessionState::Connecting,
        last_error: None,
        active_request_id: Some(request_id.clone()),
    })
    .await;

    let mut process = RemoteSessionProcess::start(&profile, secret.as_ref()).await?;
    self.set_status(RemoteSessionStatus {
        profile_id: profile.id.clone(),
        state: RemoteSessionState::Busy,
        last_error: None,
        active_request_id: Some(request_id.clone()),
    })
    .await;

    let result = tokio::time::timeout(
        REMOTE_SESSION_REQUEST_TIMEOUT,
        process.execute_value(&request_id, &helper_args),
    )
    .await
    .map_err(|_| AppError::Message(RemoteSessionError::Timeout.to_string()))?
    .map_err(|error| AppError::Message(error.to_string()));

    match &result {
        Ok(_) => {
            self.set_status(RemoteSessionStatus {
                profile_id: profile.id.clone(),
                state: RemoteSessionState::Ready,
                last_error: None,
                active_request_id: None,
            })
            .await;
        }
        Err(error) => {
            self.set_status(RemoteSessionStatus {
                profile_id: profile.id.clone(),
                state: RemoteSessionState::Failed,
                last_error: Some(error.to_string()),
                active_request_id: None,
            })
            .await;
        }
    }

    result
}
```

This step starts a new session per command. The persistent map is added in Task 7. Keeping this intermediate step allows protocol and status plumbing to compile before adding process reuse.

- [ ] **Step 7: Run Rust check**

Run:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --test remote_session
```

Expected: PASS after Task 6 provides the Tokio password-auth helper.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/remote/session.rs src-tauri/src/remote/mod.rs src-tauri/tests/remote_session.rs
git commit -m "feat(remote): execute helper commands through session protocol"
```

## Task 6: Shared Password Auth Setup for Tokio SSH

**Files:**
- Modify: `src-tauri/src/remote/ssh.rs`
- Test: `src-tauri/tests/remote_ssh.rs`

- [ ] **Step 1: Write command env setup test**

Add to `src-tauri/tests/remote_ssh.rs`:

```rust
#[cfg(unix)]
#[test]
fn tokio_password_auth_setup_uses_askpass_env() {
    use cc_switch_lib::remote::{
        configure_password_auth_for_tokio_test, RemoteConnectionSecret,
    };

    let mut profile = profile();
    profile.auth_method = RemoteAuthMethod::Password;
    let secret = RemoteConnectionSecret {
        password: Some("secret".to_string()),
    };
    let mut command = tokio::process::Command::new("ssh");

    let _guard = configure_password_auth_for_tokio_test(&profile, Some(&secret), &mut command)
        .expect("askpass guard");
}
```

- [ ] **Step 2: Extract shared askpass script creation**

Modify `src-tauri/src/remote/ssh.rs`:

```rust
#[cfg(unix)]
fn create_askpass_script() -> Result<tempfile::TempPath, AppError> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

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
    Ok(askpass.into_temp_path())
}
```

- [ ] **Step 3: Add shared password lookup helper**

Add:

```rust
#[cfg(unix)]
fn password_for_profile<'a>(
    profile: &RemoteHostProfile,
    secret: Option<&'a RemoteConnectionSecret>,
) -> Result<Option<String>, AppError> {
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
        .or_else(|| stored_secret.as_ref().and_then(|secret| secret.password.as_deref()))
        .filter(|password| !password.is_empty())
        .ok_or_else(|| AppError::Message("Remote SSH password is required".to_string()))?;

    Ok(Some(password.to_string()))
}
```

- [ ] **Step 4: Add Tokio command configuration**

Add to `src-tauri/src/remote/ssh.rs`:

```rust
#[cfg(unix)]
pub fn configure_password_auth_for_tokio(
    profile: &RemoteHostProfile,
    secret: Option<&RemoteConnectionSecret>,
    command: &mut tokio::process::Command,
) -> Result<Option<tempfile::TempPath>, AppError> {
    let Some(password) = password_for_profile(profile, secret)? else {
        return Ok(None);
    };
    let askpass_path = create_askpass_script()?;
    command.env("SSH_ASKPASS", &askpass_path);
    command.env("SSH_ASKPASS_REQUIRE", "force");
    command.env("DISPLAY", "cc-switch");
    command.env("CC_SWITCH_REMOTE_SSH_PASSWORD", password);
    Ok(Some(askpass_path))
}

#[cfg(test)]
#[cfg(unix)]
pub fn configure_password_auth_for_tokio_test(
    profile: &RemoteHostProfile,
    secret: Option<&RemoteConnectionSecret>,
    command: &mut tokio::process::Command,
) -> Result<Option<tempfile::TempPath>, AppError> {
    configure_password_auth_for_tokio(profile, secret, command)
}
```

For non-Unix:

```rust
#[cfg(not(unix))]
pub fn configure_password_auth_for_tokio(
    profile: &RemoteHostProfile,
    _secret: Option<&RemoteConnectionSecret>,
    _command: &mut tokio::process::Command,
) -> Result<Option<()>, AppError> {
    if matches!(profile.auth_method, RemoteAuthMethod::Password) {
        return Err(AppError::Message(
            "Remote SSH password auth is only supported on Unix desktops".to_string(),
        ));
    }
    Ok(None)
}
```

- [ ] **Step 5: Reuse helpers in existing std command configuration**

Modify the Unix `configure_password_auth` body so it uses `password_for_profile` and `create_askpass_script` instead of duplicating script creation:

```rust
let Some(password) = password_for_profile(profile, secret)? else {
    return Ok(None);
};
let askpass_path = create_askpass_script()?;
command.env("SSH_ASKPASS", &askpass_path);
command.env("SSH_ASKPASS_REQUIRE", "force");
command.env("DISPLAY", "cc-switch");
command.env("CC_SWITCH_REMOTE_SSH_PASSWORD", password);
Ok(Some(askpass_path))
```

- [ ] **Step 6: Run SSH tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test remote_ssh
cargo test --manifest-path src-tauri/Cargo.toml --test remote_session
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/remote/ssh.rs src-tauri/tests/remote_ssh.rs
git commit -m "feat(remote): share password auth setup with session transport"
```

## Task 7: Persistent Per-Host Session Reuse

**Files:**
- Modify: `src-tauri/src/remote/session.rs`
- Test: `src-tauri/tests/remote_session.rs`

- [ ] **Step 1: Add session factory seam**

Modify `src-tauri/src/remote/session.rs` to make process creation replaceable in tests:

```rust
#[async_trait::async_trait]
trait RemoteSessionExecutor: Send + Sync {
    async fn execute(&self, request_id: &str, command: &[String]) -> Result<Value, RemoteSessionError>;
    async fn close(&self);
}
```

If `async-trait` is not already a dependency, avoid adding it and use boxed futures:

```rust
use std::future::Future;
use std::pin::Pin;

trait RemoteSessionExecutor: Send + Sync {
    fn execute<'a>(
        &'a self,
        request_id: &'a str,
        command: &'a [String],
    ) -> Pin<Box<dyn Future<Output = Result<Value, RemoteSessionError>> + Send + 'a>>;

    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
}
```

Use the boxed future version to avoid adding a new dependency.

- [ ] **Step 2: Implement executor for SSH process**

Add:

```rust
impl RemoteSessionExecutor for Mutex<RemoteSessionProcess> {
    fn execute<'a>(
        &'a self,
        request_id: &'a str,
        command: &'a [String],
    ) -> Pin<Box<dyn Future<Output = Result<Value, RemoteSessionError>> + Send + 'a>> {
        Box::pin(async move {
            self.lock()
                .await
                .execute_value(request_id, command)
                .await
        })
    }

    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let mut process = self.lock().await;
            let _ = process.child.kill().await;
        })
    }
}
```

- [ ] **Step 3: Add active session map**

Change the manager fields:

```rust
#[derive(Default)]
pub struct RemoteSessionManager {
    sessions: Arc<Mutex<HashMap<String, ManagedRemoteSession>>>,
}

struct ManagedRemoteSession {
    status: RemoteSessionStatus,
    executor: Option<Arc<dyn RemoteSessionExecutor>>,
}
```

- [ ] **Step 4: Update status storage**

Replace `set_status` and `status` with versions that operate on `ManagedRemoteSession`:

```rust
pub async fn status(&self, profile_id: &str) -> RemoteSessionStatus {
    self.sessions
        .lock()
        .await
        .get(profile_id)
        .map(|session| session.status.clone())
        .unwrap_or(RemoteSessionStatus {
            profile_id: profile_id.to_string(),
            state: RemoteSessionState::Idle,
            last_error: None,
            active_request_id: None,
        })
}

async fn set_status(&self, status: RemoteSessionStatus) {
    let mut sessions = self.sessions.lock().await;
    sessions
        .entry(status.profile_id.clone())
        .and_modify(|session| session.status = status.clone())
        .or_insert(ManagedRemoteSession {
            status,
            executor: None,
        });
}
```

- [ ] **Step 5: Add get-or-start executor method**

Add:

```rust
async fn get_or_start_executor(
    &self,
    profile: &RemoteHostProfile,
    secret: Option<&RemoteConnectionSecret>,
) -> Result<Arc<dyn RemoteSessionExecutor>, AppError> {
    if let Some(executor) = self
        .sessions
        .lock()
        .await
        .get(&profile.id)
        .and_then(|session| session.executor.clone())
    {
        return Ok(executor);
    }

    self.set_status(RemoteSessionStatus {
        profile_id: profile.id.clone(),
        state: RemoteSessionState::Connecting,
        last_error: None,
        active_request_id: None,
    })
    .await;

    let process = RemoteSessionProcess::start(profile, secret).await?;
    let executor: Arc<dyn RemoteSessionExecutor> = Arc::new(Mutex::new(process));
    let mut sessions = self.sessions.lock().await;
    sessions.insert(
        profile.id.clone(),
        ManagedRemoteSession {
            status: RemoteSessionStatus {
                profile_id: profile.id.clone(),
                state: RemoteSessionState::Ready,
                last_error: None,
                active_request_id: None,
            },
            executor: Some(executor.clone()),
        },
    );
    Ok(executor)
}
```

- [ ] **Step 6: Update execute to reuse executor**

Modify `execute_value`:

```rust
let request_id = uuid::Uuid::new_v4().to_string();
let executor = self.get_or_start_executor(&profile, secret.as_ref()).await?;
self.set_status(RemoteSessionStatus {
    profile_id: profile.id.clone(),
    state: RemoteSessionState::Busy,
    last_error: None,
    active_request_id: Some(request_id.clone()),
})
.await;

let result = tokio::time::timeout(
    REMOTE_SESSION_REQUEST_TIMEOUT,
    executor.execute(&request_id, &helper_args),
)
.await
.map_err(|_| AppError::Message(RemoteSessionError::Timeout.to_string()))?
.map_err(|error| AppError::Message(error.to_string()));
```

Keep the existing Ready/Failed status update after the result.

- [ ] **Step 7: Add close API**

Add:

```rust
pub async fn close(&self, profile_id: &str) -> bool {
    let session = self.sessions.lock().await.remove(profile_id);
    if let Some(session) = session {
        if let Some(executor) = session.executor {
            executor.close().await;
        }
        true
    } else {
        false
    }
}
```

- [ ] **Step 8: Run session tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test remote_session
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/remote/session.rs src-tauri/tests/remote_session.rs
git commit -m "feat(remote): reuse persistent helper sessions per host"
```

## Task 8: Route Remote Commands Through Session Manager

**Files:**
- Modify: `src-tauri/src/commands/remote.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/commands/remote.rs`

- [ ] **Step 1: Register manager as Tauri state**

Modify `src-tauri/src/lib.rs` in the builder setup:

```rust
.manage(crate::remote::RemoteSessionManager::default())
```

Place it next to other `.manage(...)` calls.

- [ ] **Step 2: Add manager parameter to shared helper executor**

Modify `run_remote_helper_json` in `src-tauri/src/commands/remote.rs`:

```rust
async fn run_remote_helper_json<T>(
    manager: tauri::State<'_, crate::remote::RemoteSessionManager>,
    profile: RemoteHostProfile,
    helper_args: Vec<String>,
    secret: Option<RemoteConnectionSecret>,
    task_name: &'static str,
) -> Result<T, String>
where
    T: DeserializeOwned + Send + 'static,
{
    validate_profile(&profile).map_err(|e| e.to_string())?;
    manager
        .execute_json(profile.clone(), secret.clone(), helper_args.clone())
        .await
        .or_else(|session_error| {
            if should_fallback_to_one_shot(&session_error.to_string()) {
                Err(session_error)
            } else {
                Err(session_error)
            }
        })
        .map_err(|e| format!("{task_name} task failed: {e}"))
}
```

Keep fallback disabled in this first connection step. The `or_else` block is intentionally equivalent so the function shape is ready for Task 9 without silently hiding stale-helper problems.

- [ ] **Step 3: Add `manager` parameter to every Tauri command that calls `run_remote_helper_json`**

For each command:

```rust
#[tauri::command]
pub async fn remote_get_settings(
    manager: tauri::State<'_, crate::remote::RemoteSessionManager>,
    profile: RemoteHostProfile,
    secret: Option<RemoteConnectionSecret>,
) -> Result<AppSettings, String> {
    run_remote_helper_json(
        manager,
        profile,
        vec!["settings".to_string(), "get".to_string()],
        secret,
        "Remote settings get",
    )
    .await
}
```

Apply the same pattern to provider, MCP, prompt, skills, sessions, Hermes memory, OpenClaw, tools, settings, plugin, import/export commands. Do not change profile CRUD, profile validation, build-command debug helpers, health check, or helper install in this task.

- [ ] **Step 4: Add session status command**

Add:

```rust
#[tauri::command]
pub async fn remote_get_session_status(
    manager: tauri::State<'_, crate::remote::RemoteSessionManager>,
    profile_id: String,
) -> Result<crate::remote::RemoteSessionStatus, String> {
    Ok(manager.status(&profile_id).await)
}

#[tauri::command]
pub async fn remote_close_session(
    manager: tauri::State<'_, crate::remote::RemoteSessionManager>,
    profile_id: String,
) -> Result<bool, String> {
    Ok(manager.close(&profile_id).await)
}
```

- [ ] **Step 5: Register Tauri commands**

Modify the `tauri::generate_handler!` list in `src-tauri/src/lib.rs` and add:

```rust
commands::remote::remote_get_session_status,
commands::remote::remote_close_session,
```

- [ ] **Step 6: Run Rust check**

Run:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands/remote.rs src-tauri/src/lib.rs
git commit -m "feat(remote): route helper commands through session manager"
```

## Task 9: Helper Upgrade Required and Compatibility Classification

**Files:**
- Modify: `src-tauri/src/commands/remote.rs`
- Modify: `src-tauri/src/remote/session.rs`
- Test: `src-tauri/src/commands/remote.rs`

- [ ] **Step 1: Add helper capability classifier test**

Add to `src-tauri/src/commands/remote.rs` tests:

```rust
#[test]
fn detects_missing_session_capability() {
    let status = json!({
        "version": "3.16.3",
        "build": "abc123",
        "platform": "linux",
        "arch": "x86_64",
        "capabilities": ["providers", "settings"]
    });

    let health = remote_health_from_status_with_latest_result(status, None);

    assert!(!health.capabilities.contains(&RemoteCapability::Session));
    assert_eq!(
        health.helper_update_error.as_deref(),
        Some("远程 Helper 版本过旧，不支持持久会话；请更新 Helper。")
    );
}
```

- [ ] **Step 2: Parse session capability**

Modify the capability parser in `src-tauri/src/commands/remote.rs`:

```rust
"session" => Some(RemoteCapability::Session),
```

- [ ] **Step 3: Set helper update guidance for old helper**

In `remote_health_from_status_with_latest_result`, after capabilities are parsed:

```rust
let session_missing = !capabilities.contains(&RemoteCapability::Session);
let helper_update_error = if session_missing {
    Some("远程 Helper 版本过旧，不支持持久会话；请更新 Helper。".to_string())
} else {
    None
};
```

Preserve any existing `helper_update_error` from GitHub query failures by only setting this message when no previous error exists:

```rust
let helper_update_error = helper_update_error.or_else(|| {
    session_missing.then(|| "远程 Helper 版本过旧，不支持持久会话；请更新 Helper。".to_string())
});
```

- [ ] **Step 4: Add clear session startup error**

In `src-tauri/src/remote/session.rs`, when SSH exits before returning a valid line, return:

```rust
RemoteSessionError::Closed
```

and let the Display implementation show:

```rust
"Remote helper session closed before returning a response"
```

- [ ] **Step 5: Run tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml detects_missing_session_capability
cargo test --manifest-path src-tauri/Cargo.toml remote
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/remote.rs src-tauri/src/remote/session.rs
git commit -m "feat(remote): classify helper session compatibility"
```

## Task 10: Frontend Remote Session API

**Files:**
- Modify: `src/lib/api/remote.ts`
- Modify: `src/lib/query/remote.ts`
- Test: `src/lib/managementTarget.test.ts` or `src/lib/query/remote.test.ts`

- [ ] **Step 1: Add frontend status types**

Modify `src/lib/api/remote.ts`:

```ts
export type RemoteSessionState =
  | "idle"
  | "connecting"
  | "ready"
  | "busy"
  | "reconnecting"
  | "failed"
  | "closed";

export interface RemoteSessionStatus {
  profileId: string;
  state: RemoteSessionState;
  lastError?: string;
  activeRequestId?: string;
}
```

- [ ] **Step 2: Add remote API methods**

Add to `remoteApi`:

```ts
getSessionStatus(profileId: string): Promise<RemoteSessionStatus> {
  return invoke<RemoteSessionStatus>("remote_get_session_status", {
    profileId,
  });
},

closeSession(profileId: string): Promise<boolean> {
  return invoke<boolean>("remote_close_session", { profileId });
},
```

- [ ] **Step 3: Add query keys and hook**

Modify `src/lib/query/remote.ts`:

```ts
import { useQuery } from "@tanstack/react-query";
import { remoteApi, type RemoteHostProfile } from "@/lib/api";

export const remoteQueryKeys = {
  all: ["remote"] as const,
  host: (id: string) => ["remote", "host", id] as const,
  session: (id: string) => ["remote", "host", id, "session"] as const,
};

export function useRemoteSessionStatus(profile?: RemoteHostProfile | null) {
  return useQuery({
    queryKey: profile
      ? remoteQueryKeys.session(profile.id)
      : ["remote", "host", "none", "session"],
    queryFn: () => remoteApi.getSessionStatus(profile!.id),
    enabled: Boolean(profile),
    refetchInterval: profile ? 2_000 : false,
    staleTime: 1_000,
  });
}
```

Keep `useValidateRemoteProfile` in the same file.

- [ ] **Step 4: Write query key test**

Create `src/lib/query/remote.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { remoteQueryKeys } from "./remote";

describe("remote query keys", () => {
  it("scopes session status by host id", () => {
    expect(remoteQueryKeys.session("host-1")).toEqual([
      "remote",
      "host",
      "host-1",
      "session",
    ]);
  });
});
```

- [ ] **Step 5: Run frontend tests**

Run:

```bash
pnpm vitest run src/lib/query/remote.test.ts
pnpm exec tsc --noEmit
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/lib/api/remote.ts src/lib/query/remote.ts src/lib/query/remote.test.ts
git commit -m "feat(remote): expose session status to frontend"
```

## Task 11: Remote Health UI for Session Capability

**Files:**
- Modify: `src/components/remote/RemoteHealthPanel.tsx`
- Modify: translation files if remote health text is stored there
- Test: `tests/components/RemoteHealthPanel.test.tsx`

- [ ] **Step 1: Write missing session capability test**

Add to `tests/components/RemoteHealthPanel.test.tsx`:

```tsx
it("shows helper upgrade requirement when session capability is missing", async () => {
  render(
    <RemoteHealthPanel
      profile={profile}
      health={{
        reachable: true,
        helperInstalled: true,
        helperVersion: "3.16.3",
        helperBuild: "abc123",
        helperArch: "x86_64",
        helperUpdateAvailable: true,
        helperUpdateError: "远程 Helper 版本过旧，不支持持久会话；请更新 Helper。",
        platform: "linux",
        capabilities: ["providers", "settings"],
      }}
    />,
  );

  expect(
    screen.getByText("远程 Helper 版本过旧，不支持持久会话；请更新 Helper。"),
  ).toBeInTheDocument();
});
```

Adapt the render helper to the existing test setup.

- [ ] **Step 2: Render session capability state**

Modify `RemoteHealthPanel.tsx`:

```tsx
const hasSessionCapability = health?.capabilities.includes("session") ?? false;
const sessionUpgradeMessage =
  health?.helperInstalled && !hasSessionCapability
    ? t("remote.health.sessionUpgradeRequired", {
        defaultValue: "远程 Helper 版本过旧，不支持持久会话；请更新 Helper。",
      })
    : null;
```

Render near existing helper update messages:

```tsx
{sessionUpgradeMessage ? (
  <Alert variant="warning">
    <AlertCircle className="h-4 w-4" />
    <AlertDescription>{sessionUpgradeMessage}</AlertDescription>
  </Alert>
) : null}
```

Use the existing project alert component/imports. If the component uses a different warning pattern, reuse that exact local pattern.

- [ ] **Step 3: Run component test**

Run:

```bash
pnpm vitest run tests/components/RemoteHealthPanel.test.tsx
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/components/remote/RemoteHealthPanel.tsx tests/components/RemoteHealthPanel.test.tsx
git commit -m "feat(remote): show helper session upgrade state"
```

## Task 12: Panel-Local Loading and Stale Data

**Files:**
- Modify: `src/lib/query/queries.ts`
- Modify: remote-aware hooks that fetch providers/settings/skills/tools
- Modify: remote panels that currently trigger full-page loading
- Test: existing query and component tests

- [ ] **Step 1: Preserve previous remote provider data during refetch**

Confirm `useProvidersQuery` already has:

```ts
placeholderData: keepPreviousData,
staleTime: target.type === "remote" ? 30_000 : 0,
refetchOnWindowFocus: target.type === "local",
```

If any remote query lacks `keepPreviousData`, add it with target-scoped query keys.

- [ ] **Step 2: Add transport status usage to remote shell**

In the component that owns the active remote target, call:

```tsx
const sessionStatus = useRemoteSessionStatus(
  activeTarget.type === "remote" ? activeTarget.profile : null,
);
```

Pass `sessionStatus.data` into remote panels that need transport state.

- [ ] **Step 3: Add a shared status label helper**

Create or add near remote utilities:

```ts
export function remoteSessionStatusLabel(state?: RemoteSessionState): string {
  switch (state) {
    case "connecting":
      return "正在连接远程主机";
    case "busy":
      return "正在执行远程操作";
    case "reconnecting":
      return "正在重新连接远程主机";
    case "failed":
      return "远程连接失败";
    case "closed":
      return "远程连接已关闭";
    case "ready":
      return "远程连接就绪";
    default:
      return "远程连接未建立";
  }
}
```

Prefer i18n keys if the target file already uses `t(...)`; otherwise keep this helper close to the component and migrate text into i18n in the same task.

- [ ] **Step 4: Replace global-looking loading with panel-local loading**

For each remote panel, render stale data plus a small panel indicator:

```tsx
{target.type === "remote" && query.isFetching ? (
  <div className="text-muted-foreground flex items-center gap-2 text-sm">
    <Loader2 className="h-4 w-4 animate-spin" />
    {t("remote.loadingPanel", { defaultValue: "正在刷新远程数据" })}
  </div>
) : null}
```

Do not block the entire page when previous data exists:

```tsx
const showInitialLoading = query.isLoading && !query.data;
const showStaleRefresh = query.isFetching && Boolean(query.data);
```

- [ ] **Step 5: Add component test for stale provider data**

In the provider-related component test, assert that existing provider names remain visible while remote query refetches:

```tsx
expect(screen.getByText("Existing Provider")).toBeInTheDocument();
expect(screen.getByText("正在刷新远程数据")).toBeInTheDocument();
```

Use the actual rendered provider component and existing test helpers.

- [ ] **Step 6: Run frontend checks**

Run:

```bash
pnpm vitest run src/lib/query/queries.test.ts tests/components/RemoteHealthPanel.test.tsx
pnpm exec tsc --noEmit
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/lib/query src/components tests/components
git commit -m "feat(remote): use panel-local remote loading states"
```

## Task 13: End-to-End Verification and Release Readiness

**Files:**
- Modify docs only if verification reveals user-facing behavior that needs documenting.

- [ ] **Step 1: Run Rust remote test suite**

Run:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml --test remote_ssh
cargo test --manifest-path src-tauri/Cargo.toml --test remote_session
cargo test --manifest-path src-tauri/Cargo.toml --test cli_status
cargo test --manifest-path src-tauri/Cargo.toml remote
```

Expected: PASS.

- [ ] **Step 2: Run frontend test suite slice**

Run:

```bash
pnpm vitest run src/lib/query/remote.test.ts src/lib/query/queries.test.ts tests/components/RemoteHealthPanel.test.tsx tests/components/RemoteSettingsPage.test.tsx
pnpm exec tsc --noEmit
pnpm exec prettier --check src-tauri/src/remote src-tauri/src/cli src/lib/api/remote.ts src/lib/query/remote.ts
```

Expected: PASS.

- [ ] **Step 3: Build helper locally**

Run:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --bin cc-switch-remote-helper --no-default-features
```

Expected: PASS and produces `src-tauri/target/debug/cc-switch-remote-helper`.

- [ ] **Step 4: Manual local serve smoke test**

Run:

```bash
printf '%s\n' '{"id":"1","command":["status"]}' '{"id":"2","command":["settings","get"]}' | src-tauri/target/debug/cc-switch-remote-helper --json serve
```

Expected:

- Two JSON lines.
- First line has `"id":"1"` and `"ok":true`.
- First line capabilities include `"session"`.
- Second line has `"id":"2"` and either valid settings data or a clear command error.

- [ ] **Step 5: Manual remote validation**

Against a real test host:

1. Install the new helper binary.
2. Open the Tauri app.
3. Select the remote host.
4. Run health check.
5. Open providers, settings, skills, and tools pages.
6. Switch providers twice.
7. Save a remote setting.
8. Kill the SSH process from the local machine.
9. Trigger another remote action.

Expected:

- Repeated commands reuse the same SSH session until it is killed or closed.
- After killing SSH, the UI shows reconnect/failure state instead of freezing.
- Provider/settings/skills/tools panels show local loading indicators.
- No remote provider secrets or API keys appear in desktop logs.

- [ ] **Step 6: Check git diff**

Run:

```bash
git diff --check
git status --short
```

Expected:

- No whitespace errors.
- Only intentional files are modified.

- [ ] **Step 7: Commit verification docs if changed**

If docs changed:

```bash
git add docs
git commit -m "docs(remote): document session validation"
```

If no docs changed, do not create an empty commit.
