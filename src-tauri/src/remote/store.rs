use crate::error::AppError;
use crate::remote::types::RemoteHostProfile;

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
    Ok(())
}
