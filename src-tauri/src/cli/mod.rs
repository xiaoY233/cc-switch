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
        [group, cmd, app] if group == "providers" && cmd == "current" => match app.parse() {
            Ok(app_type) => match commands::current_provider(app_type) {
                Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize current"),
                Err(message) => serde_json::to_value(types::err::<()>(
                    "providers_current_failed",
                    message,
                ))
                .expect("serialize provider current error"),
            },
            Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                .expect("serialize invalid app error"),
        },
        [group, cmd, app, id] if group == "providers" && cmd == "switch" => match app.parse() {
            Ok(app_type) => match commands::switch_provider(app_type, id) {
                Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize switch"),
                Err(message) => serde_json::to_value(types::err::<()>(
                    "providers_switch_failed",
                    message,
                ))
                .expect("serialize provider switch error"),
            },
            Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                .expect("serialize invalid app error"),
        },
        [group, cmd, app, provider_json, add_to_live]
            if group == "providers" && cmd == "add" =>
        {
            match app.parse() {
                Ok(app_type) => {
                    let add_to_live = add_to_live != "false";
                    match commands::add_provider(app_type, provider_json, add_to_live) {
                        Ok(value) => {
                            serde_json::to_value(types::ok(value)).expect("serialize add")
                        }
                        Err(message) => serde_json::to_value(types::err::<()>(
                            "providers_add_failed",
                            message,
                        ))
                        .expect("serialize provider add error"),
                    }
                }
                Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                    .expect("serialize invalid app error"),
            }
        }
        [group, cmd, app, provider_json, original_id]
            if group == "providers" && cmd == "update" =>
        {
            match app.parse() {
                Ok(app_type) => {
                    let original_id = if original_id == "-" {
                        None
                    } else {
                        Some(original_id.as_str())
                    };
                    match commands::update_provider(app_type, provider_json, original_id) {
                        Ok(value) => {
                            serde_json::to_value(types::ok(value)).expect("serialize update")
                        }
                        Err(message) => serde_json::to_value(types::err::<()>(
                            "providers_update_failed",
                            message,
                        ))
                        .expect("serialize provider update error"),
                    }
                }
                Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                    .expect("serialize invalid app error"),
            }
        }
        [group, cmd, app, id] if group == "providers" && cmd == "delete" => match app.parse() {
            Ok(app_type) => match commands::delete_provider(app_type, id) {
                Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize delete"),
                Err(message) => serde_json::to_value(types::err::<()>(
                    "providers_delete_failed",
                    message,
                ))
                .expect("serialize provider delete error"),
            },
            Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                .expect("serialize invalid app error"),
        },
        _ => serde_json::to_value(types::err::<()>(
            "unsupported_command",
            "Supported commands: status, providers list|current <app>, providers switch|delete <app> <id>, providers add <app> <provider-json> <add-to-live>, providers update <app> <provider-json> <original-id|->",
        ))
        .expect("serialize error response"),
    }
}
