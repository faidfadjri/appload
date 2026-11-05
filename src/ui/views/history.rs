//! Deployment history and audit log view.

use super::*;

#[derive(Default)]
pub(crate) struct HistoryActions {
    pub redeploy_target: Option<(DeploymentRecord, bool)>,
    pub cancel_target: Option<String>,
    pub delete_target: Option<DeploymentRecord>,
}

pub(crate) struct DesktopColWidths {
    pub col_status: f32,
    pub col_version: f32,
    pub col_branch: f32,
    pub col_commit: f32,
    pub col_deployed: f32,
    pub col_trigger: f32,
}

impl ShiplyApp {
    pub(crate) fn render_history_and_logs_view(
        &mut self,
        ui: &mut egui::Ui,
        project_filter: Option<&str>,
    ) {
        let is_project_view = project_filter.is_some();
        let project_envs: Vec<(String, String)> = if let Some(pid) = project_filter {
            self.storage
                .projects
                .iter()
                .find(|p| p.id == pid)
                .map(|p| {
                    p.environments
                        .iter()
                        .map(|e| (e.name.clone(), e.key.clone()))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let filter_options = if !is_project_view {
            let opts = self.storage.existing_environment_tabs(None);
            if self.history_env_filter.is_some()
                && !opts.iter().any(|(_, k)| *k == self.history_env_filter)
            {
                self.history_env_filter = None;
            }
            opts
        } else {
            Vec::new()
        };

        let mut switched_env_key = None;
        let mut switched_history_filter = None;

        let effective_env_filter_preview = if is_project_view {
            if self.selected_env_key.is_empty() {
                None
            } else {
                Some(self.selected_env_key.clone())
            }
        } else {
            self.history_env_filter.clone()
        };
        let deletable_count = self
            .storage
            .filter_deployments(
                project_filter,
                effective_env_filter_preview.as_deref(),
                None,
            )
            .into_iter()
            .filter(|d| d.status != DeploymentStatus::Running)
            .count();

        // Section Header + Filter Pills + Clear History Button (Fixed 34px height)
        let mut trigger_clear_history = false;
        horizontal_row(ui, 36.0, |ui| {
            if is_project_view {
                let mut selected_idx = project_envs
                    .iter()
                    .position(|(_, k)| k == &self.selected_env_key)
                    .unwrap_or(0);
                let env_names: Vec<&str> = project_envs.iter().map(|(n, _)| n.as_str()).collect();
                if !env_names.is_empty() && kit::segmented(ui, &mut selected_idx, &env_names) {
                    if let Some((_, k)) = project_envs.get(selected_idx) {
                        switched_env_key = Some(k.clone());
                    }
                }
            } else {
                let mut selected_idx = if let Some(ref cur) = self.history_env_filter {
                    filter_options
                        .iter()
                        .position(|(_, k)| k.as_ref() == Some(cur))
                        .unwrap_or(0)
                } else {
                    0
                };
                let filter_labels: Vec<&str> =
                    filter_options.iter().map(|(lbl, _)| lbl.as_str()).collect();
                if !filter_labels.is_empty()
                    && kit::segmented(ui, &mut selected_idx, &filter_labels)
                {
                    if let Some((_, opt_val)) = filter_options.get(selected_idx) {
                        switched_history_filter = Some(opt_val.clone());
                    }
                }
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

                // ⋯ Overflow menu button containing "Clear history…"
                let more_id = ui.make_persistent_id("history_overflow_menu");
                let more_resp = outline_icon_button(
                    ui,
                    ("history_more_btn", project_filter.unwrap_or("all")),
                    OutlineIcon::MoreHorizontal,
                    None,
                    TEXT_PRIMARY,
                    CARD_BG,
                    SUBTLE_BG,
                    CARD_BORDER,
                    true,
                )
                .on_hover_text("More actions");

                if more_resp.clicked() {
                    ui.memory_mut(|m| m.toggle_popup(more_id));
                }

                egui::popup_below_widget(
                    ui,
                    more_id,
                    &more_resp,
                    egui::PopupCloseBehavior::CloseOnClick,
                    |ui| {
                        ui.set_min_width(140.0);
                        let can_clear = deletable_count > 0;
                        let clr_btn = egui::Button::new(
                            RichText::new("Clear history…")
                                .typo(Typo::SmallMedium)
                                .color(if can_clear {
                                    FAIL_PILL_TEXT
                                } else {
                                    TEXT_PLACEHOLDER
                                }),
                        )
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE);
                        if ui.add_enabled(can_clear, clr_btn).clicked() {
                            trigger_clear_history = true;
                            ui.memory_mut(|m| m.close_popup());
                        }
                    },
                );

                // Search/filter input
                styled_text_edit(
                    ui,
                    "history_search_input",
                    &mut self.history_search_query,
                    160.0,
                    Some("Filter deployments..."),
                );
            });
        });

        if trigger_clear_history && deletable_count > 0 {
            let proj_name = project_filter.and_then(|pid| {
                self.storage
                    .projects
                    .iter()
                    .find(|p| p.id == pid)
                    .map(|p| p.name.clone())
            });
            let env_name = if is_project_view {
                project_envs
                    .iter()
                    .find(|(_, k)| k == &self.selected_env_key)
                    .map(|(n, _)| n.clone())
                    .or_else(|| {
                        if self.selected_env_key.is_empty() {
                            None
                        } else {
                            Some(self.selected_env_key.clone())
                        }
                    })
            } else {
                self.history_env_filter.clone()
            };
            self.pending_delete_confirm = Some(ConfirmDeleteAction::ClearDeploymentHistory {
                project_id: project_filter.map(str::to_string),
                project_name: proj_name,
                env_key: effective_env_filter_preview,
                env_name,
                count: deletable_count,
            });
        }

        if let Some(env_key) = switched_env_key {
            self.selected_env_key = env_key;
            self.history_visible_limit = 5;
            self.refresh_selected_project_metadata(true);
        } else if let Some(filter_val) = switched_history_filter {
            self.history_env_filter = filter_val;
            self.history_visible_limit = 5;
        }

        ui.add_space(10.0);

        // When viewing a project workspace, deployment history strictly filters by the active environment
        let effective_env_filter = if is_project_view {
            if self.selected_env_key.is_empty() {
                None
            } else {
                Some(self.selected_env_key.as_str())
            }
        } else {
            self.history_env_filter.as_deref()
        };

        let query = self.history_search_query.trim().to_lowercase();
        let filtered: Vec<DeploymentRecord> = self
            .storage
            .filter_deployments(project_filter, effective_env_filter, None)
            .into_iter()
            .filter(|d| {
                if query.is_empty() {
                    true
                } else {
                    d.version.to_lowercase().contains(&query)
                        || d.project_name.to_lowercase().contains(&query)
                        || d.environment.to_lowercase().contains(&query)
                        || d.git_branch
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&query)
                        || d.git_commit
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&query)
                }
            })
            .cloned()
            .collect();

        let is_stacked_view =
            ui.ctx().screen_rect().width() < 720.0 || ui.available_width() < 560.0;
        let total_count = filtered.len();
        let visible_count = total_count.min(self.history_visible_limit.max(5));

        let mut actions = HistoryActions::default();

        // Structured Deployment History Table Card
        egui::Frame::new()
            .fill(CARD_BG)
            .stroke(Stroke::new(1.0_f32, CARD_BORDER))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(18, 14))
            .show(ui, |ui| {
                // Fixed column widths: Status: 90, Version: 80, Branch: 120, Commit: 70, Deployed: 140, Trigger: 110, Actions: remaining
                let col_status = 90.0;
                let col_version = 80.0;
                let col_branch = 120.0;
                let col_commit = 70.0;
                let col_deployed = 140.0;
                let col_trigger = 110.0;

                if !is_stacked_view {
                    horizontal_row(ui, 24.0, |ui| {
                        ui.add_sized(
                            [col_status, 20.0],
                            egui::Label::new(
                                RichText::new("Status")
                                    .typo(Typo::Caption)
                                    .color(TEXT_MUTED),
                            ),
                        );
                        ui.add_sized(
                            [col_version, 20.0],
                            egui::Label::new(
                                RichText::new("Version")
                                    .typo(Typo::Caption)
                                    .color(TEXT_MUTED),
                            ),
                        );
                        ui.add_sized(
                            [col_branch, 20.0],
                            egui::Label::new(
                                RichText::new("Branch")
                                    .typo(Typo::Caption)
                                    .color(TEXT_MUTED),
                            ),
                        );
                        ui.add_sized(
                            [col_commit, 20.0],
                            egui::Label::new(
                                RichText::new("Commit")
                                    .typo(Typo::Caption)
                                    .color(TEXT_MUTED),
                            ),
                        );
                        ui.add_sized(
                            [col_deployed, 20.0],
                            egui::Label::new(
                                RichText::new("Deployed")
                                    .typo(Typo::Caption)
                                    .color(TEXT_MUTED),
                            ),
                        );
                        ui.add_sized(
                            [col_trigger, 20.0],
                            egui::Label::new(
                                RichText::new("Trigger / User")
                                    .typo(Typo::Caption)
                                    .color(TEXT_MUTED),
                            ),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new("Actions")
                                    .typo(Typo::Caption)
                                    .color(TEXT_MUTED),
                            );
                        });
                    });

                    ui.add_space(6.0);
                    draw_divider(ui, CARD_BORDER);
                }

                if filtered.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        let msg = if is_project_view {
                            format!(
                                "No deployments recorded for '{}' yet.",
                                self.selected_env_key
                            )
                        } else {
                            "No deployments recorded yet.".to_string()
                        };
                        ui.label(RichText::new(msg).typo(Typo::Body).color(TEXT_MUTED));
                        ui.add_space(20.0);
                    });
                }

                if is_stacked_view {
                    self.render_history_mobile_rows(
                        ui,
                        &filtered,
                        visible_count,
                        project_filter,
                        &mut actions,
                    );
                } else {
                    let widths = DesktopColWidths {
                        col_status,
                        col_version,
                        col_branch,
                        col_commit,
                        col_deployed,
                        col_trigger,
                    };
                    self.render_history_desktop_rows(
                        ui,
                        &filtered,
                        visible_count,
                        project_filter,
                        &widths,
                        &mut actions,
                    );
                }
                if total_count > visible_count {
                    if visible_count > 0 {
                        draw_divider(ui, SUBTLE_BG);
                    }
                    ui.add_space(10.0);
                    ui.vertical_centered(|ui| {
                        let remaining = total_count - visible_count;
                        let next_chunk = remaining.min(10);
                        let btn_text =
                            format!("Show More (+{next_chunk} of {remaining} remaining)...");
                        let show_more_btn = egui::Button::new(
                            RichText::new(btn_text)
                                .typo(Typo::SmallMedium)
                                .color(PRIMARY_BLUE),
                        )
                        .fill(SUBTLE_BG)
                        .stroke(Stroke::new(1.0_f32, PRIMARY_BLUE.gamma_multiply(0.4)))
                        .corner_radius(6.0)
                        .min_size(Vec2::new(240.0, 32.0));

                        if ui
                            .add(show_more_btn)
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            self.history_visible_limit =
                                self.history_visible_limit.saturating_add(10);
                        }
                    });
                    ui.add_space(4.0);
                } else if total_count > 5 {
                    draw_divider(ui, SUBTLE_BG);
                    ui.add_space(8.0);
                    ui.vertical_centered(|ui| {
                        horizontal_row(ui, 24.0, |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Showing all {total_count} deployment records."
                                ))
                                .typo(Typo::Caption)
                                .color(TEXT_MUTED),
                            );
                            let collapse_btn = egui::Button::new(
                                RichText::new("Collapse to Last 5")
                                    .typo(Typo::Caption)
                                    .color(TEXT_SECONDARY),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE);
                            if ui
                                .add(collapse_btn)
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.history_visible_limit = 5;
                            }
                        });
                    });
                    ui.add_space(4.0);
                }
            });

        if let Some(cancel_id) = actions.cancel_target {
            self.cancel_deployment(&cancel_id);
        } else if let Some(dep) = actions.delete_target {
            self.pending_delete_confirm = Some(ConfirmDeleteAction::DeploymentRecord {
                deployment_id: dep.id,
                project_name: dep.project_name,
                env_name: dep.environment,
                version: dep.version,
            });
        } else if let Some((target, is_rb)) = actions.redeploy_target {
            self.trigger_redeploy_from_record(&target, is_rb);
        }

        ui.add_space(18.0);

        let selected_dep = self
            .selected_deployment_id
            .as_ref()
            .and_then(|id| filtered.iter().find(|d| &d.id == id))
            .or_else(|| filtered.first())
            .cloned();

        if let Some(dep) = &selected_dep {
            if self.selected_deployment_id.as_ref() != Some(&dep.id) {
                self.selected_deployment_id = Some(dep.id.clone());
            }
        }

        // Code changes to display:
        // Strictly show code changes of the active project.
        // If a deployment of the active project is selected and has recorded git_changes, show them.
        // Otherwise, if viewing a project workspace, fall back to the active project's current git branch commits (cached_git_changes).
        let active_project_name = project_filter.and_then(|pid| {
            self.storage
                .projects
                .iter()
                .find(|p| p.id == pid)
                .map(|p| p.name.clone())
        });

        let code_changes_data: Option<Vec<String>> = if let Some(dep) = &selected_dep {
            if !dep.git_changes.is_empty() {
                Some(dep.git_changes.clone())
            } else if active_project_name.is_some() {
                if !self.cached_git_changes.is_empty() {
                    Some(self.cached_git_changes.clone())
                } else {
                    None
                }
            } else {
                None
            }
        } else if active_project_name.is_some() {
            if !self.cached_git_changes.is_empty() {
                Some(self.cached_git_changes.clone())
            } else {
                None
            }
        } else {
            None
        };

        if let Some(changes_lines) = code_changes_data {
            self.render_history_code_changes(ui, &changes_lines);
        }

        if let Some(dep) = selected_dep {
            self.render_terminal_window(ui, &dep);
        }
    }
}
