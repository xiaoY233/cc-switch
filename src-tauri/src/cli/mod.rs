pub mod commands;
pub mod types;

use serde_json::Value;

pub fn run(args: &[String]) -> Value {
    let args = normalize_args(args);
    run_normalized(&args)
}

fn normalize_args(args: &[String]) -> Vec<String> {
    args.iter()
        .filter(|arg| arg.as_str() != "--json")
        .cloned()
        .collect()
}

fn run_normalized(args: &[String]) -> Value {
    match args {
        [cmd] if cmd == "status" => serde_json::to_value(types::ok(commands::status_payload()))
            .expect("serialize status response"),
        [group, cmd, app] if group == "providers" && cmd == "list" => match app.parse() {
            Ok(app_type) => match commands::list_providers(app_type) {
                Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize providers"),
                Err(message) => {
                    serde_json::to_value(types::err::<()>("providers_list_failed", message))
                        .expect("serialize provider error")
                }
            },
            Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                .expect("serialize invalid app error"),
        },
        [group, cmd, app] if group == "providers" && cmd == "current" => match app.parse() {
            Ok(app_type) => match commands::current_provider(app_type) {
                Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize current"),
                Err(message) => {
                    serde_json::to_value(types::err::<()>("providers_current_failed", message))
                        .expect("serialize provider current error")
                }
            },
            Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                .expect("serialize invalid app error"),
        },
        [group, cmd, app, id] if group == "providers" && cmd == "switch" => match app.parse() {
            Ok(app_type) => match commands::switch_provider(app_type, id) {
                Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize switch"),
                Err(message) => {
                    serde_json::to_value(types::err::<()>("providers_switch_failed", message))
                        .expect("serialize provider switch error")
                }
            },
            Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                .expect("serialize invalid app error"),
        },
        [group, cmd, app, provider_json, add_to_live] if group == "providers" && cmd == "add" => {
            match app.parse() {
                Ok(app_type) => {
                    let add_to_live = add_to_live != "false";
                    match commands::add_provider(app_type, provider_json, add_to_live) {
                        Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize add"),
                        Err(message) => {
                            serde_json::to_value(types::err::<()>("providers_add_failed", message))
                                .expect("serialize provider add error")
                        }
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
                Err(message) => {
                    serde_json::to_value(types::err::<()>("providers_delete_failed", message))
                        .expect("serialize provider delete error")
                }
            },
            Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                .expect("serialize invalid app error"),
        },
        [group, cmd] if group == "openclaw" && cmd == "get-default-model" => {
            match commands::get_openclaw_default_model() {
                Ok(value) => serde_json::to_value(types::ok(value))
                    .expect("serialize openclaw default model"),
                Err(message) => serde_json::to_value(types::err::<()>(
                    "openclaw_get_default_model_failed",
                    message,
                ))
                .expect("serialize openclaw default model error"),
            }
        }
        [group, cmd, model_json] if group == "openclaw" && cmd == "set-default-model" => {
            match commands::set_openclaw_default_model(model_json) {
                Ok(value) => serde_json::to_value(types::ok(value))
                    .expect("serialize openclaw default model"),
                Err(message) => serde_json::to_value(types::err::<()>(
                    "openclaw_set_default_model_failed",
                    message,
                ))
                .expect("serialize openclaw default model error"),
            }
        }
        [group, cmd] if group == "mcp" && cmd == "list" => match commands::list_mcp_servers() {
            Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize mcp list"),
            Err(message) => serde_json::to_value(types::err::<()>("mcp_list_failed", message))
                .expect("serialize mcp list error"),
        },
        [group, cmd, server_json] if group == "mcp" && cmd == "upsert" => {
            match commands::upsert_mcp_server(server_json) {
                Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize mcp upsert"),
                Err(message) => {
                    serde_json::to_value(types::err::<()>("mcp_upsert_failed", message))
                        .expect("serialize mcp upsert error")
                }
            }
        }
        [group, cmd, id] if group == "mcp" && cmd == "delete" => {
            match commands::delete_mcp_server(id) {
                Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize mcp delete"),
                Err(message) => {
                    serde_json::to_value(types::err::<()>("mcp_delete_failed", message))
                        .expect("serialize mcp delete error")
                }
            }
        }
        [group, cmd, server_id, app, enabled] if group == "mcp" && cmd == "toggle" => {
            match app.parse() {
                Ok(app_type) => {
                    let enabled = enabled == "true";
                    match commands::toggle_mcp_app(server_id, app_type, enabled) {
                        Ok(value) => {
                            serde_json::to_value(types::ok(value)).expect("serialize mcp toggle")
                        }
                        Err(message) => {
                            serde_json::to_value(types::err::<()>("mcp_toggle_failed", message))
                                .expect("serialize mcp toggle error")
                        }
                    }
                }
                Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                    .expect("serialize invalid app error"),
            }
        }
        [group, cmd] if group == "mcp" && cmd == "import" => {
            match commands::import_mcp_from_apps() {
                Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize mcp import"),
                Err(message) => {
                    serde_json::to_value(types::err::<()>("mcp_import_failed", message))
                        .expect("serialize mcp import error")
                }
            }
        }
        [group, cmd, app] if group == "prompts" && cmd == "list" => match app.parse() {
            Ok(app_type) => match commands::list_prompts(app_type) {
                Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize prompts"),
                Err(message) => {
                    serde_json::to_value(types::err::<()>("prompts_list_failed", message))
                        .expect("serialize prompts error")
                }
            },
            Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                .expect("serialize invalid app error"),
        },
        [group, cmd, app, id, prompt_json] if group == "prompts" && cmd == "upsert" => {
            match app.parse() {
                Ok(app_type) => match commands::upsert_prompt(app_type, id, prompt_json) {
                    Ok(value) => {
                        serde_json::to_value(types::ok(value)).expect("serialize prompt upsert")
                    }
                    Err(message) => {
                        serde_json::to_value(types::err::<()>("prompt_upsert_failed", message))
                            .expect("serialize prompt upsert error")
                    }
                },
                Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                    .expect("serialize invalid app error"),
            }
        }
        [group, cmd, app, id] if group == "prompts" && cmd == "delete" => match app.parse() {
            Ok(app_type) => match commands::delete_prompt(app_type, id) {
                Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize delete"),
                Err(message) => {
                    serde_json::to_value(types::err::<()>("prompt_delete_failed", message))
                        .expect("serialize prompt delete error")
                }
            },
            Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                .expect("serialize invalid app error"),
        },
        [group, cmd, app, id] if group == "prompts" && cmd == "enable" => match app.parse() {
            Ok(app_type) => match commands::enable_prompt(app_type, id) {
                Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize enable"),
                Err(message) => {
                    serde_json::to_value(types::err::<()>("prompt_enable_failed", message))
                        .expect("serialize prompt enable error")
                }
            },
            Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                .expect("serialize invalid app error"),
        },
        [group, cmd, app] if group == "prompts" && cmd == "import" => match app.parse() {
            Ok(app_type) => match commands::import_prompt_from_file(app_type) {
                Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize import"),
                Err(message) => {
                    serde_json::to_value(types::err::<()>("prompt_import_failed", message))
                        .expect("serialize prompt import error")
                }
            },
            Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                .expect("serialize invalid app error"),
        },
        [group, cmd, app] if group == "prompts" && cmd == "current" => match app.parse() {
            Ok(app_type) => match commands::current_prompt_file_content(app_type) {
                Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize current"),
                Err(message) => {
                    serde_json::to_value(types::err::<()>("prompt_current_failed", message))
                        .expect("serialize prompt current error")
                }
            },
            Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                .expect("serialize invalid app error"),
        },
        [group, cmd] if group == "skills" && cmd == "installed" => {
            match commands::list_installed_skills() {
                Ok(value) => {
                    serde_json::to_value(types::ok(value)).expect("serialize skills installed")
                }
                Err(message) => {
                    serde_json::to_value(types::err::<()>("skills_installed_failed", message))
                        .expect("serialize skills installed error")
                }
            }
        }
        [group, cmd] if group == "skills" && cmd == "backups" => {
            match commands::list_skill_backups() {
                Ok(value) => {
                    serde_json::to_value(types::ok(value)).expect("serialize skill backups")
                }
                Err(message) => {
                    serde_json::to_value(types::err::<()>("skill_backups_failed", message))
                        .expect("serialize skill backups error")
                }
            }
        }
        [group, cmd, backup_id] if group == "skills" && cmd == "delete-backup" => {
            match commands::delete_skill_backup(backup_id) {
                Ok(value) => {
                    serde_json::to_value(types::ok(value)).expect("serialize delete backup")
                }
                Err(message) => {
                    serde_json::to_value(types::err::<()>("skill_delete_backup_failed", message))
                        .expect("serialize delete backup error")
                }
            }
        }
        [group, cmd, skill_json, current_app] if group == "skills" && cmd == "install" => {
            match current_app.parse() {
                Ok(app_type) => match commands::install_skill_unified(skill_json, app_type) {
                    Ok(value) => {
                        serde_json::to_value(types::ok(value)).expect("serialize skill install")
                    }
                    Err(message) => {
                        serde_json::to_value(types::err::<()>("skill_install_failed", message))
                            .expect("serialize skill install error")
                    }
                },
                Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                    .expect("serialize invalid app error"),
            }
        }
        [group, cmd, id] if group == "skills" && cmd == "uninstall" => {
            match commands::uninstall_skill_unified(id) {
                Ok(value) => {
                    serde_json::to_value(types::ok(value)).expect("serialize skill uninstall")
                }
                Err(message) => {
                    serde_json::to_value(types::err::<()>("skill_uninstall_failed", message))
                        .expect("serialize skill uninstall error")
                }
            }
        }
        [group, cmd, backup_id, current_app] if group == "skills" && cmd == "restore" => {
            match current_app.parse() {
                Ok(app_type) => match commands::restore_skill_backup(backup_id, app_type) {
                    Ok(value) => {
                        serde_json::to_value(types::ok(value)).expect("serialize skill restore")
                    }
                    Err(message) => {
                        serde_json::to_value(types::err::<()>("skill_restore_failed", message))
                            .expect("serialize skill restore error")
                    }
                },
                Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                    .expect("serialize invalid app error"),
            }
        }
        [group, cmd, id, app, enabled] if group == "skills" && cmd == "toggle" => {
            match app.parse() {
                Ok(app_type) => {
                    let enabled = enabled == "true";
                    match commands::toggle_skill_app(id, app_type, enabled) {
                        Ok(value) => {
                            serde_json::to_value(types::ok(value)).expect("serialize skill toggle")
                        }
                        Err(message) => {
                            serde_json::to_value(types::err::<()>("skill_toggle_failed", message))
                                .expect("serialize skill toggle error")
                        }
                    }
                }
                Err(err) => serde_json::to_value(types::err::<()>("invalid_app", err.to_string()))
                    .expect("serialize invalid app error"),
            }
        }
        [group, cmd] if group == "skills" && cmd == "scan-unmanaged" => {
            match commands::scan_unmanaged_skills() {
                Ok(value) => {
                    serde_json::to_value(types::ok(value)).expect("serialize unmanaged skills")
                }
                Err(message) => {
                    serde_json::to_value(types::err::<()>("skill_scan_failed", message))
                        .expect("serialize unmanaged skills error")
                }
            }
        }
        [group, cmd, imports_json] if group == "skills" && cmd == "import" => {
            match commands::import_skills_from_apps(imports_json) {
                Ok(value) => {
                    serde_json::to_value(types::ok(value)).expect("serialize skill import")
                }
                Err(message) => {
                    serde_json::to_value(types::err::<()>("skill_import_failed", message))
                        .expect("serialize skill import error")
                }
            }
        }
        [group, cmd] if group == "skills" && cmd == "discover" => {
            match commands::discover_available_skills() {
                Ok(value) => {
                    serde_json::to_value(types::ok(value)).expect("serialize skill discover")
                }
                Err(message) => {
                    serde_json::to_value(types::err::<()>("skill_discover_failed", message))
                        .expect("serialize skill discover error")
                }
            }
        }
        [group, cmd] if group == "skills" && cmd == "check-updates" => {
            match commands::check_skill_updates() {
                Ok(value) => {
                    serde_json::to_value(types::ok(value)).expect("serialize skill updates")
                }
                Err(message) => {
                    serde_json::to_value(types::err::<()>("skill_check_updates_failed", message))
                        .expect("serialize skill updates error")
                }
            }
        }
        [group, cmd, id] if group == "skills" && cmd == "update" => {
            match commands::update_skill(id) {
                Ok(value) => {
                    serde_json::to_value(types::ok(value)).expect("serialize skill update")
                }
                Err(message) => {
                    serde_json::to_value(types::err::<()>("skill_update_failed", message))
                        .expect("serialize skill update error")
                }
            }
        }
        [group, cmd] if group == "skills" && cmd == "repos" => match commands::list_skill_repos() {
            Ok(value) => serde_json::to_value(types::ok(value)).expect("serialize skill repos"),
            Err(message) => serde_json::to_value(types::err::<()>("skill_repos_failed", message))
                .expect("serialize skill repos error"),
        },
        [group, cmd, repo_json] if group == "skills" && cmd == "add-repo" => {
            match commands::add_skill_repo(repo_json) {
                Ok(value) => {
                    serde_json::to_value(types::ok(value)).expect("serialize skill add repo")
                }
                Err(message) => {
                    serde_json::to_value(types::err::<()>("skill_add_repo_failed", message))
                        .expect("serialize skill add repo error")
                }
            }
        }
        [group, cmd, owner, name] if group == "skills" && cmd == "remove-repo" => {
            match commands::remove_skill_repo(owner, name) {
                Ok(value) => {
                    serde_json::to_value(types::ok(value)).expect("serialize skill remove repo")
                }
                Err(message) => {
                    serde_json::to_value(types::err::<()>("skill_remove_repo_failed", message))
                        .expect("serialize skill remove repo error")
                }
            }
        }
        _ => serde_json::to_value(types::err::<()>(
            "unsupported_command",
            "Supported commands: status, providers, openclaw, mcp, prompts, skills",
        ))
        .expect("serialize error response"),
    }
}
