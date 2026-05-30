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
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_./:".contains(c))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}
