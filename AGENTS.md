# Project Rules

## Remote Server Management

- Treat local and remote management as separate targets. Local state remains in the local CC Switch data directory and must not become the source of truth for a remote server.
- A remote server owns its own CC Switch state, database, tool config files, MCP entries, prompts, and skills. The desktop app stores only remote connection profiles and cached health metadata.
- Build remote execution around a Rust CLI helper running on the remote host. The helper should reuse the Rust service/core logic that backs the Tauri app instead of reimplementing provider, MCP, prompt, or skill logic in TypeScript.
- Keep any NPM package as an optional installer or thin wrapper for downloading and invoking the Rust binary. Do not put durable business logic in an NPM implementation.
- Communicate with remote hosts through SSH commands that return stable JSON. Avoid editing remote files directly from the GUI through ad hoc SFTP path logic.
- Keep remote feature code isolated behind a remote adapter and remote UI shell so upstream local app changes remain easy to merge.
- Mark feature parity explicitly. Provider switching, MCP, prompts, skills, import/export, and health checks should aim for parity; tray controls, desktop deeplinks, browser OAuth flows, local terminal launch, and local proxy takeover are local-only unless a remote-safe equivalent is designed.
- Do not silently merge local and remote configuration. Any import, export, or copy between targets must be explicit, previewable, and reversible.
- Do not persist remote API keys or provider secrets in the local desktop database. Secrets should stay on the remote host unless the user explicitly performs an export or migration workflow.
- Prefer small, upstream-friendly changes: add new remote modules, commands, and UI entry points before modifying existing provider, MCP, prompt, skill, or settings flows.
- Remote helper commands must be thin adapters over shared Rust core/service logic. Do not add helper-only implementations for tool version checks, install/update commands, provider import/switch logic, MCP import aggregation, prompt management, skill management, sessions, Hermes memory, or OpenClaw settings.
- Tool environment checks and lifecycle actions must share the same Rust implementation for local and remote targets. If local behavior changes for PATH search, version classification, latest-version lookup, installer fallback, or update command anchoring, the remote helper must pick it up through the shared implementation rather than copying the change.
- Secret redaction for remote provider payloads must stay centralized. If provider schemas or secret-field detection rules change, update the shared redaction/restore path and its tests rather than adding ad hoc redaction in remote commands.
- Remote UI should reuse local page components for equivalent workflows. Add remote-specific shells only for target selection, SSH/helper status, capability gating, and host management; do not rebuild provider, MCP, prompt, skill, environment, or settings controls with separate visual patterns.
- Every new remote capability must declare whether it is parity, remote-adapted, or unsupported, and include focused tests for local/remote adapter behavior. A helper capability should not be advertised until the corresponding command is implemented and covered.
