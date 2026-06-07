# Remote Session Management Design

## Goal

Improve long-term remote-management responsiveness by replacing one SSH process per remote action with a persistent SSH-backed helper session, while keeping local and remote management isolated and preserving an upstream-friendly merge surface.

## Current Problem

Remote management currently follows this shape:

```text
React page or hook
  -> remoteApi
  -> Tauri remote command
  -> spawn_blocking
  -> ssh host helper --json <command>
  -> parse one JSON response
```

This is reliable and simple, but every page load, switch, save, and health check pays the cost of launching `ssh`, authenticating, starting the helper, loading remote state, and parsing a single response. When several remote panels load together, the desktop app looks stuck even if each command is technically running off the main thread.

OpenSSH `ControlMaster` is not an acceptable long-term default for this project. It improves some repeated connections, but it depends on local control sockets and user SSH configuration, and it has already produced `getsockname failed: Not a socket` failures. Remote responsiveness should be owned by the application transport layer, not by implicit OpenSSH socket reuse.

## Recommended Architecture

Use an SSH stdio JSON-RPC session.

The desktop app opens one SSH process per active remote host:

```bash
ssh <host> <helper_path> --json serve
```

The remote helper stays alive and reads newline-delimited JSON requests from stdin. It writes newline-delimited JSON responses to stdout. The Tauri backend owns the child process, request queue, timeouts, reconnect behavior, and lifecycle state.

```text
React shared/local pages
  -> remoteApi
  -> Tauri remote commands
  -> RemoteSessionManager
  -> RemoteSession
  -> SSH child process
  -> cc-switch-remote-helper --json serve
  -> existing CLI command dispatcher
  -> Rust core/service logic on the remote host
```

The existing one-shot SSH command path remains as a compatibility and diagnostic path, but it should no longer be the preferred path for normal remote UI interactions after the remote helper supports `serve`.

## Design Principles

- Keep local and remote state separate. The session manager must never make local app state the source of truth for a remote host.
- Keep remote business logic in Rust helper commands that reuse existing service/core logic.
- Keep the frontend API shape stable. Pages should keep calling `remoteApi.getSettings`, `remoteApi.getProviders`, and similar methods without knowing whether the transport is one-shot SSH or a persistent session.
- Avoid remote daemon requirements. The remote host should not need systemd services, open TCP ports, TLS certificates, or background installation beyond the helper binary.
- Make unsupported helper versions obvious. If a helper does not support `serve`, the UI should show that an upgrade is required instead of silently feeling slow or broken.
- Prefer sequential command execution in the first version. It is simpler and safer for mixed read/write operations. Read concurrency can be added later only after command semantics are classified.

## Components

### `RemoteSessionManager`

Location: `src-tauri/src/remote/session.rs`

Responsibilities:

- Store sessions keyed by `profile.id`.
- Start a session on first use or explicit connect.
- Reuse a ready session for subsequent commands.
- Serialize requests per host through a queue.
- Apply per-request timeouts.
- Mark sessions failed when SSH exits, stdout closes, JSON parsing fails, or a timeout expires.
- Close sessions when the user switches away, deletes a host, app exits, or a session is idle for a configured period.

State model:

```text
idle -> connecting -> ready -> busy -> ready
ready -> reconnecting -> ready
ready -> failed
failed -> connecting
ready -> closing -> idle
```

The first implementation should expose enough status for UI panels to show local loading state:

- `idle`
- `connecting`
- `ready`
- `busy`
- `reconnecting`
- `failed`
- `closed`

### `RemoteSession`

Location: `src-tauri/src/remote/session.rs` or `src-tauri/src/remote/transport.rs`

Responsibilities:

- Own one `ssh` child process.
- Own helper stdin/stdout.
- Assign request IDs.
- Write one JSON request per line.
- Read one JSON response per line.
- Match responses to the active request.
- Capture stderr for diagnostics.
- Kill the child process on close or hard failure.

The first version should process one request at a time. This avoids response routing complexity and prevents write operations from racing each other.

### Helper `serve` Mode

Location: `src-tauri/src/cli/serve.rs`

The helper should support:

```bash
cc-switch-remote-helper --json serve
```

Protocol request:

```json
{"id":"request-1","command":["providers","list"]}
```

Protocol success response:

```json
{"id":"request-1","ok":true,"data":{}}
```

Protocol error response:

```json
{"id":"request-1","ok":false,"error":{"code":"unsupported_command","message":"Supported commands: status, providers, openclaw, mcp, prompts, skills, sessions, hermes-memory, import-export, tools, settings, and plugin"}}
```

The `command` field deliberately matches the existing one-shot helper command vector. This keeps the dispatcher shared and prevents a second remote command system from drifting.

### Remote Adapter

Location: `src-tauri/src/commands/remote.rs`

The existing Tauri commands should keep their public command names. Internally they should call a shared execution function:

```text
run_remote_helper_json
  -> RemoteSessionManager.execute(profile, secret, helper_args)
  -> fallback one-shot SSH only when explicitly allowed
```

This keeps the TypeScript `remoteApi` stable and minimizes frontend churn.

### Frontend Status and Cache

Location:

- `src/lib/api/remote.ts`
- `src/lib/query/remote.ts`
- existing remote-aware hooks and panels

The UI should distinguish transport state from data state:

- Transport state: connecting, reconnecting, disconnected, helper upgrade required.
- Data state: loading providers, saving settings, loading skills, checking tool versions.

Remote page transitions should not trigger full-app cursor loading. Each panel should show its own loading or stale-data state.

## Data Flow

### First Remote Read

```text
User opens remote providers
  -> remoteApi.getProviders(profile)
  -> Tauri remote_get_provider_state
  -> RemoteSessionManager finds no ready session
  -> spawn ssh helper --json serve
  -> send {"id":"1","command":["providers","state"]}
  -> helper dispatches existing provider command
  -> response returns provider state
  -> UI renders providers
```

### Subsequent Remote Read

```text
User opens remote settings
  -> remoteApi.getSettings(profile)
  -> RemoteSessionManager reuses ready session
  -> send {"id":"2","command":["settings","get"]}
  -> response returns remote settings
```

### Write Operation

```text
User saves remote settings
  -> remoteApi.saveSettings(profile, settings)
  -> enqueue settings save command
  -> helper writes remote settings on remote host
  -> response confirms success
  -> UI invalidates remote target caches
```

## Compatibility and Upgrade Strategy

The helper `status` command should advertise a new capability:

```json
"session"
```

The desktop app should prefer session mode only when the helper reports that capability. If helper status cannot run because the helper is missing, the current install flow remains unchanged.

Fallback behavior:

- Helper missing: show install helper action.
- Helper installed but lacks `session`: show helper upgrade required for fast remote management.
- Session start fails: show the SSH/helper error with a retry action.
- Session fails after being ready: mark stale data, show reconnect, and retry on the next command.

The one-shot SSH path may remain behind an explicit compatibility mode during the transition. It should not mask a stale helper by silently making the UI slow.

## Error Handling

Errors should be classified before they reach the UI:

- Authentication required or failed.
- Host unreachable.
- SSH process failed before helper start.
- Helper missing or wrong path.
- Helper version too old.
- Helper exited during session.
- Helper returned invalid JSON.
- Command unsupported by helper.
- Command timed out.

The UI should show the action that can resolve the problem:

- Enter password.
- Install helper.
- Update helper.
- Retry connection.
- Check host SSH config.

## Security

- Passwords remain in the local secret storage path already used by remote profiles.
- Remote API keys and provider secrets stay on the remote host.
- JSON-RPC traffic flows through SSH stdio only; no remote TCP port is opened.
- Session logs must not include passwords, API keys, provider tokens, or full command payloads that can contain secrets.
- The session manager must terminate helper processes when profiles are deleted or the app exits.

## Upstream Sync Impact

This design keeps upstream merge impact low by concentrating changes in remote-specific layers:

- `src-tauri/src/remote/session.rs`
- `src-tauri/src/remote/transport.rs`
- `src-tauri/src/cli/serve.rs`
- `src-tauri/src/cli/commands.rs`
- `src-tauri/src/commands/remote.rs`
- `src/lib/api/remote.ts`
- `src/lib/query/remote.ts`

Local provider, MCP, prompt, skill, settings, and session code should not be modified just to support persistent remote transport. When upstream adds a local feature, the remote project should map that feature through helper commands and remote adapters rather than editing the local feature implementation in place.

## Phases

### Phase 1: Session Transport

Deliver:

- Helper `--json serve` mode.
- Remote session manager in Tauri.
- Sequential request queue.
- Timeout and close handling.
- Unit tests for protocol parsing and session state transitions.
- Integration-style tests that verify remote commands use session execution when capability is available.

Success criteria:

- Multiple remote commands for the same host reuse one SSH process.
- Unsupported helper versions produce an upgrade-required message.
- Existing one-shot helper commands still work for diagnostics and installation.

### Phase 2: UI Responsiveness

Deliver:

- Remote transport status surfaced to the frontend.
- Panel-local loading and saving states.
- Stale-data rendering when reconnecting.
- Query invalidation scoped by remote host ID.
- Removal of avoidable full-page or global loading behavior for remote actions.

Success criteria:

- Switching between remote provider, settings, skills, and tool pages no longer makes the whole app feel frozen.
- Slow remote operations show progress in the relevant panel.
- Failed remote actions leave existing data visible when safe.

### Phase 3: Batch Reads and Preload

Deliver:

- Optional helper batch command for read-heavy startup paths.
- Remote host activation preload for health, settings summary, provider state, and tool status summary.
- Cache warming on target switch.

Success criteria:

- Selecting a remote host quickly renders a useful shell with cached or preloaded data.
- Page transitions avoid repeated identical reads.
- Cache invalidation remains target-scoped and does not mix local and remote state.

## Out of Scope

- Remote daemon installation.
- Opening remote TCP ports.
- TLS or token-based RPC outside SSH.
- Rewriting local pages into separate remote-only pages.
- Making all remote commands concurrent in the first session version.
- Moving provider secrets or API keys into the local desktop database.

## Testing Strategy

Rust tests:

- Helper serve request/response parsing.
- Dispatcher reuse between one-shot commands and serve mode.
- Session manager state transitions.
- Timeout behavior.
- Child process exit handling.
- Fallback and upgrade-required classification.

Frontend tests:

- Remote queries preserve host-scoped cache keys.
- Remote panels show local loading states.
- Stale data remains visible during reconnect when safe.
- Helper-upgrade-required state appears when session capability is missing.

Manual validation:

- Add a password-auth remote host.
- Install or update helper.
- Switch providers, settings, skills, and tool pages repeatedly.
- Confirm only one SSH session is used for repeated commands on the same host.
- Kill the SSH session and confirm the UI shows reconnect behavior instead of freezing.
