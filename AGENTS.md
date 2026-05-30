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

