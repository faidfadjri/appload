//! Project creation and configuration editing modals.

use super::*;

impl AppLoadApp {
    pub(crate) fn render_new_project_modal(&mut self, ctx: &egui::Context) {
        let Some(anim_t) =
            prepare_animated_modal(ctx, "new_project", &mut self.show_new_project_modal)
        else {
            return;
        };
        let (window, inner_w) = animated_modal_window(ctx, "Add Project", anim_t, None);
        window.show(ctx, |ui| {
            ui.set_width(inner_w);

                if render_modal_header(
                    ui,
                    "close_new_proj_modal",
                    "Add New Project",
                    "Auto-import a PowerShell deployment script (.ps1) to populate environments and steps, or configure manually.",
                ) {
                    self.show_new_project_modal = false;
                }

                ui.add_space(SP_16);

                // Auto-Import from PowerShell Script Box
                egui::Frame::new()
                    .fill(SUBTLE_BG)
                    .stroke(Stroke::new(1.0_f32, CARD_BORDER))
                    .corner_radius(CornerRadius::same(R_MD as u8))
                    .inner_margin(Margin::same(14))
                    .show(ui, |ui| {
                        ui.label(Typo::H3.rich("Auto-import from PowerShell script"));
                        ui.add_space(2.0);
                        ui.label(
                            Typo::Small
                                .rich("Select a deploy.ps1 file to auto-populate environments and pipeline steps.")
                                .color(TEXT_MUTED),
                        );
                        ui.add_space(SP_12);
                        let ps1_input_w = (inner_w - 28.0 - 200.0).max(120.0);
                        horizontal_row(ui, 40.0, |ui| {
                            modal_text_input(
                                ui,
                                "new_project_ps1_path",
                                &mut self.new_project_ps1_path,
                                ps1_input_w,
                                Some(r"e.g. C:\Projects\my-app\deploy-dev.ps1"),
                            );
                            ui.add_space(SP_8);
                            let browse_btn = modal_secondary_button("Browse…");
                            if ui.add(browse_btn).clicked() {
                                if let Some(picked) = pick_ps1_file_dialog() {
                                    self.new_project_ps1_path = picked.clone();
                                    self.apply_ps1_auto_import_to_modal(&picked);
                                }
                            }
                            ui.add_space(SP_8);
                            let can_autofill = !self.new_project_ps1_path.trim().is_empty();
                            let parse_btn = modal_primary_button("Auto-fill", can_autofill);
                            if ui.add(parse_btn).clicked() && can_autofill {
                                let path_copy = self.new_project_ps1_path.clone();
                                self.apply_ps1_auto_import_to_modal(&path_copy);
                            }
                        });

                        ui.add_space(SP_8);
                        if kit::checkbox(
                            ui,
                            &mut self.new_project_import_siblings,
                            "Also auto-import sibling deploy-*.ps1 scripts in the same folder",
                        )
                        .changed()
                            && !self.new_project_ps1_path.trim().is_empty()
                        {
                            let path_copy = self.new_project_ps1_path.clone();
                            self.apply_ps1_auto_import_to_modal(&path_copy);
                        }

                        if let Some(notice) = &self.new_project_import_notice {
                            ui.add_space(SP_8);
                            let is_err = notice.starts_with("Import error:");
                            ui.label(
                                Typo::SmallMedium
                                    .rich(notice)
                                    .color(if is_err { DANGER } else { SUCCESS }),
                            );
                        }
                    });

                ui.add_space(SP_16);

                // Project Form Fields (vertical gap 16px, label Typo::Label weight 500, margin-bottom 8px)
                ui.label(
                    RichText::new("Project Name")
                        .typo(Typo::Label).color(MD_TEXT),
                );
                ui.add_space(SP_8);
                modal_text_input(
                    ui,
                    "new_project_name",
                    &mut self.new_project_name,
                    inner_w,
                    Some("e.g. my-web-service"),
                );

                ui.add_space(SP_16);
                ui.label(
                    RichText::new("Local Project Directory")
                        .typo(Typo::Label).color(MD_TEXT),
                );
                ui.add_space(SP_8);
                let dir_input_w = (inner_w - 152.0).max(140.0);
                horizontal_row(ui, 40.0, |ui| {
                    modal_text_input(
                        ui,
                        "new_project_path",
                        &mut self.new_project_path,
                        dir_input_w,
                        Some(r"e.g. C:\Projects\my-web-service"),
                    );
                    ui.add_space(SP_8);
                    let folder_btn = modal_secondary_button("Browse Folder...");
                    if ui.add(folder_btn).clicked() {
                        if let Some(folder) = pick_folder_dialog() {
                            self.new_project_path = folder.clone();
                            if self.new_project_name.trim().is_empty() {
                                self.apply_ps1_auto_import_to_modal(&folder);
                            }
                        }
                    }
                });

                ui.add_space(SP_16);
                ui.label(
                    RichText::new("Description")
                        .typo(Typo::Label).color(MD_TEXT),
                );
                ui.add_space(SP_8);
                modal_text_input(
                    ui,
                    "new_project_desc",
                    &mut self.new_project_desc,
                    inner_w,
                    Some("e.g. Backend API service (Node.js / PM2)"),
                );

                ui.add_space(SP_16);
                ui.label(
                    RichText::new("Group (Optional)")
                        .typo(Typo::Label).color(MD_TEXT),
                );
                ui.add_space(SP_8);
                modal_text_input(
                    ui,
                    "new_project_group",
                    &mut self.new_project_group,
                    inner_w,
                    Some("e.g. Backend, Frontend, Microservices..."),
                );
                let existing_groups = self.storage.existing_project_groups();
                if !existing_groups.is_empty() {
                    ui.add_space(SP_8);
                    horizontal_row(ui, 24.0, |ui| {
                        ui.label(Typo::Caption.rich("Quick select:").color(TEXT_MUTED));
                        for grp in existing_groups {
                            let is_sel = self.new_project_group == grp;
                            let (bg, fg) = if is_sel {
                                (PRIMARY_BLUE, Color32::WHITE)
                            } else {
                                (SUBTLE_BG, TEXT_SECONDARY)
                            };
                            let resp = ui.add(
                                egui::Button::new(Typo::Caption.rich(&grp).color(fg))
                                    .fill(bg)
                                    .stroke(Stroke::new(1.0_f32, if is_sel { PRIMARY_BLUE } else { CARD_BORDER }))
                                    .corner_radius(CornerRadius::same(R_SM as u8)),
                            );
                            if resp.clicked() {
                                self.new_project_group = grp;
                            }
                        }
                    });
                }

                // Footer (top border 1px MD_BORDER, padding-top 16px, margin-top 24px)
                ui.add_space(SP_24);
                draw_divider(ui, MD_BORDER);
                ui.add_space(SP_16);

                horizontal_row(ui, 40.0, |ui| {
                    let env_count = self.new_project_imported_envs.len();
                    ui.label(
                        RichText::new(if env_count > 0 {
                            format!("{env_count} environment(s) ready to import")
                        } else {
                            "Starts empty (you can add or import environments anytime)".to_string()
                        })
                        .typo(Typo::Caption)
                        .color(MD_TEXT_MUTED),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let can_create = !self.new_project_name.trim().is_empty()
                            && !self.new_project_path.trim().is_empty();
                        let create_btn = modal_primary_button("Create Project", can_create);

                        let enter_submitted = can_create && ctx.input(|i| i.key_pressed(egui::Key::Enter));
                        if (ui.add(create_btn).clicked() || enter_submitted) && can_create {
                            let slug = crate::domain::normalize_env_key(&self.new_project_name);
                            let proj = Project {
                                id: if slug.is_empty() {
                                    Uuid::new_v4().to_string()
                                } else {
                                    slug
                                },
                                name: self.new_project_name.trim().to_string(),
                                description: if self.new_project_desc.trim().is_empty() {
                                    "Custom deployment project".to_string()
                                } else {
                                    self.new_project_desc.trim().to_string()
                                },
                                local_path: self.new_project_path.trim().to_string(),
                                environments: self.new_project_imported_envs.clone(),
                                group: if self.new_project_group.trim().is_empty() {
                                    None
                                } else {
                                    Some(self.new_project_group.trim().to_string())
                                },
                                sort_order: Some(self.storage.projects.len() as i32),
                            };
                            let id = proj.id.clone();
                            let first_env = proj
                                .environments
                                .first()
                                .map(|e| e.key.clone())
                                .unwrap_or_default();
                            self.storage.projects.push(proj);
                            let _ = self.storage.save();
                            self.selection = SidebarSelection::Project(id);
                            self.selected_env_key = first_env;
                            self.refresh_selected_project_metadata(true);

                            self.new_project_ps1_path.clear();
                            self.new_project_name.clear();
                            self.new_project_path.clear();
                            self.new_project_desc.clear();
                            self.new_project_group.clear();
                            self.new_project_imported_envs.clear();
                            self.new_project_import_notice = None;
                            self.show_new_project_modal = false;
                        }

                        ui.add_space(SP_8);
                        let cancel_btn = modal_secondary_button("Cancel");
                        if ui.add(cancel_btn).clicked() {
                            self.show_new_project_modal = false;
                        }
                    });
                });
            });
    }

    pub(crate) fn open_edit_project_modal(&mut self, proj_idx: usize) {
        if let Some(proj) = self.storage.projects.get(proj_idx) {
            self.edit_project_id = proj.id.clone();
            self.edit_project_name = proj.name.clone();
            self.edit_project_path = proj.local_path.clone();
            self.edit_project_desc = proj.description.clone();
            self.edit_project_group = proj.group.clone().unwrap_or_default();
            self.edit_project_ps1_path.clear();
            self.edit_project_import_notice = None;
            self.show_edit_project_modal = true;
        }
    }

    pub(crate) fn render_edit_project_modal(&mut self, ctx: &egui::Context) {
        let Some(proj_idx) = self
            .storage
            .projects
            .iter()
            .position(|p| p.id == self.edit_project_id)
        else {
            self.show_edit_project_modal = false;
            return;
        };

        let Some(anim_t) =
            prepare_animated_modal(ctx, "edit_project", &mut self.show_edit_project_modal)
        else {
            return;
        };
        let (window, inner_w) = animated_modal_window(ctx, "Edit Project", anim_t, None);
        window.show(ctx, |ui| {
            ui.set_width(inner_w);

                if render_modal_header(
                    ui,
                    "close_edit_proj_modal",
                    "Edit Project Settings",
                    "Update the project name, local directory path, description, or sync environments from a .ps1 script.",
                ) {
                    self.show_edit_project_modal = false;
                }

                ui.add_space(SP_16);

                // Optional: Import/Sync additional .ps1 script into this project
                egui::Frame::new()
                    .fill(SUBTLE_BG)
                    .stroke(Stroke::new(1.0_f32, CARD_BORDER))
                    .corner_radius(CornerRadius::same(R_MD as u8))
                    .inner_margin(Margin::same(14))
                    .show(ui, |ui| {
                        ui.label(Typo::H3.rich("Sync / import environment from script"));
                        ui.add_space(2.0);
                        ui.label(
                            Typo::Small
                                .rich("Select a deploy.ps1 file to import or update an environment configuration.")
                                .color(TEXT_MUTED),
                        );
                        ui.add_space(SP_12);
                        let ps1_input_w = (inner_w - 28.0 - 180.0).max(120.0);
                        horizontal_row(ui, 40.0, |ui| {
                            modal_text_input(
                                ui,
                                "edit_project_ps1_path",
                                &mut self.edit_project_ps1_path,
                                ps1_input_w,
                                Some(r"e.g. C:\Projects\my-app\deploy-prod.ps1"),
                            );
                            ui.add_space(SP_8);
                            let browse_btn = modal_secondary_button("Browse…");
                            if ui.add(browse_btn).clicked() {
                                if let Some(picked) = pick_ps1_file_dialog() {
                                    self.edit_project_ps1_path = picked;
                                }
                            }
                            ui.add_space(SP_8);
                            let can_import = !self.edit_project_ps1_path.trim().is_empty();
                            let imp_btn = modal_primary_button("Import", can_import);
                            if ui.add_enabled(can_import, imp_btn).clicked() && can_import {
                                let p = PathBuf::from(
                                    self.edit_project_ps1_path.trim().trim_matches('"'),
                                );
                                match parse_ps1_file(&p) {
                                    Ok(env_cfg) => {
                                        let env_name = env_cfg.name.clone();
                                        let proj = &mut self.storage.projects[proj_idx];
                                        if let Some(existing) = proj.get_env_mut(&env_cfg.key) {
                                            *existing = env_cfg;
                                        } else {
                                            proj.environments.push(env_cfg);
                                        }
                                        let _ = self.storage.save();
                                        self.edit_project_import_notice = Some(format!(
                                            "Synced environment '{env_name}' from script."
                                        ));
                                    }
                                    Err(err) => {
                                        self.edit_project_import_notice =
                                            Some(format!("Import error: {err}"));
                                    }
                                }
                            }
                        });
                        if let Some(notice) = &self.edit_project_import_notice {
                            ui.add_space(SP_8);
                            let is_err = notice.starts_with("Import error:");
                            ui.label(
                                Typo::SmallMedium
                                    .rich(notice)
                                    .color(if is_err {
                                        DANGER
                                    } else {
                                        SUCCESS
                                    }),
                            );
                        }
                    });

                ui.add_space(SP_16);

                // Project Form Fields (vertical gap 16px, label Typo::Label weight 500, margin-bottom 8px)
                ui.label(
                    RichText::new("Project Name")
                        .typo(Typo::Label).color(MD_TEXT),
                );
                ui.add_space(SP_8);
                modal_text_input(
                    ui,
                    "edit_project_name",
                    &mut self.edit_project_name,
                    inner_w,
                    Some("e.g. my-web-service"),
                );

                ui.add_space(SP_16);
                ui.label(
                    RichText::new("Local Project Directory")
                        .typo(Typo::Label).color(MD_TEXT),
                );
                ui.add_space(SP_8);
                let dir_input_w = (inner_w - 152.0).max(140.0);
                horizontal_row(ui, 40.0, |ui| {
                    modal_text_input(
                        ui,
                        "edit_project_path",
                        &mut self.edit_project_path,
                        dir_input_w,
                        Some(r"e.g. C:\Projects\my-web-service"),
                    );
                    ui.add_space(SP_8);
                    let folder_btn = modal_secondary_button("Browse Folder...");
                    if ui.add(folder_btn).clicked() {
                        if let Some(folder) = pick_folder_dialog() {
                            self.edit_project_path = folder;
                        }
                    }
                });

                ui.add_space(SP_16);
                ui.label(
                    RichText::new("Description")
                        .typo(Typo::Label).color(MD_TEXT),
                );
                ui.add_space(SP_8);
                modal_text_input(
                    ui,
                    "edit_project_desc",
                    &mut self.edit_project_desc,
                    inner_w,
                    Some("e.g. Backend API service (Node.js / PM2)"),
                );

                ui.add_space(SP_16);
                ui.label(
                    RichText::new("Group (Optional)")
                        .typo(Typo::Label).color(MD_TEXT),
                );
                ui.add_space(SP_8);
                modal_text_input(
                    ui,
                    "edit_project_group",
                    &mut self.edit_project_group,
                    inner_w,
                    Some("e.g. Backend, Frontend, Microservices..."),
                );
                let existing_groups = self.storage.existing_project_groups();
                if !existing_groups.is_empty() {
                    ui.add_space(SP_8);
                    horizontal_row(ui, 24.0, |ui| {
                        ui.label(Typo::Caption.rich("Quick select:").color(TEXT_MUTED));
                        for grp in existing_groups {
                            let is_sel = self.edit_project_group == grp;
                            let (bg, fg) = if is_sel {
                                (PRIMARY_BLUE, Color32::WHITE)
                            } else {
                                (SUBTLE_BG, TEXT_SECONDARY)
                            };
                            let resp = ui.add(
                                egui::Button::new(Typo::Caption.rich(&grp).color(fg))
                                    .fill(bg)
                                    .stroke(Stroke::new(
                                        1.0_f32,
                                        if is_sel { PRIMARY_BLUE } else { CARD_BORDER },
                                    ))
                                    .corner_radius(CornerRadius::same(R_SM as u8)),
                            );
                            if resp.clicked() {
                                self.edit_project_group = grp;
                            }
                        }
                    });
                }

                // Footer (top border 1px MD_BORDER, padding-top 16px, margin-top 24px)
                ui.add_space(SP_24);
                draw_divider(ui, MD_BORDER);
                ui.add_space(SP_16);

                let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                horizontal_row(ui, 40.0, |ui| {
                    ui.label(
                        RichText::new("Changes are saved to local database")
                            .typo(Typo::Caption)
                            .color(MD_TEXT_MUTED),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let can_save = !self.edit_project_name.trim().is_empty()
                            && !self.edit_project_path.trim().is_empty();
                        let save_btn = modal_primary_button("Save Changes", can_save);

                        if (ui.add_enabled(can_save, save_btn).clicked() || enter_pressed)
                            && can_save
                        {
                            let proj = &mut self.storage.projects[proj_idx];
                            proj.name = self.edit_project_name.trim().to_string();
                            proj.local_path = self.edit_project_path.trim().to_string();
                            proj.description = if self.edit_project_desc.trim().is_empty() {
                                "Custom deployment project".to_string()
                            } else {
                                self.edit_project_desc.trim().to_string()
                            };
                            let grp = self.edit_project_group.trim().to_string();
                            proj.group = if grp.is_empty() { None } else { Some(grp) };
                            let updated_name = proj.name.clone();
                            let _ = self.storage.save();
                            self.refresh_selected_project_metadata(false);
                            self.status_banner = Some((
                                format!("Updated project '{updated_name}'."),
                                true,
                            ));
                            self.show_edit_project_modal = false;
                        }

                        ui.add_space(SP_8);
                        let cancel_btn = modal_secondary_button("Cancel");
                        if ui.add(cancel_btn).clicked() {
                            self.show_edit_project_modal = false;
                        }
                    });
                });
            });
    }
}
