//! New environment creation and cloning modal.

use super::*;

impl ShiplyApp {
    pub(crate) fn render_new_environment_modal(&mut self, ctx: &egui::Context) {
        let SidebarSelection::Project(proj_id) = self.selection.clone() else {
            self.show_new_env_modal = false;
            return;
        };
        let Some(proj_idx) = self.storage.projects.iter().position(|p| p.id == proj_id) else {
            self.show_new_env_modal = false;
            return;
        };

        let Some(anim_t) = prepare_animated_modal(ctx, "new_env", &mut self.show_new_env_modal)
        else {
            return;
        };

        let existing_envs: Vec<(String, String)> = self.storage.projects[proj_idx]
            .environments
            .iter()
            .map(|e| (e.key.clone(), e.name.clone()))
            .collect();

        let (window, inner_w) = animated_modal_window(ctx, "Add Environment Level", anim_t, None);
        window.show(ctx, |ui| {
            ui.set_width(inner_w);
                if render_modal_header(
                    ui,
                    "close_new_env_modal",
                    "Add Environment Level",
                    "Import an environment from a .ps1 script or create a custom level (dev, staging, production, UAT, QA).",
                ) {
                    self.show_new_env_modal = false;
                }
                ui.add_space(SP_16);

                // Option 1: Import from .ps1 script
                egui::Frame::new()
                    .fill(MD_PANEL_BG)
                    .stroke(Stroke::new(1.0_f32, MD_BORDER))
                    .corner_radius(R_LG)
                    .inner_margin(Margin::same(16))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("Import environment from .ps1 script")
                                .typo(Typo::Caption)
                                .color(ACCENT),
                        );
                        ui.add_space(SP_8);
                        let env_ps1_w = (inner_w - 32.0 - 208.0).max(120.0);
                        horizontal_row(ui, 40.0, |ui| {
                            modal_text_input(
                                ui,
                                "new_env_ps1_path",
                                &mut self.new_env_ps1_path,
                                env_ps1_w,
                                Some(r"e.g. C:\Projects\my-app\deploy-staging.ps1"),
                            );
                            ui.add_space(SP_8);
                            let browse_btn = modal_secondary_button("Browse .ps1...");
                            if ui.add(browse_btn).clicked() {
                                if let Some(picked) = pick_ps1_file_dialog() {
                                    self.new_env_ps1_path = picked;
                                }
                            }
                            ui.add_space(SP_8);
                            let can_import = !self.new_env_ps1_path.trim().is_empty();
                            let imp_btn = modal_primary_button("Import", can_import);
                            if ui.add_enabled(can_import, imp_btn).clicked() && can_import {
                                let path = PathBuf::from(
                                    self.new_env_ps1_path.trim().trim_matches('"'),
                                );
                                match parse_ps1_file(&path) {
                                    Ok(env_cfg) => {
                                        let key = env_cfg.key.clone();
                                        let name = env_cfg.name.clone();
                                        let proj = &mut self.storage.projects[proj_idx];
                                        if let Some(existing) = proj.get_env_mut(&key) {
                                            *existing = env_cfg;
                                        } else {
                                            proj.environments.push(env_cfg);
                                        }
                                        let _ = self.storage.save();
                                        self.selected_env_key = key;
                                        self.refresh_selected_project_metadata(true);
                                        self.status_banner = Some((
                                            format!(
                                                "Imported environment '{name}' from '{}'.",
                                                path.display()
                                            ),
                                            true,
                                        ));
                                        self.new_env_ps1_path.clear();
                                        self.show_new_env_modal = false;
                                    }
                                    Err(err) => {
                                        self.status_banner = Some((err, false));
                                    }
                                }
                            }
                        });
                    });

                ui.add_space(SP_16);

                // Option 2: Create by Name
                ui.label(
                    RichText::new("Or Create Custom Environment Name")
                        .typo(Typo::Label).color(MD_TEXT),
                );
                ui.add_space(SP_8);
                modal_text_input(
                    ui,
                    "new_env_name",
                    &mut self.new_env_name,
                    inner_w,
                    Some("e.g. Development, Staging, Production, UAT, QA"),
                );

                if !existing_envs.is_empty() {
                    ui.add_space(SP_16);
                    ui.label(
                        RichText::new("Clone Pipeline Template From:")
                            .typo(Typo::Label).color(MD_TEXT),
                    );
                    ui.add_space(SP_8);
                    ui.horizontal_wrapped(|ui| {
                        for (key, name) in &existing_envs {
                            let selected = self.new_env_clone_from == *key;
                            let btn = egui::Button::new(
                                RichText::new(name).typo(Typo::Caption).color(if selected {
                                    Color32::WHITE
                                } else {
                                    MD_TEXT
                                }),
                            )
                            .fill(if selected { ACCENT } else { MD_PANEL_BG })
                            .stroke(Stroke::new(
                                1.0_f32,
                                if selected { ACCENT } else { MD_BORDER },
                            ))
                            .corner_radius(R_SM);
                            if ui.add(btn).clicked() {
                                self.new_env_clone_from = key.clone();
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
                        RichText::new("New environments inherit active deployment settings")
                            .typo(Typo::Caption)
                            .color(MD_TEXT_MUTED),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let can_create = !self.new_env_name.trim().is_empty();
                        let create_btn = modal_primary_button("Add Environment", can_create);

                        if (ui.add_enabled(can_create, create_btn).clicked() || enter_pressed)
                            && can_create
                        {
                            let clone_src = if self.new_env_clone_from.is_empty() {
                                None
                            } else {
                                Some(self.new_env_clone_from.as_str())
                            };
                            match self.storage.projects[proj_idx]
                                .add_environment(&self.new_env_name, clone_src)
                            {
                                Ok(new_key) => {
                                    let _ = self.storage.save();
                                    self.selected_env_key = new_key;
                                    self.refresh_selected_project_metadata(true);
                                    self.status_banner = Some((
                                        format!(
                                            "Added environment '{}' successfully.",
                                            self.new_env_name.trim()
                                        ),
                                        true,
                                    ));
                                    self.new_env_name.clear();
                                    self.show_new_env_modal = false;
                                }
                                Err(err) => {
                                    self.status_banner = Some((err, false));
                                }
                            }
                        }

                        ui.add_space(SP_8);
                        let cancel_btn = modal_secondary_button("Cancel");
                        if ui.add(cancel_btn).clicked() {
                            self.show_new_env_modal = false;
                        }
                    });
                });
            });
    }
}
