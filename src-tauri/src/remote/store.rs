use crate::config::get_app_config_dir;
use crate::error::AppError;
use crate::remote::types::{RemoteAuthMethod, RemoteHostProfile};
use std::fs;
use std::path::{Path, PathBuf};

const REMOTE_PROFILES_FILENAME: &str = "remote-hosts.json";

pub fn validate_profile(profile: &RemoteHostProfile) -> Result<(), AppError> {
    if profile.id.trim().is_empty() {
        return Err(AppError::Message(
            "Remote profile id is required".to_string(),
        ));
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
    match &profile.auth_method {
        RemoteAuthMethod::KeyFile { path } if path.trim().is_empty() => {
            return Err(AppError::Message(
                "Remote SSH key path is required".to_string(),
            ));
        }
        _ => {}
    }
    Ok(())
}

pub fn profiles_path() -> PathBuf {
    get_app_config_dir().join(REMOTE_PROFILES_FILENAME)
}

pub fn load_profiles() -> Result<Vec<RemoteHostProfile>, AppError> {
    load_profiles_from_path(&profiles_path())
}

pub fn save_profiles(profiles: &[RemoteHostProfile]) -> Result<(), AppError> {
    save_profiles_to_path(&profiles_path(), profiles)
}

pub fn upsert_profile(profile: RemoteHostProfile) -> Result<RemoteHostProfile, AppError> {
    validate_profile(&profile)?;
    let mut profiles = load_profiles()?;
    if let Some(existing) = profiles.iter_mut().find(|item| item.id == profile.id) {
        *existing = profile.clone();
    } else {
        profiles.insert(0, profile.clone());
    }
    save_profiles(&profiles)?;
    Ok(profile)
}

pub fn delete_profile(id: &str) -> Result<bool, AppError> {
    let mut profiles = load_profiles()?;
    let before = profiles.len();
    profiles.retain(|profile| profile.id != id);
    if profiles.len() == before {
        return Ok(false);
    }
    save_profiles(&profiles)?;
    Ok(true)
}

pub fn load_profiles_from_path(path: &Path) -> Result<Vec<RemoteHostProfile>, AppError> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(path)
        .map_err(|e| AppError::Message(format!("Failed to read remote host profiles: {e}")))?;
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }

    let profiles: Vec<RemoteHostProfile> = serde_json::from_str(&raw)
        .map_err(|e| AppError::Message(format!("Failed to parse remote host profiles: {e}")))?;
    for profile in &profiles {
        validate_profile(profile)?;
    }
    Ok(profiles)
}

pub fn save_profiles_to_path(path: &Path, profiles: &[RemoteHostProfile]) -> Result<(), AppError> {
    for profile in profiles {
        validate_profile(profile)?;
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            AppError::Message(format!("Failed to create remote profile directory: {e}"))
        })?;
    }

    let raw = serde_json::to_string_pretty(profiles)
        .map_err(|e| AppError::Message(format!("Failed to serialize remote host profiles: {e}")))?;
    fs::write(path, raw)
        .map_err(|e| AppError::Message(format!("Failed to write remote host profiles: {e}")))
}
