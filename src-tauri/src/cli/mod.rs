pub mod commands;
pub mod types;

use serde_json::Value;

pub fn run(args: &[String]) -> Value {
    match args {
        [cmd] if cmd == "status" => serde_json::to_value(types::ok(commands::status_payload()))
            .expect("serialize status response"),
        [group, cmd, app] if group == "providers" && cmd == "list" => match app.parse() {
            Ok(app_type) => match commands::list_providers(app_type) {
                Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize providers"),
                Err(message) => serde_json::to_value(types::err::<()>(
                    "providers_list_failed",
                    message,
                ))
                .expect("serialize provider error"),
            },
            Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                .expect("serialize invalid app error"),
        },
        _ => serde_json::to_value(types::err::<()>(
            "unsupported_command",
            "Supported commands: status, providers list <app>",
        ))
        .expect("serialize error response"),
    }
}
