//! Bulk multi-project deployment orchestration modal.

use super::*;

impl AppLoadApp {
    pub(crate) fn render_bulk_deploy_modal(&mut self, ctx: &egui::Context) {
        let Some(anim_t) =
            prepare_animated_modal(ctx, "bulk_deploy", &mut self.show_bulk_deploy_modal)
        else {
            return;
        };
        let (window, inner_w) =
            animated_modal_window_with_width(ctx, "Bulk Deployment", anim_t, 720.0, None);
        window.show(ctx, |ui| {
            ui.set_width(inner_w);

                if render_modal_header(
                    ui,
                    "close_bulk_deploy_modal",
                    "Bulk Deployment — Select Projects to Deploy",
                    "Choose which projects to deploy, select the target environment, and review each project's auto-bumped release version.",
                ) {
                    self.show_bulk_deploy_modal = false;
                }
                ui.add_space(SP_12);

                // Target Environment Selector + Select All / Clear All
                let env_tabs = self.storage.existing_environment_tabs(None);
                let available_envs: Vec<(String, String)> = env_tabs
                    .into_iter()
                    .filter_map(|(name, key_opt)| key_opt.map(|k| (name, k)))
                    .collect();

                let mut env_changed = false;
                horizontal_row(ui, 34.0, |ui| {
                    ui.label(
                        RichText::new("Target Environment:")
                            .typo(Typo::Label).color(MD_TEXT),
                    );
                    let current_env_label = available_envs
                        .iter()
                        .find(|(_, k)| k == &self.bulk_target_env_key)
                        .map(|(n, k)| format!("{n} ({k})"))
                        .unwrap_or_else(|| {
                            if self.bulk_target_env_key.is_empty() {
                                "Default (First Environment)".to_string()
                            } else {
                                self.bulk_target_env_key.clone()
                            }
                        });

                    egui::ComboBox::from_id_salt("bulk_target_env_combo")
                        .selected_text(
                            RichText::new(current_env_label)
                                .typo(Typo::Label)
                                .color(ACCENT),
                        )
                        .width(190.0)
                        .show_ui(ui, |ui| {
                            for (env_name, env_key) in &available_envs {
                                let is_sel = &self.bulk_target_env_key == env_key;
                                if ui
                                    .selectable_label(is_sel, format!("{env_name} ({env_key})"))
                                    .clicked()
                                    && !is_sel
                                {
                                    self.bulk_target_env_key = env_key.clone();
                                    env_changed = true;
                                }
                            }
                        });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let clear_btn = egui::Button::new(
                            RichText::new("Clear All")
                                .typo(Typo::Caption)
                                .color(MD_TEXT_MUTED),
                        )
                        .fill(MD_PANEL_BG)
                        .stroke(Stroke::new(1.0_f32, MD_BORDER))
                        .corner_radius(R_SM)
                        .min_size(Vec2::new(72.0, 28.0));
                        if ui.add(clear_btn).clicked() {
                            self.bulk_selected_projects.clear();
                        }

                        ui.add_space(SP_8);
                        let all_btn = egui::Button::new(
                            RichText::new("Select All")
                                .typo(Typo::Caption)
                                .color(ACCENT),
                        )
                        .fill(ACCENT_SOFT)
                        .stroke(Stroke::new(1.0_f32, ACCENT.gamma_multiply(0.4)))
                        .corner_radius(R_SM)
                        .min_size(Vec2::new(76.0, 28.0));
                        if ui.add(all_btn).clicked() {
                            for proj in &self.storage.projects {
                                if !proj.environments.is_empty() {
                                    self.bulk_selected_projects.insert(proj.id.clone());
                                }
                            }
                        }
                    });
                });

                if env_changed {
                    self.sync_bulk_project_versions();
                }

                ui.add_space(SP_8);

                // Project Checklist Table
                egui::Frame::new()
                    .fill(MD_PANEL_BG)
                    .stroke(Stroke::new(1.0_f32, MD_BORDER))
                    .corner_radius(R_MD)
                    .inner_margin(Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ScrollArea::vertical()
                            .id_salt("bulk_projects_list_scroll")
                            .max_height(230.0)
                            .show(ui, |ui| {
                                struct BulkProjectRow {
                                    id: String,
                                    name: String,
                                    resolved_env: Option<(String, String)>,
                                }

                                let proj_snapshots: Vec<BulkProjectRow> = self
                                    .storage
                                    .projects
                                    .iter()
                                    .map(|p| {
                                        let resolved_env = p
                                            .get_env(&self.bulk_target_env_key)
                                            .or_else(|| p.environments.first())
                                            .map(|e| (e.key.clone(), e.name.clone()));
                                        BulkProjectRow {
                                            id: p.id.clone(),
                                            name: p.name.clone(),
                                            resolved_env,
                                        }
                                    })
                                    .collect();

                                for row in proj_snapshots {
                                    let pid = row.id;
                                    let pname = row.name;
                                    let resolved_env = row.resolved_env;
                                    let mut checked = self.bulk_selected_projects.contains(&pid);
                                    let has_env = resolved_env.is_some();
                                    horizontal_row(ui, 38.0, |ui| {
                                        if ui
                                            .add_enabled(
                                                has_env,
                                                egui::Checkbox::new(
                                                    &mut checked,
                                                    RichText::new(&pname).typo(Typo::Label)
                                                        .color(MD_TEXT),
                                                ),
                                            )
                                            .changed()
                                        {
                                            if checked {
                                                self.bulk_selected_projects.insert(pid.clone());
                                            } else {
                                                self.bulk_selected_projects.remove(&pid);
                                            }
                                        }

                                        if let Some((ekey, ename)) = &resolved_env {
                                            let (bg, fg) = env_pastel_pill_colors(ekey);
                                            render_pill(ui, &ename.to_uppercase(), bg, fg);
                                        } else {
                                            render_pill(
                                                ui,
                                                "No environment",
                                                FAIL_PILL_BG,
                                                FAIL_PILL_TEXT,
                                            );
                                        }

                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            let ver_entry = self
                                                .bulk_project_versions
                                                .entry(pid.clone())
                                                .or_insert_with(|| "1.0.0".to_string());
                                            modal_text_input(
                                                ui,
                                                ("bulk_ver_entry", &pid),
                                                ver_entry,
                                                90.0,
                                                Some("1.0.0"),
                                            );
                                            ui.add_space(SP_4);
                                            ui.label(
                                                RichText::new("Version:")
                                                    .typo(Typo::Caption)
                                                    .color(MD_TEXT_MUTED),
                                            );
                                        });
                                    });
                                    draw_divider(ui, MD_BORDER);
                                }
                            });
                    });

                ui.add_space(SP_12);

                // Options + Trigger Action Footer (top border 1px MD_BORDER, padding-top 16px, margin-top 24px)
                ui.add_space(SP_12);
                draw_divider(ui, MD_BORDER);
                ui.add_space(SP_16);

                horizontal_row(ui, 40.0, |ui| {
                    ui.checkbox(
                        &mut self.bulk_bump_pkg,
                        RichText::new("Bump package.json")
                            .typo(Typo::Small)
                            .color(MD_TEXT_MUTED),
                    );
                    ui.add_space(SP_8);
                    ui.checkbox(
                        &mut self.bulk_create_tag,
                        RichText::new("Git Tag")
                            .typo(Typo::Small)
                            .color(MD_TEXT_MUTED),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let sel_count = self.bulk_selected_projects.len();
                        let run_btn = modal_primary_button(&format!("Deploy {sel_count} Selected"), sel_count > 0);

                        if ui.add_enabled(sel_count > 0, run_btn).clicked() {
                            self.trigger_bulk_deployment();
                        }

                        ui.add_space(SP_8);
                        let cancel_btn = modal_secondary_button("Cancel");
                        if ui.add(cancel_btn).clicked() {
                            self.show_bulk_deploy_modal = false;
                        }
                    });
                });
            });
    }
}
