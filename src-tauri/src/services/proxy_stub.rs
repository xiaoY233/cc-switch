use crate::app_config::AppType;
use crate::database::Database;
use crate::provider::Provider;
use std::sync::Arc;

#[derive(Clone)]
pub struct ProxyService {
    _db: Arc<Database>,
}

pub struct ProxySwitchGuard;

impl ProxyService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { _db: db }
    }

    pub async fn is_running(&self) -> bool {
        false
    }

    pub fn detect_takeover_in_live_config_for_app(&self, _app_type: &AppType) -> bool {
        false
    }

    pub async fn update_live_backup_from_provider(
        &self,
        _app: &str,
        _provider: &Provider,
    ) -> Result<(), String> {
        Ok(())
    }

    pub async fn sync_claude_live_from_provider_while_proxy_active(
        &self,
        _provider: &Provider,
    ) -> Result<(), String> {
        Ok(())
    }

    pub async fn hot_switch_provider(&self, _app: &str, _id: &str) -> Result<(), String> {
        Ok(())
    }

    pub async fn lock_switch_for_app(&self, _app: &str) -> ProxySwitchGuard {
        ProxySwitchGuard
    }

    pub async fn hot_switch_provider_inner(&self, _app: &str, _id: &str) -> Result<(), String> {
        Ok(())
    }
}
