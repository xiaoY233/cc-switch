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

    let mut command = vec![
        shell_quote_helper_path(&profile.helper_path),
        "--json".to_string(),
    ];
    command.extend(helper_args.iter().map(|arg| shell_quote(arg)));
    args.push(command.join(" "));
    args
}

fn shell_quote_helper_path(value: &str) -> String {
    if is_safe_unquoted_helper_path(value) {
        return value.to_string();
    }
    shell_quote(value)
}

fn is_safe_unquoted_helper_path(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:~".contains(c))
}

fn shell_quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:".contains(c))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}
