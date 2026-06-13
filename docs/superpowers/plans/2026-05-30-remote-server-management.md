# Remote Server Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add remote Linux/macOS server management from the CC Switch desktop app while keeping local and remote state independent and preserving an upstream-friendly merge surface.

**Architecture:** The desktop app manages remote connection profiles and calls a Rust CLI helper on the remote host over SSH. The CLI helper reuses Rust core/service logic where possible and returns stable JSON responses. Remote UI code is isolated behind a remote adapter so existing local provider, MCP, prompt, and skill flows remain minimally changed.

**Tech Stack:** Rust/Tauri 2, rusqlite, serde JSON command contracts, React 18, TanStack Query, existing shadcn-style UI components, SSH command execution through Rust `std::process::Command`, optional NPM installer wrapper for Rust release binaries.

---

## Next Phase: Remote Routing Parity Hardening

Audit date: 2026-06-12.

The current remote routing implementation has real frontend, Tauri, helper CLI, and shared Rust proxy-service wiring for routing runtime, app routing, failover queue operations, rectifier/optimizer settings, and global outbound proxy settings. The next phase must focus on closing the remaining parity and robustness gaps rather than adding more surface area.

- [x] Make failover health state target-aware. `useProviderHealth`, `useResetCircuitBreaker`, `useCircuitBreakerConfig`, and `useCircuitBreakerStats` now accept a management target, use target-keyed query caches, and call remote helper commands for remote hosts. Remote routing queue health badges no longer read local provider health.
- [x] Fix remote failover query invalidation. Remote failover mutations now invalidate target-keyed `providers`, `proxyStatus`, `failoverQueue`, `availableProvidersForFailover`, `providerHealth`, and `circuitBreakerStats` queries. Local-only query keys are refreshed only for local mutations.
- [x] Make remote runtime mutations failure-preserving. Remote app routing config writes now roll back database state if runtime takeover application fails. Remote auto failover rolls back the config and any automatically-added queue item if switching to the P1 provider fails. Command-level failures do not stop the long-lived helper session.
- [x] Collapse the remote routing settings shell toward the local routing page structure. Remote routing uses the same accordion grouping, icons, default-collapsed behavior, and local title translation keys for proxy, failover, rectifier, and global outbound proxy. Remote-specific differences are kept inside the existing sections as targeted hints or capability gates.
- [x] Add explicit remote homepage display settings. Per-remote-host settings now control whether the main page shows the active-app remote routing toggle and the remote failover toggle. These settings live in the remote routing settings page, not the remote general page.
- [x] Reconcile homepage switch count and semantics. The remote homepage no longer shows a separate routing runtime master switch. It mirrors local homepage semantics with an app-specific routing entry plus optional failover entry, both hidden by default and controlled by remote-host settings.
- [ ] Add routing helper command round-trip tests. Circuit breaker config/health/reset/stats now have a temp-home JSON CLI round-trip test. Still expand coverage for `routing-config global`, `set-global`, `app`, `set-app`, failover queue add/remove/list, `set-auto-failover`, rectifier/optimizer get/set, global outbound proxy get/set, and `routing-runtime status/start/stop`.
- [ ] Mark unsupported or remote-adapted routing subfeatures explicitly. If local-only actions such as local proxy scanning/testing or desktop-specific takeover state cannot apply remotely, hide or label them using the existing UI pattern instead of leaving controls that imply local behavior.

---

## File Structure

- Create `src-tauri/src/remote/mod.rs`: remote module entry point and shared exports.
- Create `src-tauri/src/remote/types.rs`: remote host profile, auth, health, command request/response, and capability types.
- Create `src-tauri/src/remote/store.rs`: local storage for remote profiles and non-secret health metadata.
- Create `src-tauri/src/remote/ssh.rs`: SSH command runner that executes remote helper commands and parses JSON.
- Create `src-tauri/src/commands/remote.rs`: Tauri command layer for the remote UI.
- Modify `src-tauri/src/commands/mod.rs`: export remote commands.
- Modify `src-tauri/src/lib.rs`: register remote commands.
- Create `src-tauri/src/bin/cc-switch-cli.rs`: Rust CLI helper entry point.
- Create `src-tauri/src/cli/mod.rs`: CLI dispatch module.
- Create `src-tauri/src/cli/types.rs`: CLI JSON envelope types.
- Create `src-tauri/src/cli/commands.rs`: helper commands such as `status`, `providers list`, and `providers switch`.
- Modify `src-tauri/Cargo.toml`: add binary target and CLI argument dependency if needed.
- Create `src/lib/api/remote.ts`: frontend Tauri API wrapper.
- Create `src/lib/query/remote.ts`: React Query keys and hooks.
- Create `src/components/remote/RemoteServersPage.tsx`: remote management page shell.
- Create `src/components/remote/RemoteHostDialog.tsx`: add/edit host profile dialog.
- Create `src/components/remote/RemoteHealthPanel.tsx`: helper installation and health display.
- Create `src/components/remote/RemoteProvidersPanel.tsx`: remote provider list and switch flow.
- Modify `src/App.tsx`: add a `remoteServers` view key, add it to `VALID_VIEWS`, add a render branch, add a header title branch, and add one toolbar/menu button whose click handler calls `setCurrentView("remoteServers")`.
- Add tests under `src-tauri/tests/remote_*` and `tests/components/Remote*.test.tsx`.

## Task 1: Remote Data Contract

**Files:**

- Create: `src-tauri/src/remote/mod.rs`
- Create: `src-tauri/src/remote/types.rs`

- [ ] **Step 1: Create remote module exports**

Create `src-tauri/src/remote/mod.rs`:

```rust
pub mod types;

pub use types::{
    RemoteAuthMethod, RemoteCapability, RemoteCommandError, RemoteCommandRequest,
    RemoteCommandResponse, RemoteHealth, RemoteHostProfile, RemotePlatform,
};
```

- [ ] **Step 2: Define serializable remote types**

Create `src-tauri/src/remote/types.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteHostProfile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: RemoteAuthMethod,
    pub helper_path: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum RemoteAuthMethod {
    SshAgent,
    KeyFile { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteHealth {
    pub reachable: bool,
    pub helper_installed: bool,
    pub helper_version: Option<String>,
    pub platform: Option<RemotePlatform>,
    pub capabilities: Vec<RemoteCapability>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RemotePlatform {
    Linux,
    Macos,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RemoteCapability {
    Providers,
    Mcp,
    Prompts,
    Skills,
    ImportExport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteCommandRequest {
    pub command: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteCommandResponse<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<RemoteCommandError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteCommandError {
    pub code: String,
    pub message: String,
}
```

- [ ] **Step 3: Register the module**

Modify `src-tauri/src/lib.rs` near the existing module declarations:

```rust
mod remote;
```

- [ ] **Step 4: Run Rust check for type validity**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: command completes successfully, or reports only pre-existing warnings.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/remote/mod.rs src-tauri/src/remote/types.rs
git commit -m "feat(remote): add remote management data contracts"
```

## Task 2: Local Remote Profile Store

**Files:**

- Create: `src-tauri/src/remote/store.rs`
- Modify: `src-tauri/src/remote/mod.rs`
- Test: `src-tauri/tests/remote_store.rs`

- [ ] **Step 1: Write store tests**

Create `src-tauri/tests/remote_store.rs`:

```rust
use cc_switch_lib::remote::{RemoteAuthMethod, RemoteHostProfile};

#[test]
fn remote_profile_keeps_secrets_out_of_local_state() {
    let profile = RemoteHostProfile {
        id: "prod".to_string(),
        name: "Production".to_string(),
        host: "10.0.0.10".to_string(),
        port: 22,
        username: "deploy".to_string(),
        auth_method: RemoteAuthMethod::KeyFile {
            path: "~/.ssh/id_ed25519".to_string(),
        },
        helper_path: "~/.local/bin/cc-switch".to_string(),
        created_at: 1,
        updated_at: 1,
    };

    let serialized = serde_json::to_string(&profile).expect("serialize profile");
    assert!(!serialized.contains("api_key"));
    assert!(!serialized.contains("providerSecret"));
    assert!(serialized.contains("id_ed25519"));
}
```

- [ ] **Step 2: Run test to verify current export gap**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test remote_store`

Expected: FAIL because `remote` is not exported from the crate.

- [ ] **Step 3: Export remote types from the library**

Modify `src-tauri/src/lib.rs` near the existing `pub use` block:

```rust
pub use remote;
```

If Rust rejects re-exporting a private module, change the module declaration to:

```rust
pub mod remote;
```

- [ ] **Step 4: Create profile store helpers**

Create `src-tauri/src/remote/store.rs`:

```rust
use crate::error::AppError;
use crate::remote::types::RemoteHostProfile;

pub fn validate_profile(profile: &RemoteHostProfile) -> Result<(), AppError> {
    if profile.id.trim().is_empty() {
        return Err(AppError::Message("Remote profile id is required".to_string()));
    }
    if profile.host.trim().is_empty() {
        return Err(AppError::Message("Remote host is required".to_string()));
    }
    if profile.username.trim().is_empty() {
        return Err(AppError::Message("Remote username is required".to_string()));
    }
    if profile.port == 0 {
        return Err(AppError::Message("Remote SSH port is required".to_string()));
    }
    Ok(())
}
```

Modify `src-tauri/src/remote/mod.rs`:

```rust
pub mod store;
pub mod types;

pub use store::validate_profile;
pub use types::{
    RemoteAuthMethod, RemoteCapability, RemoteCommandError, RemoteCommandRequest,
    RemoteCommandResponse, RemoteHealth, RemoteHostProfile, RemotePlatform,
};
```

- [ ] **Step 5: Run the store test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test remote_store`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/remote/mod.rs src-tauri/src/remote/store.rs src-tauri/tests/remote_store.rs
git commit -m "feat(remote): add remote profile validation"
```

## Task 3: Rust CLI Helper Skeleton

**Files:**

- Create: `src-tauri/src/bin/cc-switch-cli.rs`
- Create: `src-tauri/src/cli/mod.rs`
- Create: `src-tauri/src/cli/types.rs`
- Create: `src-tauri/src/cli/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`
- Test: `src-tauri/tests/cli_status.rs`

- [ ] **Step 1: Add CLI JSON envelope**

Create `src-tauri/src/cli/types.rs`:

```rust
use serde::Serialize;

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
```

- [ ] **Step 2: Add status command**

Create `src-tauri/src/cli/commands.rs`:

```rust
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusPayload {
    pub version: String,
    pub platform: String,
    pub capabilities: Vec<String>,
}

pub fn status_payload() -> StatusPayload {
    StatusPayload {
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        capabilities: vec![
            "providers".to_string(),
            "mcp".to_string(),
            "prompts".to_string(),
            "skills".to_string(),
            "import-export".to_string(),
        ],
    }
}
```

- [ ] **Step 3: Add CLI dispatcher**

Create `src-tauri/src/cli/mod.rs`:

```rust
pub mod commands;
pub mod types;

use serde_json::Value;

pub fn run(args: &[String]) -> Value {
    match args {
        [cmd] if cmd == "status" => serde_json::to_value(types::ok(commands::status_payload()))
            .expect("serialize status response"),
        _ => serde_json::to_value(types::err::<()>(
            "unsupported_command",
            "Supported command: status",
        ))
        .expect("serialize error response"),
    }
}
```

- [ ] **Step 4: Add binary entry point**

Create `src-tauri/src/bin/cc-switch-cli.rs`:

```rust
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let response = cc_switch_lib::cli::run(&args);
    println!(
        "{}",
        serde_json::to_string(&response).expect("serialize CLI response")
    );
}
```

Modify `src-tauri/src/lib.rs` near module declarations:

```rust
pub mod cli;
```

- [ ] **Step 5: Add binary declaration**

Modify `src-tauri/Cargo.toml`:

```toml
[[bin]]
name = "cc-switch-cli"
path = "src/bin/cc-switch-cli.rs"
```

- [ ] **Step 6: Run CLI manually**

Run: `cargo run --manifest-path src-tauri/Cargo.toml --bin cc-switch-cli -- status`

Expected stdout contains:

```json
{"ok":true
```

- [ ] **Step 7: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/lib.rs src-tauri/src/bin/cc-switch-cli.rs src-tauri/src/cli/mod.rs src-tauri/src/cli/types.rs src-tauri/src/cli/commands.rs
git commit -m "feat(remote): add Rust CLI helper skeleton"
```

## Task 4: SSH Remote Adapter

**Files:**

- Create: `src-tauri/src/remote/ssh.rs`
- Modify: `src-tauri/src/remote/mod.rs`
- Test: `src-tauri/tests/remote_ssh.rs`

- [ ] **Step 1: Write command builder tests**

Create `src-tauri/tests/remote_ssh.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify missing adapter**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test remote_ssh`

Expected: FAIL because `build_ssh_args` does not exist.

- [ ] **Step 3: Implement SSH argument builder**

Create `src-tauri/src/remote/ssh.rs`:

```rust
use crate::remote::types::{RemoteAuthMethod, RemoteHostProfile};

pub fn build_ssh_args(profile: &RemoteHostProfile, helper_args: &[String]) -> Vec<String> {
    let mut args = vec![
        "-p".to_string(),
        profile.port.to_string(),
        "-o".to_string(),
        "BatchMode=yes".to_string(),
    ];

    if let RemoteAuthMethod::KeyFile { path } = &profile.auth_method {
        args.push("-i".to_string());
        args.push(path.clone());
    }

    args.push(format!("{}@{}", profile.username, profile.host));

    let escaped_args = helper_args
        .iter()
        .map(|arg| shell_quote(arg))
        .collect::<Vec<_>>()
        .join(" ");
    args.push(format!("{} --json {}", profile.helper_path, escaped_args));
    args
}

fn shell_quote(value: &str) -> String {
    if value.chars().all(|c| c.is_ascii_alphanumeric() || "-_./:".contains(c)) {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}
```

Modify `src-tauri/src/remote/mod.rs`:

```rust
pub mod ssh;
pub mod store;
pub mod types;

pub use ssh::build_ssh_args;
pub use store::validate_profile;
pub use types::{
    RemoteAuthMethod, RemoteCapability, RemoteCommandError, RemoteCommandRequest,
    RemoteCommandResponse, RemoteHealth, RemoteHostProfile, RemotePlatform,
};
```

- [ ] **Step 4: Run adapter tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test remote_ssh`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/remote/mod.rs src-tauri/src/remote/ssh.rs src-tauri/tests/remote_ssh.rs
git commit -m "feat(remote): add SSH command adapter"
```

## Task 5: Tauri Remote Commands

**Files:**

- Create: `src-tauri/src/commands/remote.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add command functions**

Create `src-tauri/src/commands/remote.rs`:

```rust
use serde_json::Value;

use crate::remote::{build_ssh_args, validate_profile, RemoteHostProfile};

#[tauri::command]
pub async fn remote_validate_profile(profile: RemoteHostProfile) -> Result<bool, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn remote_build_status_command(profile: RemoteHostProfile) -> Result<Vec<String>, String> {
    validate_profile(&profile).map_err(|e| e.to_string())?;
    Ok(build_ssh_args(&profile, &["status".to_string()]))
}

#[tauri::command]
pub async fn remote_parse_helper_response(raw: String) -> Result<Value, String> {
    serde_json::from_str::<Value>(&raw).map_err(|e| format!("Invalid helper JSON: {e}"))
}
```

- [ ] **Step 2: Export commands**

Modify `src-tauri/src/commands/mod.rs`:

```rust
mod remote;
pub use remote::*;
```

- [ ] **Step 3: Register commands**

Modify `src-tauri/src/lib.rs` inside `tauri::generate_handler!`:

```rust
commands::remote_validate_profile,
commands::remote_build_status_command,
commands::remote_parse_helper_response,
```

- [ ] **Step 4: Run check**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/mod.rs src-tauri/src/commands/remote.rs src-tauri/src/lib.rs
git commit -m "feat(remote): expose remote management commands"
```

## Task 6: Frontend Remote API and Query Layer

**Files:**

- Create: `src/lib/api/remote.ts`
- Modify: `src/lib/api/index.ts`
- Create: `src/lib/query/remote.ts`

- [ ] **Step 1: Add API wrapper**

Create `src/lib/api/remote.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";

export type RemoteAuthMethod =
  | { type: "sshAgent" }
  | { type: "keyFile"; path: string };

export interface RemoteHostProfile {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  authMethod: RemoteAuthMethod;
  helperPath: string;
  createdAt: number;
  updatedAt: number;
}

export const remoteApi = {
  validateProfile(profile: RemoteHostProfile) {
    return invoke<boolean>("remote_validate_profile", { profile });
  },
  buildStatusCommand(profile: RemoteHostProfile) {
    return invoke<string[]>("remote_build_status_command", { profile });
  },
};
```

- [ ] **Step 2: Export API**

Modify `src/lib/api/index.ts`:

```ts
export * from "./remote";
```

- [ ] **Step 3: Add query helpers**

Create `src/lib/query/remote.ts`:

```ts
import { useMutation } from "@tanstack/react-query";
import { remoteApi, type RemoteHostProfile } from "@/lib/api/remote";

export const remoteQueryKeys = {
  all: ["remote"] as const,
  host: (id: string) => ["remote", "host", id] as const,
};

export function useValidateRemoteProfile() {
  return useMutation({
    mutationFn: (profile: RemoteHostProfile) =>
      remoteApi.validateProfile(profile),
  });
}
```

- [ ] **Step 4: Run typecheck**

Run: `pnpm typecheck`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/api/index.ts src/lib/api/remote.ts src/lib/query/remote.ts
git commit -m "feat(remote): add frontend remote API layer"
```

## Task 7: Remote UI Shell

**Files:**

- Create: `src/components/remote/RemoteServersPage.tsx`
- Create: `src/components/remote/RemoteHostDialog.tsx`
- Create: `src/components/remote/RemoteHealthPanel.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: Create host dialog skeleton**

Create `src/components/remote/RemoteHostDialog.tsx`:

```tsx
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";

export function RemoteHostDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Remote server</DialogTitle>
        </DialogHeader>
        <div className="grid gap-3">
          <Input placeholder="Name" />
          <Input placeholder="Host" />
          <Input placeholder="Username" />
          <Input placeholder="SSH key path" />
        </div>
        <DialogFooter>
          <Button type="button" onClick={() => onOpenChange(false)}>
            Save
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
```

- [ ] **Step 2: Create health panel skeleton**

Create `src/components/remote/RemoteHealthPanel.tsx`:

```tsx
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

export function RemoteHealthPanel() {
  return (
    <section className="flex items-center justify-between border-b px-4 py-3">
      <div className="flex items-center gap-2">
        <span className="text-sm font-medium">Helper status</span>
        <Badge variant="outline">Not checked</Badge>
      </div>
      <Button size="sm" variant="outline">
        Check
      </Button>
    </section>
  );
}
```

- [ ] **Step 3: Create page shell**

Create `src/components/remote/RemoteServersPage.tsx`:

```tsx
import { useState } from "react";
import { Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { RemoteHealthPanel } from "./RemoteHealthPanel";
import { RemoteHostDialog } from "./RemoteHostDialog";

export function RemoteServersPage() {
  const [dialogOpen, setDialogOpen] = useState(false);

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center justify-between border-b px-4 py-3">
        <h1 className="text-base font-semibold">Remote Servers</h1>
        <Button size="sm" onClick={() => setDialogOpen(true)}>
          <Plus className="mr-2 h-4 w-4" />
          Add
        </Button>
      </header>
      <RemoteHealthPanel />
      <div className="p-4 text-sm text-muted-foreground">
        Select or add a server to manage its independent CC Switch state.
      </div>
      <RemoteHostDialog open={dialogOpen} onOpenChange={setDialogOpen} />
    </div>
  );
}
```

- [ ] **Step 4: Wire the page into navigation**

Modify `src/App.tsx` imports:

```tsx
import { Server } from "lucide-react";
import { RemoteServersPage } from "@/components/remote/RemoteServersPage";
```

Extend the `View` union:

```tsx
  | "remoteServers"
```

Add `"remoteServers"` to `VALID_VIEWS`:

```tsx
  "remoteServers",
```

Add a `renderContent` branch before the default provider view:

```tsx
        case "remoteServers":
          return <RemoteServersPage />;
```

Add a header title branch near the other `currentView` title checks:

```tsx
{
  currentView === "remoteServers" && "Remote Servers";
}
```

Add one action button in the header action area next to other global tools:

```tsx
<Button
  variant="ghost"
  size="sm"
  onClick={() => setCurrentView("remoteServers")}
  title="Remote Servers"
>
  <Server className="h-4 w-4" />
</Button>
```

- [ ] **Step 5: Run typecheck**

Run: `pnpm typecheck`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/App.tsx src/components/remote/RemoteServersPage.tsx src/components/remote/RemoteHostDialog.tsx src/components/remote/RemoteHealthPanel.tsx
git commit -m "feat(remote): add remote server UI shell"
```

## Task 8: Provider Parity MVP

**Files:**

- Modify: `src-tauri/src/cli/commands.rs`
- Modify: `src-tauri/src/cli/mod.rs`
- Modify: `src-tauri/src/remote/ssh.rs`
- Create: `src/components/remote/RemoteProvidersPanel.tsx`
- Modify: `src/components/remote/RemoteServersPage.tsx`

- [ ] **Step 1: Add CLI provider list command**

Modify `src-tauri/src/cli/commands.rs`:

```rust
use crate::{AppState, AppType, Database, ProviderService};
use std::sync::Arc;

pub fn list_providers(app: AppType) -> Result<serde_json::Value, String> {
    let db_path = crate::config::get_app_config_dir().join("cc-switch.db");
    let db = Arc::new(Database::new(&db_path).map_err(|e| e.to_string())?);
    let state = AppState::new(db);
    let providers = ProviderService::list(&state, app).map_err(|e| e.to_string())?;
    serde_json::to_value(providers).map_err(|e| e.to_string())
}
```

- [ ] **Step 2: Add dispatcher branch**

Modify `src-tauri/src/cli/mod.rs`:

```rust
["providers", "list", app] => match app.parse() {
    Ok(app_type) => match commands::list_providers(app_type) {
        Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize providers"),
        Err(message) => serde_json::to_value(types::err::<()>("providers_list_failed", message))
            .expect("serialize provider error"),
    },
    Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
        .expect("serialize invalid app error"),
},
```

- [ ] **Step 3: Run CLI provider list**

Run: `cargo run --manifest-path src-tauri/Cargo.toml --bin cc-switch-cli -- providers list claude`

Expected: JSON response with `ok` and either provider data or a clear database error.

- [ ] **Step 4: Add remote provider UI panel**

Create `src/components/remote/RemoteProvidersPanel.tsx`:

```tsx
import { Button } from "@/components/ui/button";

export function RemoteProvidersPanel() {
  return (
    <section className="flex flex-col gap-3 p-4">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-semibold">Remote providers</h2>
        <Button size="sm" variant="outline">
          Refresh
        </Button>
      </div>
      <div className="rounded-md border p-3 text-sm text-muted-foreground">
        Provider list is loaded from the selected remote server, not local
        state.
      </div>
    </section>
  );
}
```

- [ ] **Step 5: Render the panel**

Modify `src/components/remote/RemoteServersPage.tsx`:

```tsx
import { RemoteProvidersPanel } from "./RemoteProvidersPanel";
```

Place it after `<RemoteHealthPanel />`:

```tsx
<RemoteProvidersPanel />
```

- [ ] **Step 6: Run checks**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

Run: `pnpm typecheck`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/cli/commands.rs src-tauri/src/cli/mod.rs src/components/remote/RemoteProvidersPanel.tsx src/components/remote/RemoteServersPage.tsx
git commit -m "feat(remote): add remote provider parity foundation"
```

## Task 9: Distribution Strategy

**Files:**

- Modify: `.github/workflows/release.yml`
- Create: `scripts/install-remote-helper.sh`
- Create: `docs/guides/remote-server-management-zh.md`

- [ ] **Step 1: Add helper binary collection to release workflow**

Modify `.github/workflows/release.yml` after the platform-specific `Prepare macOS Assets`, `Prepare Windows Assets`, and `Prepare Linux Assets` steps and before `List prepared assets`:

```yaml
- name: Prepare Remote Helper CLI Asset
  shell: bash
  run: |
    set -euxo pipefail
    mkdir -p release-assets
    VERSION="${GITHUB_REF_NAME}"
    case "${{ runner.os }}" in
      macOS)
        HELPER_PATH="src-tauri/target/universal-apple-darwin/release/cc-switch-cli"
        HELPER_OS="macOS"
        HELPER_ARCH="universal"
        ;;
      Linux)
        HELPER_PATH="src-tauri/target/release/cc-switch-cli"
        HELPER_OS="Linux"
        HELPER_ARCH="${{ matrix.arch || 'x86_64' }}"
        ;;
      Windows)
        HELPER_PATH="src-tauri/target/release/cc-switch-cli.exe"
        HELPER_OS="Windows"
        HELPER_ARCH="x86_64"
        ;;
    esac
    if [ ! -f "$HELPER_PATH" ]; then
      echo "Remote helper CLI not found at $HELPER_PATH" >&2
      exit 1
    fi
    ASSET="cc-switch-cli-${VERSION}-${HELPER_OS}-${HELPER_ARCH}"
    if [ "${{ runner.os }}" = "Windows" ]; then
      cp "$HELPER_PATH" "release-assets/${ASSET}.exe"
    else
      cp "$HELPER_PATH" "release-assets/${ASSET}"
      chmod +x "release-assets/${ASSET}"
    fi
```

- [ ] **Step 2: Add helper installer script**

Create `scripts/install-remote-helper.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

REPO="${CC_SWITCH_REPO:-xiaoY233/cc-switch}"
VERSION="${1:-latest}"
BIN_DIR="${CC_SWITCH_BIN_DIR:-$HOME/.local/bin}"
OS="$(uname -s)"
ARCH="$(uname -m)"
mkdir -p "$BIN_DIR"

case "$OS" in
  Linux) ASSET_OS="Linux" ;;
  Darwin) ASSET_OS="macOS" ;;
  *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64) ASSET_ARCH="x86_64" ;;
  arm64|aarch64) ASSET_ARCH="arm64" ;;
  *) echo "Unsupported arch: $ARCH" >&2; exit 1 ;;
esac

if [ "$ASSET_OS" = "macOS" ]; then
  ASSET_ARCH="universal"
fi

if [ "$VERSION" = "latest" ]; then
  API_URL="https://api.github.com/repos/$REPO/releases/latest"
else
  API_URL="https://api.github.com/repos/$REPO/releases/tags/$VERSION"
fi

ASSET_NAME="cc-switch-cli-${VERSION}-${ASSET_OS}-${ASSET_ARCH}"
if [ "$VERSION" = "latest" ]; then
  ASSET_NAME_PATTERN="cc-switch-cli-.*-${ASSET_OS}-${ASSET_ARCH}$"
else
  ASSET_NAME_PATTERN="^${ASSET_NAME}$"
fi

DOWNLOAD_URL="$(
  curl -fsSL "$API_URL" |
    grep -E '"browser_download_url":' |
    sed -E 's/.*"browser_download_url": "([^"]+)".*/\1/' |
    grep -E "$ASSET_NAME_PATTERN" |
    head -1
)"

if [ -z "$DOWNLOAD_URL" ]; then
  echo "No helper asset found for $ASSET_OS/$ASSET_ARCH from $API_URL" >&2
  exit 1
fi

curl -fL "$DOWNLOAD_URL" -o "$BIN_DIR/cc-switch"
chmod +x "$BIN_DIR/cc-switch"
"$BIN_DIR/cc-switch" status
```

- [ ] **Step 3: Document installation model**

Create `docs/guides/remote-server-management-zh.md`:

```markdown
# 远程服务器管理

远程服务器管理使用本机桌面端作为控制台，通过 SSH 调用远端的 Rust CLI helper。远端服务器保留自己的 CC Switch 数据目录、数据库和 AI CLI 配置文件；本机只保存连接信息和健康状态缓存。

## 分发模型

- Rust CLI helper 是真实执行层。
- NPM 包如后续提供，只作为安装器或薄包装，不承载 provider、MCP、prompts、skills 的业务逻辑。
- 远端 Linux/macOS 服务器不要求安装桌面环境。

## 独立性

本地和远端配置不会自动合并。任何导入、导出或复制都需要用户显式触发，并在执行前展示目标和影响。
```

- [ ] **Step 4: Run formatting/checks**

Run: `git diff --check`

Expected: no whitespace errors.

- [ ] **Step 5: Commit**

```bash
git add scripts/install-remote-helper.sh docs/guides/remote-server-management-zh.md .github/workflows
git commit -m "docs(remote): document helper distribution strategy"
```

## Self-Review

- Spec coverage: This plan covers local/remote independence, Rust helper CLI, optional NPM wrapper boundary, SSH JSON protocol, remote UI shell, provider MVP, and distribution documentation.
- Placeholder scan: The plan intentionally avoids open-ended implementation placeholders in code-bearing steps. The release workflow step requires identifying the active workflow before editing because this repository's workflow filename must be read from disk at execution time.
- Type consistency: Remote profile fields use camelCase in JSON and Rust snake_case internally. CLI envelopes use `{ ok, data, error }` consistently across status and provider commands.
