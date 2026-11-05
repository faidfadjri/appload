//! Project workspace container, tabs, and header.

use super::*;

impl ShiplyApp {
    pub(crate) fn render_project_workspace(&mut self, ui: &mut egui::Ui, project_id: &str) {
        if self.cached_project_id.as_deref() != Some(project_id) {
            self.refresh_selected_project_metadata(true);
        }

        let Some(proj_idx) = self
            .storage
            .projects
            .iter()
            .position(|p| p.id == project_id)
        else {
            ui.label("Project not found.");
            return;
        };

        let proj_name = self.storage.projects[proj_idx].name.clone();

        // If currently in the loading skeleton transition for switching active project:
        if self.project_switch_loading_id.as_deref() == Some(project_id) {
            if let Some(start) = self.project_switch_loading_start {
                if start.elapsed() < Duration::from_millis(300) {
                    render_project_workspace_skeleton(ui, &proj_name);
                    return;
                }
            }
            self.project_switch_loading_id = None;
            self.project_switch_loading_start = None;
        }

        if self.storage.projects[proj_idx]
            .get_env(&self.selected_env_key)
            .is_none()
        {
            self.selected_env_key = self.storage.projects[proj_idx]
                .environments
                .first()
                .map(|e| e.key.clone())
                .unwrap_or_default();
        }

        let proj_desc = self.storage.projects[proj_idx].description.clone();
        let proj_path = self.storage.projects[proj_idx].local_path.clone();
        let path_exists = PathBuf::from(&proj_path).exists();
        let avail_w = ui.available_width();

        // 1. Top Project Header Bar (Fixed 36px height — all right-side controls vertically centered at 32px height)
        let mut delete_project_clicked = false;
        let mut edit_project_clicked = false;
        let mut duplicate_project_clicked = false;
        let mut selected_branch_changed = None;

        horizontal_row(ui, 36.0, |ui| {
            let title_resp = ui.add(
                egui::Label::new(RichText::new(&proj_name).typo(Typo::H1).color(TEXT_PRIMARY))
                    .sense(Sense::click()),
            );
            title_resp.context_menu(|ui| {
                ui.set_min_width(180.0);
                ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);
                ui.spacing_mut().button_padding = Vec2::new(10.0, 6.0);

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("✎  Edit Project")
                                .typo(Typo::Label)
                                .color(TEXT_PRIMARY),
                        )
                        .fill(CARD_BG),
                    )
                    .clicked()
                {
                    edit_project_clicked = true;
                    ui.close_menu();
                }

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("⧉  Duplicate Project")
                                .typo(Typo::Label)
                                .color(TEXT_PRIMARY),
                        )
                        .fill(CARD_BG),
                    )
                    .clicked()
                {
                    duplicate_project_clicked = true;
                    ui.close_menu();
                }

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("↻  Refresh Metadata")
                                .typo(Typo::Label)
                                .color(TEXT_SECONDARY),
                        )
                        .fill(CARD_BG),
                    )
                    .clicked()
                {
                    self.refresh_selected_project_metadata(true);
                    ui.close_menu();
                }

                ui.separator();

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("🗑  Delete Project")
                                .typo(Typo::Label)
                                .color(FAIL_PILL_TEXT),
                        )
                        .fill(CARD_BG),
                    )
                    .clicked()
                {
                    delete_project_clicked = true;
                    ui.close_menu();
                }
            });

            if path_exists {
                status_badge(ui, StatusKind::Success, "Ready");
            } else {
                status_badge(ui, StatusKind::Danger, "Path missing");
            }

            let right_w = ui.available_width();
            ui.allocate_ui_with_layout(
                Vec2::new(right_w, 36.0),
                Layout::right_to_left(Align::Center),
                |ui| {
                    ui.set_min_height(36.0);
                    ui.set_max_height(36.0);
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

                    // 1. "⋯" Overflow Menu (furthest right)
                    let more_resp = outline_icon_button(
                        ui,
                        ("hdr_more_menu_btn", project_id),
                        OutlineIcon::MoreHorizontal,
                        None,
                        IconButtonVisuals::new(TEXT_MUTED, CARD_BG, SUBTLE_BG, CARD_BORDER),
                        true,
                    )
                    .on_hover_text("More actions");

                    more_resp.context_menu(|ui| {
                        ui.set_min_width(170.0);
                        ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);
                        ui.spacing_mut().button_padding = Vec2::new(10.0, 6.0);

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("⧉  Duplicate project")
                                        .typo(Typo::Label)
                                        .color(TEXT_PRIMARY),
                                )
                                .fill(CARD_BG),
                            )
                            .clicked()
                        {
                            duplicate_project_clicked = true;
                            ui.close_menu();
                        }

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("↻  Refresh metadata")
                                        .typo(Typo::Label)
                                        .color(TEXT_SECONDARY),
                                )
                                .fill(CARD_BG),
                            )
                            .clicked()
                        {
                            self.refresh_selected_project_metadata(true);
                            ui.close_menu();
                        }

                        ui.separator();

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("🗑  Delete project")
                                        .typo(Typo::Label)
                                        .color(DANGER),
                                )
                                .fill(CARD_BG),
                            )
                            .clicked()
                        {
                            delete_project_clicked = true;
                            ui.close_menu();
                        }
                    });

                    // 2. Secondary "Edit" Button
                    if outline_icon_button(
                        ui,
                        ("hdr_edit_proj", project_id),
                        OutlineIcon::Edit,
                        Some("Edit Project"),
                        IconButtonVisuals::new(TEXT_PRIMARY, CARD_BG, SUBTLE_BG, CARD_BORDER),
                        true,
                    )
                    .on_hover_text("Edit project settings")
                    .clicked()
                    {
                        edit_project_clicked = true;
                    }

                    // 3. Git Branch Selector
                    if !self.cached_git_branches.is_empty() {
                        let current_b = if self.selected_git_branch.is_empty() {
                            "main".to_string()
                        } else {
                            self.selected_git_branch.clone()
                        };
                        let commit_suffix = self
                            .cached_git_meta
                            .commit_short
                            .as_deref()
                            .map(|c| format!(" @ {c}"))
                            .unwrap_or_default();

                        ui.scope(|ui| {
                            ui.spacing_mut().interact_size.y = 32.0;
                            ui.spacing_mut().button_padding = Vec2::new(10.0, 6.0);
                            egui::ComboBox::from_id_salt("header_git_branch_selector")
                                .selected_text(
                                    RichText::new(format!("{current_b}{commit_suffix}"))
                                        .typo(Typo::SmallMedium)
                                        .color(TEXT_PRIMARY),
                                )
                                .width(160.0)
                                .show_ui(ui, |ui| {
                                    for branch in self.cached_git_branches.clone() {
                                        let is_sel = self.selected_git_branch == branch;
                                        if ui.selectable_label(is_sel, &branch).clicked() && !is_sel
                                        {
                                            selected_branch_changed = Some(branch);
                                        }
                                    }
                                });
                        });
                    }
                },
            );
        });

        if edit_project_clicked {
            self.open_edit_project_modal(proj_idx);
        }

        if duplicate_project_clicked {
            if let Some(new_proj) = self.storage.duplicate_project(project_id) {
                self.selection = SidebarSelection::Project(new_proj.id.clone());
                self.refresh_selected_project_metadata(true);
                self.status_banner = Some((
                    format!("Duplicated project successfully as '{}'", new_proj.name),
                    true,
                ));
            }
        }

        if delete_project_clicked {
            self.pending_delete_confirm = Some(ConfirmDeleteAction::Project {
                project_id: project_id.to_string(),
                project_name: proj_name.clone(),
            });
        }

        if let Some(new_branch) = selected_branch_changed {
            self.selected_git_branch = new_branch.clone();
            let path = PathBuf::from(&proj_path);
            match checkout_git_branch(&path, &new_branch) {
                Ok(msg) => {
                    self.refresh_selected_project_metadata(false);
                    self.status_banner = Some((msg, true));
                }
                Err(err) => {
                    self.status_banner = Some((
                        format!("Selected branch '{new_branch}' for deploy ({err})"),
                        false,
                    ));
                }
            }
        }

        ui.add_space(8.0);
        draw_divider(ui, CARD_BORDER);
        ui.add_space(6.0);

        // 2. Sub-header description & folder path row
        horizontal_row(ui, 24.0, |ui| {
            if !proj_desc.is_empty() {
                ui.label(
                    RichText::new(&proj_desc)
                        .typo(Typo::Small)
                        .color(TEXT_SECONDARY),
                );
                ui.label(RichText::new("·").typo(Typo::Small).color(TEXT_MUTED));
            }

            let truncated_path = if proj_path.len() > 55 {
                let start = &proj_path[..24];
                let end = &proj_path[proj_path.len() - 26..];
                format!("{start}...{end}")
            } else {
                proj_path.clone()
            };

            let path_resp = ui
                .horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                    draw_icon_folder(
                        ui.painter(),
                        Pos2::new(ui.cursor().min.x + 6.0, ui.cursor().center().y),
                        TEXT_MUTED,
                    );
                    ui.add_space(14.0);
                    ui.label(
                        RichText::new(truncated_path)
                            .typo(Typo::Mono)
                            .color(TEXT_MUTED),
                    )
                })
                .response;

            if path_resp
                .interact(Sense::click())
                .on_hover_text(format!("Click to copy: {proj_path}"))
                .clicked()
            {
                ui.ctx().copy_text(proj_path.clone());
                self.set_status_banner(format!("Copied project path: {proj_path}"), false);
            }
        });

        ui.add_space(14.0);

        // 3. Environment Cards Grid — Responsive columns (3 columns fill workspace width >= 650px, 2 columns >= 440px, 1 column < 440px)
        let env_summaries: Vec<(String, String)> = self.storage.projects[proj_idx]
            .environments
            .iter()
            .map(|e| (e.key.clone(), e.name.clone()))
            .collect();

        let mut new_selected_env_key = None;
        let card_spacing = 14.0;
        let cols = if avail_w >= 650.0 {
            3
        } else if avail_w >= 440.0 {
            2
        } else {
            1
        };
        let card_w = if cols == 1 {
            avail_w
        } else {
            ((avail_w - card_spacing * (cols - 1) as f32) / cols as f32).floor()
        };

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(card_spacing, 14.0);

            for (env_key, env_name) in &env_summaries {
                let is_active = &self.selected_env_key == env_key;
                let last_ok = self.storage.last_succeeded_deployment(project_id, env_key);
                let latest_any = self.storage.latest_deployment(project_id, env_key);

                let (card_rect, card_resp) =
                    ui.allocate_exact_size(Vec2::new(card_w, ENV_CARD_HEIGHT), Sense::click());
                let card_resp = card_resp.on_hover_cursor(egui::CursorIcon::PointingHand);

                let (bg_fill, stroke) = if is_active {
                    (
                        Color32::from_rgb(240, 246, 255),
                        Stroke::new(1.5_f32, PRIMARY_BLUE),
                    )
                } else if card_resp.hovered() {
                    (CARD_BG, Stroke::new(1.0_f32, INPUT_BORDER_HOVER))
                } else {
                    (CARD_BG, Stroke::new(1.0_f32, CARD_BORDER))
                };

                ui.painter()
                    .rect_filled(card_rect, CornerRadius::same(10), bg_fill);
                ui.painter().rect_stroke(
                    card_rect,
                    CornerRadius::same(10),
                    stroke,
                    StrokeKind::Inside,
                );

                // Top row: Environment Name as H3 (sentence case)
                ui.painter().text(
                    Pos2::new(card_rect.left() + 16.0, card_rect.top() + 20.0),
                    Align2::LEFT_CENTER,
                    env_name,
                    Typo::H3.font(),
                    TEXT_PRIMARY,
                );

                // Top-right: Status badge ("Succeeded", "Running", "Failed", "Not deployed")
                let right_badge_pos = Pos2::new(card_rect.right() - 16.0, card_rect.top() + 20.0);
                if let Some(latest) = latest_any {
                    let (s_bg, s_fg) = status_pill_colors(latest.status);
                    let label = latest.status.label();
                    if latest.status == DeploymentStatus::Running {
                        ui.ctx().request_repaint();
                        let time = ui.input(|i| i.time);
                        paint_running_pill_right_aligned(
                            ui.painter(),
                            right_badge_pos,
                            label,
                            s_bg,
                            s_fg,
                            time,
                        );
                    } else {
                        paint_pill_right_aligned(ui.painter(), right_badge_pos, label, s_bg, s_fg);
                    }
                } else {
                    ui.painter().text(
                        right_badge_pos,
                        Align2::RIGHT_CENTER,
                        "Not deployed",
                        Typo::Caption.font(),
                        TEXT_MUTED,
                    );
                }

                // Middle: Version in Typo::MonoLg (20px mono-medium)
                if let Some(ok_dep) = &last_ok {
                    ui.painter().text(
                        Pos2::new(card_rect.left() + 16.0, card_rect.top() + 52.0),
                        Align2::LEFT_CENTER,
                        format!("v{}", ok_dep.version),
                        Typo::MonoLg.font(),
                        TEXT_PRIMARY,
                    );
                    ui.painter().text(
                        Pos2::new(card_rect.left() + 16.0, card_rect.top() + 74.0),
                        Align2::LEFT_CENTER,
                        "Current version",
                        Typo::Small.font(),
                        TEXT_MUTED,
                    );

                    // Bottom: "Deployed 2h ago · main@852f02f"
                    let rel_time = format_relative_time(ok_dep.started_at);
                    let git_str = match (&ok_dep.git_branch, &ok_dep.git_commit) {
                        (Some(b), Some(c)) => format!(" · {b}@{}", &c[..7.min(c.len())]),
                        (Some(b), None) => format!(" · {b}"),
                        _ => String::new(),
                    };
                    ui.painter().text(
                        Pos2::new(card_rect.left() + 16.0, card_rect.top() + 94.0),
                        Align2::LEFT_CENTER,
                        format!("Deployed {rel_time}{git_str}"),
                        Typo::Small.font(),
                        TEXT_MUTED,
                    );
                } else {
                    ui.painter().text(
                        Pos2::new(card_rect.left() + 16.0, card_rect.top() + 56.0),
                        Align2::LEFT_CENTER,
                        "No active build",
                        Typo::H3.font(),
                        TEXT_MUTED,
                    );
                }

                if card_resp.clicked() {
                    new_selected_env_key = Some(env_key.clone());
                }
            }

            // "Add environment" tile: same size (card_w x 112px)
            let (add_rect, add_resp) =
                ui.allocate_exact_size(Vec2::new(card_w, ENV_CARD_HEIGHT), Sense::click());
            let add_resp = add_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            let (add_bg, add_stroke) = if add_resp.hovered() {
                (CARD_BG, Stroke::new(1.0_f32, PRIMARY_BLUE))
            } else {
                (SUBTLE_BG, Stroke::new(1.0_f32, CARD_BORDER))
            };
            ui.painter()
                .rect_filled(add_rect, CornerRadius::same(10), add_bg);
            ui.painter().rect_stroke(
                add_rect,
                CornerRadius::same(10),
                add_stroke,
                StrokeKind::Inside,
            );

            let plus_center = Pos2::new(add_rect.center().x, add_rect.center().y - 18.0);
            draw_icon_plus(
                ui.painter(),
                plus_center,
                if add_resp.hovered() {
                    PRIMARY_BLUE
                } else {
                    TEXT_MUTED
                },
            );
            ui.painter().text(
                Pos2::new(add_rect.center().x, add_rect.center().y + 4.0),
                Align2::CENTER_CENTER,
                "Add environment",
                Typo::Label.font(),
                if add_resp.hovered() {
                    PRIMARY_BLUE
                } else {
                    TEXT_PRIMARY
                },
            );
            ui.painter().text(
                Pos2::new(add_rect.center().x, add_rect.center().y + 24.0),
                Align2::CENTER_CENTER,
                "or import from .ps1",
                Typo::Small.font(),
                TEXT_MUTED,
            );

            if add_resp.clicked() {
                self.new_env_clone_from = self.selected_env_key.clone();
                self.show_new_env_modal = true;
            }
        });

        if let Some(env_key) = new_selected_env_key {
            self.servers_edit_buffer = None;
            self.selected_env_key = env_key;
            self.refresh_selected_project_metadata(true);
        }

        if self.storage.projects[proj_idx].environments.is_empty() {
            ui.add_space(16.0);
            egui::Frame::new()
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0_f32, CARD_BORDER))
                .corner_radius(10.0)
                .inner_margin(Margin::symmetric(24, 24))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("No Environments in This Project Yet")
                                .typo(Typo::H3)
                                .color(TEXT_PRIMARY),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(
                                "Click 'ADD / IMPORT ENVIRONMENT' above to import a deploy-dev.ps1 / deploy-prod.ps1 script or add a custom environment.",
                            )
                            .typo(Typo::Body)
                            .color(TEXT_SECONDARY),
                        );
                    });
                });
            return;
        }

        ui.add_space(16.0);

        // 4. Version Control, Rollback & Deploy Action Card (Fixed 78px height)
        let active_env_name = self.storage.projects[proj_idx]
            .get_env(&self.selected_env_key)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| self.selected_env_key.clone());
        let rollback_candidate = self
            .storage
            .previous_rollback_candidate(project_id, &self.selected_env_key)
            .cloned();
        let mut sync_trigger_rect = None;
        self.render_deploy_action_bar(
            ui,
            &active_env_name,
            rollback_candidate.as_ref(),
            &mut sync_trigger_rect,
        );

        ui.add_space(18.0);

        // 5. Section Tabs: Underline tabs with 1px baseline CARD_BORDER across avail_w
        let total_steps = self.storage.projects[proj_idx]
            .get_env(&self.selected_env_key)
            .map(|e| e.steps.len())
            .unwrap_or(0);
        let history_count = self
            .storage
            .filter_deployments(Some(project_id), Some(&self.selected_env_key), None)
            .len();

        let tab_bar_y = ui.cursor().min.y;
        ui.painter().line_segment(
            [
                Pos2::new(ui.cursor().min.x, tab_bar_y + TAB_HEIGHT - 1.0),
                Pos2::new(ui.cursor().min.x + avail_w, tab_bar_y + TAB_HEIGHT - 1.0),
            ],
            Stroke::new(1.0_f32, CARD_BORDER),
        );

        horizontal_row(ui, TAB_HEIGHT, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(12.0, 0.0);
            for (tab, label, count) in [
                (
                    WorkspaceTab::PipelineSteps,
                    "Pipeline steps",
                    Some(total_steps),
                ),
                (
                    WorkspaceTab::HistoryAndLogs,
                    "Deployments",
                    Some(history_count),
                ),
                (WorkspaceTab::ServerSettings, "Settings", None),
            ] {
                let is_active = self.active_tab == tab;
                let resp = kit::underline_tab(ui, is_active, label, count);
                if resp.clicked() {
                    self.active_tab = tab;
                }
            }
        });

        ui.add_space(14.0);

        // 6. Render Active Full-Width Tab Content
        match self.active_tab {
            WorkspaceTab::PipelineSteps => {
                self.render_pipeline_steps_tab(ui, proj_idx);
            }
            WorkspaceTab::HistoryAndLogs => {
                let pid = self.storage.projects[proj_idx].id.clone();
                self.render_history_and_logs_view(ui, Some(&pid));
            }
            WorkspaceTab::ServerSettings => {
                self.render_server_settings_tab(ui, proj_idx);
            }
        }

        if let Some(r) = sync_trigger_rect {
            self.render_sync_version_dropdown(ui.ctx(), r, project_id);
        }
    }
}
