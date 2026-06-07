pub const REMOTE_HELPER_CAPABILITIES: &[&str] = &[
    "providers",
    "openclaw",
    "mcp",
    "prompts",
    "skills",
    "sessions",
    "hermes-memory",
    "import-export",
    "tools",
    "settings",
    "plugin",
    "session",
];

pub const REMOTE_HELPER_REQUIRED_CAPABILITIES: &[&str] = REMOTE_HELPER_CAPABILITIES;

pub fn remote_helper_capabilities() -> Vec<String> {
    REMOTE_HELPER_CAPABILITIES
        .iter()
        .map(|capability| (*capability).to_string())
        .collect()
}
