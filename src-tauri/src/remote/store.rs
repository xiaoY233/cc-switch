use crate::config::get_app_config_dir;
use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::remote::types::{RemoteAuthMethod, RemoteConnectionSecret, RemoteHostProfile};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rusqlite::params;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const REMOTE_PROFILES_FILENAME: &str = "remote-hosts.json";
const REMOTE_SECRET_PREFIX: &str = "remote.profile.password.";
const REMOTE_SECRET_VALUE_PREFIX: &str = "v1:";

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
    delete_profile_secret(id)?;
    Ok(true)
}

pub fn save_profile_secret(
    profile_id: &str,
    secret: &RemoteConnectionSecret,
) -> Result<(), AppError> {
    let Some(password) = secret.password.as_deref().filter(|value| !value.is_empty()) else {
        return delete_profile_secret(profile_id);
    };
    let db = Database::init()?;
    db.set_setting(
        &profile_secret_key(profile_id),
        &encode_secret_value(profile_id, password),
    )
}

pub fn load_profile_secret(profile_id: &str) -> Result<RemoteConnectionSecret, AppError> {
    let db = Database::init()?;
    let password = db
        .get_setting(&profile_secret_key(profile_id))?
        .and_then(|value| decode_secret_value(profile_id, &value).ok());
    Ok(RemoteConnectionSecret { password })
}

pub fn delete_profile_secret(profile_id: &str) -> Result<(), AppError> {
    let db = Database::init()?;
    let conn = lock_conn!(db.conn);
    conn.execute(
        "DELETE FROM settings WHERE key = ?1",
        params![profile_secret_key(profile_id)],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
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

fn profile_secret_key(profile_id: &str) -> String {
    format!("{REMOTE_SECRET_PREFIX}{profile_id}")
}

fn encode_secret_value(profile_id: &str, password: &str) -> String {
    let encrypted = xor_with_profile_key(profile_id, password.as_bytes());
    format!("{REMOTE_SECRET_VALUE_PREFIX}{}", STANDARD.encode(encrypted))
}

fn decode_secret_value(profile_id: &str, encoded: &str) -> Result<String, AppError> {
    let payload = encoded
        .strip_prefix(REMOTE_SECRET_VALUE_PREFIX)
        .ok_or_else(|| AppError::Message("Unsupported remote secret format".to_string()))?;
    let bytes = STANDARD
        .decode(payload)
        .map_err(|e| AppError::Message(format!("Invalid remote secret encoding: {e}")))?;
    let decrypted = xor_with_profile_key(profile_id, &bytes);
    String::from_utf8(decrypted)
        .map_err(|e| AppError::Message(format!("Invalid remote secret UTF-8: {e}")))
}

fn xor_with_profile_key(profile_id: &str, input: &[u8]) -> Vec<u8> {
    let key_seed = format!("{}::{profile_id}", get_app_config_dir().display());
    let digest = Sha256::digest(key_seed.as_bytes());
    input
        .iter()
        .enumerate()
        .map(|(index, byte)| byte ^ digest[index % digest.len()])
        .collect()
}
