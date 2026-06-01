#![cfg_attr(not(feature = "desktop"), allow(unused))]

#[path = "../app_config.rs"]
#[cfg(not(feature = "desktop"))]
mod app_config;
#[path = "../claude_desktop_config.rs"]
#[cfg(not(feature = "desktop"))]
mod claude_desktop_config;
#[path = "../claude_mcp.rs"]
#[cfg(not(feature = "desktop"))]
mod claude_mcp;
#[path = "../cli/mod.rs"]
#[cfg(not(feature = "desktop"))]
mod cli;
#[path = "../codex_config.rs"]
#[cfg(not(feature = "desktop"))]
mod codex_config;
#[path = "../config.rs"]
#[cfg(not(feature = "desktop"))]
mod config;
#[path = "../database/mod.rs"]
#[cfg(not(feature = "desktop"))]
mod database;
#[path = "../error.rs"]
#[cfg(not(feature = "desktop"))]
mod error;
#[path = "../gemini_config.rs"]
#[cfg(not(feature = "desktop"))]
mod gemini_config;
#[path = "../gemini_mcp.rs"]
#[cfg(not(feature = "desktop"))]
mod gemini_mcp;
#[path = "../hermes_config.rs"]
#[cfg(not(feature = "desktop"))]
pub mod hermes_config;
#[path = "../mcp/mod.rs"]
#[cfg(not(feature = "desktop"))]
mod mcp;
#[path = "../openclaw_config.rs"]
#[cfg(not(feature = "desktop"))]
mod openclaw_config;
#[path = "../opencode_config.rs"]
#[cfg(not(feature = "desktop"))]
mod opencode_config;
#[path = "../prompt.rs"]
#[cfg(not(feature = "desktop"))]
mod prompt;
#[path = "../prompt_files.rs"]
#[cfg(not(feature = "desktop"))]
mod prompt_files;
#[path = "../provider.rs"]
#[cfg(not(feature = "desktop"))]
mod provider;
#[path = "../provider_defaults.rs"]
#[cfg(not(feature = "desktop"))]
mod provider_defaults;
#[path = "../proxy/mod.rs"]
#[cfg(not(feature = "desktop"))]
mod proxy;
#[path = "../services/mod.rs"]
#[cfg(not(feature = "desktop"))]
mod services;
#[path = "../session_manager/mod.rs"]
#[cfg(not(feature = "desktop"))]
mod session_manager;
#[path = "../settings.rs"]
#[cfg(not(feature = "desktop"))]
mod settings;
#[path = "../store.rs"]
#[cfg(not(feature = "desktop"))]
mod store;
#[path = "../usage_script.rs"]
#[cfg(not(feature = "desktop"))]
mod usage_script;

#[cfg(not(feature = "desktop"))]
mod app_store {
    pub fn get_app_config_dir_override() -> Option<std::path::PathBuf> {
        None
    }
}

#[cfg(not(feature = "desktop"))]
mod usage_events {
    pub fn notify_log_recorded() {}
}

#[cfg(not(feature = "desktop"))]
pub use app_config::{AppType, InstalledSkill, McpApps, McpServer, MultiAppConfig, SkillApps};
#[cfg(not(feature = "desktop"))]
pub use codex_config::{get_codex_auth_path, get_codex_config_path, write_codex_live_atomic};
#[cfg(not(feature = "desktop"))]
pub use config::{get_claude_mcp_path, get_claude_settings_path, read_json_file};
#[cfg(not(feature = "desktop"))]
pub use database::Database;
#[cfg(not(feature = "desktop"))]
pub use error::AppError;
#[cfg(not(feature = "desktop"))]
pub use mcp::{
    import_from_claude, import_from_codex, import_from_gemini, remove_server_from_claude,
    remove_server_from_codex, remove_server_from_gemini, sync_enabled_to_claude,
    sync_enabled_to_codex, sync_enabled_to_gemini, sync_single_server_to_claude,
    sync_single_server_to_codex, sync_single_server_to_gemini,
};
#[cfg(not(feature = "desktop"))]
pub use provider::{Provider, ProviderMeta};
#[cfg(not(feature = "desktop"))]
pub use services::{
    skill::{migrate_skills_to_ssot, ImportSkillSelection},
    ConfigService, EndpointLatency, McpService, PromptService, ProviderService, ProxyService,
    SkillService, SpeedtestService,
};
#[cfg(not(feature = "desktop"))]
pub use settings::{update_settings, AppSettings};
#[cfg(not(feature = "desktop"))]
pub use store::AppState;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(feature = "desktop")]
    let response = cc_switch_lib::cli::run(&args);
    #[cfg(not(feature = "desktop"))]
    let response = cli::run(&args);
    println!(
        "{}",
        serde_json::to_string(&response).expect("serialize CLI response")
    );
}
