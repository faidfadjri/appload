//! Environment server settings tab.

use super::*;

impl ShiplyApp {
    pub(crate) fn render_server_settings_tab(&mut self, ui: &mut egui::Ui, proj_idx: usize) {
        let env_key = self.selected_env_key.clone();
        let mut config_changed = false;
        let mut delete_current_env = false;
        let mut verify_vault_clicked = false;
        let mut open_cred_guide_on_overview = false;

        let current_env_name = self.storage.projects[proj_idx]
            .get_env(&env_key)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| env_key.clone());

        // Header
        ui.horizontal(|ui| {
            ui.label(Typo::H2.rich("Environment settings"));
            ui.add_space(SP_8);
            let env_colors = env_pastel_pill_colors(&current_env_name);
            kit::tag(ui, &current_env_name, env_colors);
            ui.add_space(SP_8);
            if self.cached_vault_found {
                kit::status_badge(ui, StatusKind::Success, "Vault ready");
            } else {
                kit::status_badge(ui, StatusKind::Warning, "Vault missing");
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if kit::button(
                    ui,
                    ButtonVariant::Secondary,
                    ButtonSize::Sm,
                    "Manage secrets",
                    Some(OutlineIcon::Key),
                )
                .on_hover_text("Open Secret Management to configure OS vault credentials")
                .clicked()
                {
                    open_cred_guide_on_overview = true;
                }
            });
        });

        ui.add_space(SP_16);

        let total_avail = ui.available_width();
        let max_content_w = total_avail.min(880.0);
        let pad_x = ((total_avail - max_content_w) / 2.0).max(0.0);

        let mut guide_vault_target = String::new();
        let mut guide_remote_user = String::new();

        ui.horizontal(|ui| {
            if pad_x > 0.0 {
                ui.add_space(pad_x);
            }
            ui.vertical(|ui| {
                ui.set_max_width(max_content_w);
                let content_w = ui.available_width();
                let is_2col = content_w >= 600.0;
                let col_w = if is_2col {
                    (content_w - 40.0 - SP_16) / 2.0
                } else {
                    content_w - 40.0
                };
                let full_w = content_w - 40.0;

                let project = &mut self.storage.projects[proj_idx];

                // Card 1: General
                kit::card(ui, |ui| {
                    ui.label(Typo::H3.rich("General"));
                    ui.add_space(2.0);
                    ui.label(Typo::Small.rich("Environment name and local repository path").color(TEXT_SECONDARY));
                    ui.add_space(SP_12);

                    if let Some(env_cfg) = project.get_env_mut(&env_key) {
                        if is_2col {
                            ui.horizontal(|ui| {
                                kit::form_field(ui, "Environment name", None, true, |ui| {
                                    if styled_text_edit(
                                        ui,
                                        ("srv_env_name", &env_key),
                                        &mut env_cfg.name,
                                        col_w,
                                        Some("e.g. Development, Staging, Production"),
                                    ) {
                                        config_changed = true;
                                    }
                                });
                                ui.add_space(SP_16);
                                kit::form_field(ui, "Application name (PM2 / folder)", None, false, |ui| {
                                    if styled_text_edit(
                                        ui,
                                        ("srv_app_name", &env_key),
                                        &mut env_cfg.app_name,
                                        col_w,
                                        Some("e.g. my-web-service-dev"),
                                    ) {
                                        config_changed = true;
                                    }
                                });
                            });
                        } else {
                            kit::form_field(ui, "Environment name", None, true, |ui| {
                                if styled_text_edit(
                                    ui,
                                    ("srv_env_name", &env_key),
                                    &mut env_cfg.name,
                                    full_w,
                                    Some("e.g. Development, Staging, Production"),
                                ) {
                                    config_changed = true;
                                }
                            });
                            ui.add_space(SP_12);
                            kit::form_field(ui, "Application name (PM2 / folder)", None, false, |ui| {
                                if styled_text_edit(
                                    ui,
                                    ("srv_app_name", &env_key),
                                    &mut env_cfg.app_name,
                                    full_w,
                                    Some("e.g. my-web-service-dev"),
                                ) {
                                    config_changed = true;
                                }
                            });
                        }

                        ui.add_space(SP_12);
                        kit::form_field(ui, "Local project directory", None, true, |ui| {
                            ui.horizontal(|ui| {
                                let path_input_w = (full_w - 96.0).max(120.0);
                                if styled_text_edit(
                                    ui,
                                    "srv_local_path",
                                    &mut project.local_path,
                                    path_input_w,
                                    Some(r"e.g. C:\Projects\my-web-service"),
                                ) {
                                    config_changed = true;
                                }
                                ui.add_space(SP_8);
                                if kit::button(
                                    ui,
                                    ButtonVariant::Secondary,
                                    ButtonSize::Md,
                                    "Browse",
                                    Some(OutlineIcon::Folder),
                                )
                                .on_hover_text("Browse local folder")
                                .clicked()
                                {
                                    if let Some(folder) = helpers::pick_folder_dialog() {
                                        project.local_path = folder;
                                        config_changed = true;
                                    }
                                }
                            });
                        });
                    }
                });

                ui.add_space(SP_16);

                // Card 2: Servers
                kit::card(ui, |ui| {
                    ui.label(Typo::H3.rich("Servers"));
                    ui.add_space(2.0);
                    ui.label(Typo::Small.rich("Target destination and SSH access").color(TEXT_SECONDARY));
                    ui.add_space(SP_12);

                    if let Some(env_cfg) = project.get_env_mut(&env_key) {
                        let servers_text = match &mut self.servers_edit_buffer {
                            Some((k, buf)) if k == &env_key => buf,
                            _ => {
                                let slot = self.servers_edit_buffer.insert((env_key.clone(), env_cfg.servers.join(", ")));
                                &mut slot.1
                            }
                        };

                        kit::form_field(
                            ui,
                            "Target servers",
                            Some("Comma-separated hosts or IPs"),
                            true,
                            |ui| {
                                if styled_text_edit(
                                    ui,
                                    ("srv_servers", &env_key),
                                    servers_text,
                                    full_w,
                                    Some("e.g. 192.168.1.10, 192.168.1.11"),
                                ) {
                                    env_cfg.servers = servers_text
                                        .split(',')
                                        .map(|s| s.trim().to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                    config_changed = true;
                                }
                            },
                        );

                        ui.add_space(SP_12);

                        if is_2col {
                            ui.horizontal(|ui| {
                                kit::form_field(ui, "Remote directory", None, true, |ui| {
                                    if styled_text_edit(
                                        ui,
                                        ("srv_remote_dir", &env_key),
                                        &mut env_cfg.remote_dir,
                                        col_w,
                                        Some("e.g. /var/www/my-web-service"),
                                    ) {
                                        config_changed = true;
                                    }
                                });
                                ui.add_space(SP_16);
                                kit::form_field(ui, "SSH username", None, true, |ui| {
                                    if styled_text_edit(
                                        ui,
                                        ("srv_remote_user", &env_key),
                                        &mut env_cfg.remote_user,
                                        col_w,
                                        Some("e.g. deploy or ubuntu"),
                                    ) {
                                        config_changed = true;
                                    }
                                });
                            });
                        } else {
                            kit::form_field(ui, "Remote directory", None, true, |ui| {
                                if styled_text_edit(
                                    ui,
                                    ("srv_remote_dir", &env_key),
                                    &mut env_cfg.remote_dir,
                                    full_w,
                                    Some("e.g. /var/www/my-web-service"),
                                ) {
                                    config_changed = true;
                                }
                            });
                            ui.add_space(SP_12);
                            kit::form_field(ui, "SSH username", None, true, |ui| {
                                if styled_text_edit(
                                    ui,
                                    ("srv_remote_user", &env_key),
                                    &mut env_cfg.remote_user,
                                    full_w,
                                    Some("e.g. deploy or ubuntu"),
                                ) {
                                    config_changed = true;
                                }
                            });
                        }
                    }
                });

                ui.add_space(SP_16);

                // Card 3: Authentication
                kit::card(ui, |ui| {
                    ui.label(Typo::H3.rich("Authentication"));
                    ui.add_space(2.0);
                    ui.label(Typo::Small.rich("OS Credential Vault integration and passwords").color(TEXT_SECONDARY));
                    ui.add_space(SP_12);

                    if let Some(env_cfg) = project.get_env_mut(&env_key) {
                        guide_vault_target = env_cfg.vault_target.trim().to_string();
                        guide_remote_user = env_cfg.remote_user.trim().to_string();

                        kit::form_field(
                            ui,
                            "Vault target",
                            Some("Key in OS Credential Manager (stored encrypted)"),
                            false,
                            |ui| {
                                ui.horizontal(|ui| {
                                    let btn_reserve = 190.0;
                                    let vault_input_w = (full_w - btn_reserve).max(120.0);
                                    if styled_text_edit(
                                        ui,
                                        ("srv_vault_target", &env_key),
                                        &mut env_cfg.vault_target,
                                        vault_input_w,
                                        Some("e.g. prod-ssh-server or my-vault-target"),
                                    ) {
                                        self.cached_vault_found =
                                            read_os_vault_credential(&env_cfg.vault_target).is_some();
                                        config_changed = true;
                                    }
                                    ui.add_space(SP_8);
                                    if kit::button(
                                        ui,
                                        ButtonVariant::Secondary,
                                        ButtonSize::Md,
                                        "Verify",
                                        Some(OutlineIcon::Refresh),
                                    )
                                    .on_hover_text("Check if this target exists in your OS Credential Manager / Keychain")
                                    .clicked()
                                    {
                                        verify_vault_clicked = true;
                                    }
                                    ui.add_space(SP_8);
                                    if kit::button(
                                        ui,
                                        ButtonVariant::Secondary,
                                        ButtonSize::Md,
                                        "Set secret",
                                        Some(OutlineIcon::Key),
                                    )
                                    .on_hover_text("Configure username & password for this target in Secret Management")
                                    .clicked()
                                    {
                                        open_cred_guide_on_overview = true;
                                    }
                                });
                            },
                        );

                        ui.add_space(SP_12);

                        kit::form_field(
                            ui,
                            "Password override",
                            Some("Leave blank to use the OS vault or an SSH key"),
                            false,
                            |ui| {
                                styled_password_edit(
                                    ui,
                                    ("srv_manual_pwd", &env_key),
                                    &mut self.manual_ssh_password,
                                    full_w,
                                    Some("Optional — leave blank to use OS Credential Vault or SSH Key"),
                                );
                            },
                        );
                    }
                });

                ui.add_space(SP_16);

                // Card 4: Runtime
                kit::card(ui, |ui| {
                    ui.label(Typo::H3.rich("Runtime"));
                    ui.add_space(2.0);
                    ui.label(Typo::Small.rich("Environment variables and network proxy").color(TEXT_SECONDARY));
                    ui.add_space(SP_12);

                    if let Some(env_cfg) = project.get_env_mut(&env_key) {
                        let env_src_text = match &mut self.server_env_source_buffer {
                            Some((k, buf)) if k == &env_key => buf,
                            _ => {
                                let slot = self.server_env_source_buffer.insert((env_key.clone(), env_cfg.env_source.clone().unwrap_or_default()));
                                &mut slot.1
                            }
                        };

                        if is_2col {
                            ui.horizontal(|ui| {
                                kit::form_field(ui, "Env file (.env source)", None, false, |ui| {
                                    if styled_text_edit(
                                        ui,
                                        ("srv_env_source", &env_key),
                                        env_src_text,
                                        col_w,
                                        Some("Optional — e.g. ./.env.dev or ./.env.production"),
                                    ) {
                                        env_cfg.env_source = if env_src_text.trim().is_empty() {
                                            None
                                        } else {
                                            Some(env_src_text.clone())
                                        };
                                        config_changed = true;
                                    }
                                });
                                ui.add_space(SP_16);
                                let proxy_text = match &mut self.server_http_proxy_buffer {
                                    Some((k, buf)) if k == &env_key => buf,
                                    _ => {
                                        let slot = self.server_http_proxy_buffer.insert((env_key.clone(), env_cfg.http_proxy.clone().unwrap_or_default()));
                                        &mut slot.1
                                    }
                                };
                                kit::form_field(ui, "HTTP proxy", None, false, |ui| {
                                    if styled_text_edit(
                                        ui,
                                        ("srv_http_proxy", &env_key),
                                        proxy_text,
                                        col_w,
                                        Some("Optional — e.g. http://proxy.example.com:8080"),
                                    ) {
                                        env_cfg.http_proxy = if proxy_text.trim().is_empty() {
                                            None
                                        } else {
                                            Some(proxy_text.clone())
                                        };
                                        config_changed = true;
                                    }
                                });
                            });
                        } else {
                            kit::form_field(ui, "Env file (.env source)", None, false, |ui| {
                                if styled_text_edit(
                                    ui,
                                    ("srv_env_source", &env_key),
                                    env_src_text,
                                    full_w,
                                    Some("Optional — e.g. ./.env.dev or ./.env.production"),
                                ) {
                                    env_cfg.env_source = if env_src_text.trim().is_empty() {
                                        None
                                    } else {
                                        Some(env_src_text.clone())
                                    };
                                    config_changed = true;
                                }
                            });
                            ui.add_space(SP_12);
                            let proxy_text = match &mut self.server_http_proxy_buffer {
                                Some((k, buf)) if k == &env_key => buf,
                                _ => {
                                    let slot = self.server_http_proxy_buffer.insert((env_key.clone(), env_cfg.http_proxy.clone().unwrap_or_default()));
                                    &mut slot.1
                                }
                            };
                            kit::form_field(ui, "HTTP proxy", None, false, |ui| {
                                if styled_text_edit(
                                    ui,
                                    ("srv_http_proxy", &env_key),
                                    proxy_text,
                                    full_w,
                                    Some("Optional — e.g. http://proxy.example.com:8080"),
                                ) {
                                    env_cfg.http_proxy = if proxy_text.trim().is_empty() {
                                        None
                                    } else {
                                        Some(proxy_text.clone())
                                    };
                                    config_changed = true;
                                }
                            });
                        }
                    }
                });

                ui.add_space(SP_16);

                // Danger zone card
                egui::Frame::new()
                    .fill(CARD_BG)
                    .stroke(Stroke::new(1.0_f32, DANGER_BORDER))
                    .corner_radius(CornerRadius::same(R_LG as u8))
                    .shadow(Shadow {
                        offset: [0, 1],
                        blur: 2,
                        spread: 0,
                        color: SHADOW_CARD,
                    })
                    .inner_margin(Margin::same(20))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(Typo::H3.rich("Delete environment").color(DANGER));
                                ui.add_space(4.0);
                                ui.label(
                                    Typo::Small
                                        .rich("Removes this environment and its pipeline. Deployment history is kept.")
                                        .color(TEXT_MUTED),
                                );
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if kit::button(
                                    ui,
                                    ButtonVariant::Danger,
                                    ButtonSize::Sm,
                                    "Delete environment",
                                    Some(OutlineIcon::Trash),
                                )
                                .on_hover_text("Delete this environment from this project")
                                .clicked()
                                {
                                    delete_current_env = true;
                                }
                            });
                        });
                    });

                ui.add_space(SP_24);
            });
        });

        // Handlers
        if open_cred_guide_on_overview {
            let desc = format!(
                "SSH Vault target for {} ({})",
                self.storage.projects[proj_idx].name, current_env_name
            );
            self.open_secret_management_for_target(&guide_vault_target, &guide_remote_user, &desc);
        } else if verify_vault_clicked {
            let _ = self.storage.save();
            self.refresh_selected_project_metadata(false);
            if self.cached_vault_found {
                self.status_banner = Some((
                    format!(
                        "Verified! Found credential for target '{guide_vault_target}' in OS Vault."
                    ),
                    true,
                ));
            } else {
                self.status_banner = Some((
                    format!(
                        "No credential found for target '{guide_vault_target}' in OS Vault. Click 'Set Secret' or 'Secret Management' in the sidebar to save it."
                    ),
                    false,
                ));
            }
        } else if delete_current_env {
            self.servers_edit_buffer = None;
            self.pending_delete_confirm = Some(ConfirmDeleteAction::Environment {
                project_id: self.storage.projects[proj_idx].id.clone(),
                project_name: self.storage.projects[proj_idx].name.clone(),
                env_key: env_key.clone(),
                env_name: current_env_name.clone(),
            });
        } else if config_changed {
            self.mark_config_dirty();
        }
    }
}
