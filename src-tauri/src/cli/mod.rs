pub mod commands;
pub mod types;

use serde_json::Value;

pub fn run(args: &[String]) -> Value {
    match args {
        [cmd] if cmd == "status" => serde_json::to_value(types::ok(commands::status_payload()))
            .expect("serialize status response"),
        _ => serde_json::to_value(types::err::<()>(
            "unsupported_command",
            "Supported command: status",
        ))
        .expect("serialize error response"),
    }
}
