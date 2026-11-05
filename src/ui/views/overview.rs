//! All deployments overview dashboard.

use super::*;

impl ShiplyApp {
    pub(crate) fn render_all_deployments_overview(&mut self, ui: &mut egui::Ui) {
        let mut open_bulk_modal = false;
        let mut clear_selection = false;
        let all_project_ids: Vec<String> =
            self.storage.projects.iter().map(|p| p.id.clone()).collect();
        let selected_count = all_project_ids
            .iter()
            .filter(|id| self.bulk_selected_projects.contains(*id))
            .count();

        // Page Header
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(Typo::H1.rich("All deployments"));
                    if selected_count > 0 {
                        ui.add_space(SP_8);
                        kit::count_badge(ui, selected_count);
                        ui.label(Typo::Small.rich("selected").color(PRIMARY_BLUE));
                    }
                });
                ui.add_space(2.0);
                ui.label(
                    Typo::Small
                        .rich("Select projects to deploy together, or open one to manage it.")
                        .color(TEXT_SECONDARY),
                );
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(SP_8, 0.0);

                let deploy_label = if selected_count > 0 {
                    format!("Deploy selected ({selected_count})")
                } else {
                    "Bulk deploy...".to_string()
                };
                if kit::button(
                    ui,
                    ButtonVariant::Primary,
                    ButtonSize::Md,
                    &deploy_label,
                    Some(OutlineIcon::Play),
                )
                .on_hover_text(
                    "Deploy multiple selected projects sequentially with auto-bumped versions",
                )
                .clicked()
                {
                    open_bulk_modal = true;
                }

                if selected_count > 0
                    && kit::button(
                        ui,
                        ButtonVariant::Ghost,
                        ButtonSize::Sm,
                        "Clear selection",
                        None,
                    )
                    .on_hover_text("Deselect all selected projects")
                    .clicked()
                {
                    clear_selection = true;
                }
            });
        });

        if clear_selection {
            self.bulk_selected_projects.clear();
        }
        if open_bulk_modal {
            self.open_bulk_deploy_modal();
        }

        ui.add_space(SP_16);

        // Live Bulk Deployment Queue Progress Banner (when a bulk deployment is running)
        if !self.bulk_deploy_queue.is_empty()
            || (self.bulk_total_count > 0 && self.has_active_deployments())
        {
            let remaining = self.bulk_deploy_queue.len();
            let current_idx = self.bulk_total_count.saturating_sub(remaining);
            let total = self.bulk_total_count.max(1);
            let mut cancel_queue = false;

            egui::Frame::new()
                .fill(INFO_SOFT)
                .stroke(Stroke::new(1.0_f32, PRIMARY_BLUE))
                .corner_radius(CornerRadius::same(R_MD as u8))
                .inner_margin(Margin::symmetric(16, 10))
                .show(ui, |ui| {
                    horizontal_row(ui, 30.0, |ui| {
                        ui.ctx().request_repaint();
                        let time = ui.input(|i| i.time);
                        let (sp_rect, _) =
                            ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
                        draw_circle_loading_spinner(
                            ui.painter(),
                            sp_rect.center(),
                            5.0,
                            PRIMARY_BLUE,
                            time,
                        );
                        ui.add_space(4.0);
                        ui.label(
                            Typo::SmallMedium
                                .rich(format!(
                                    "Bulk deployment in progress — Running {current_idx} of {total} ({remaining} queued)"
                                ))
                                .color(PRIMARY_BLUE),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if remaining > 0
                                && kit::button(
                                    ui,
                                    ButtonVariant::Danger,
                                    ButtonSize::Sm,
                                    "Cancel queue",
                                    Some(OutlineIcon::Trash),
                                )
                                .clicked()
                            {
                                cancel_queue = true;
                            }
                        });
                    });
                });
            if cancel_queue {
                self.bulk_deploy_queue.clear();
                self.bulk_total_count = 0;
                self.status_banner = Some((
                    "Cleared remaining projects from the Bulk Deployment queue.".to_string(),
                    false,
                ));
            }
            ui.add_space(SP_12);
        }

        // Project Cards Grid
        let proj_count = self.storage.projects.len().max(1);
        let spacing = SP_16;
        let grid_avail_w = ui.available_width();
        let max_cols = if grid_avail_w >= 1000.0 {
            3
        } else if grid_avail_w >= 640.0 {
            2
        } else {
            1
        };
        let cols = proj_count.min(max_cols).max(1);
        let card_w = if cols == 1 {
            grid_avail_w
        } else {
            ((grid_avail_w - spacing * (cols - 1) as f32) / cols as f32).floor()
        };

        let mut clicked_proj = None;
        let mut toggled_bulk_proj = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(spacing, SP_16);
            for project in &self.storage.projects {
                let is_checked = self.bulk_selected_projects.contains(&project.id);
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(card_w, 112.0), Sense::click());
                let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

                // Checkbox hit target in top-right of the project card
                let chk_rect = Rect::from_center_size(
                    Pos2::new(rect.right() - 24.0, rect.top() + 24.0),
                    Vec2::new(20.0, 20.0),
                );
                let chk_resp = ui
                    .interact(
                        chk_rect,
                        egui::Id::new(("bulk_chk_card", &project.id)),
                        Sense::click(),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand);

                // Card frame
                let card_bg = if is_checked {
                    Color32::from_rgba_premultiplied(20, 110, 245, 12)
                } else {
                    CARD_BG
                };
                let border_stroke = Stroke::new(
                    if is_checked { 1.5_f32 } else { 1.0_f32 },
                    if is_checked {
                        PRIMARY_BLUE
                    } else if resp.hovered() {
                        INPUT_BORDER_HOVER
                    } else {
                        CARD_BORDER
                    },
                );

                ui.painter()
                    .rect_filled(rect, CornerRadius::same(R_LG as u8), card_bg);
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(R_LG as u8),
                    border_stroke,
                    StrokeKind::Inside,
                );

                // Checkbox paint
                let chk_radius = CornerRadius::same(4);
                if is_checked {
                    ui.painter().rect_filled(chk_rect, chk_radius, PRIMARY_BLUE);
                    let p1 = Pos2::new(chk_rect.left() + 4.5, chk_rect.top() + 10.0);
                    let p2 = Pos2::new(chk_rect.left() + 8.0, chk_rect.top() + 14.0);
                    let p3 = Pos2::new(chk_rect.left() + 15.5, chk_rect.top() + 6.0);
                    ui.painter()
                        .line_segment([p1, p2], Stroke::new(1.8_f32, Color32::WHITE));
                    ui.painter()
                        .line_segment([p2, p3], Stroke::new(1.8_f32, Color32::WHITE));
                } else {
                    let chk_border = if chk_resp.hovered() {
                        INPUT_BORDER_HOVER
                    } else {
                        INPUT_BORDER
                    };
                    ui.painter().rect_filled(chk_rect, chk_radius, CARD_BG);
                    ui.painter().rect_stroke(
                        chk_rect,
                        chk_radius,
                        Stroke::new(1.0_f32, chk_border),
                        StrokeKind::Inside,
                    );
                }

                // Project Name and Path
                let text_clip = Rect::from_min_max(
                    Pos2::new(rect.left() + 16.0, rect.top() + 10.0),
                    Pos2::new(
                        (chk_rect.left() - 8.0).max(rect.left() + 40.0),
                        rect.top() + 66.0,
                    ),
                );
                let clipped_painter = ui.painter().with_clip_rect(text_clip);
                clipped_painter.text(
                    Pos2::new(rect.left() + 18.0, rect.top() + 24.0),
                    Align2::LEFT_CENTER,
                    &project.name,
                    Typo::H3.font(),
                    TEXT_PRIMARY,
                );
                let local_path_display = if project.local_path.len() > 36 {
                    format!("{}…", &project.local_path[..35])
                } else {
                    project.local_path.clone()
                };
                clipped_painter.text(
                    Pos2::new(rect.left() + 18.0, rect.top() + 48.0),
                    Align2::LEFT_CENTER,
                    &local_path_display,
                    Typo::MonoSm.font(),
                    TEXT_MUTED,
                );

                // Environment version tags row at bottom
                let mut px = rect.left() + 18.0;
                let py = rect.top() + 80.0;
                for env in &project.environments {
                    let badge = env.badge_label();
                    let is_env_running = self.storage.deployments.iter().any(|d| {
                        d.project_id == project.id
                            && d.environment == env.key
                            && d.status == DeploymentStatus::Running
                    });
                    let w = if is_env_running {
                        ui.ctx().request_repaint();
                        let time = ui.input(|i| i.time);
                        paint_running_pill_left_aligned(
                            ui.painter(),
                            Pos2::new(px, py),
                            &format!("{badge}: Running"),
                            WARN_PILL_BG,
                            WARN_PILL_TEXT,
                            time,
                        )
                    } else {
                        let (bg, fg) = env_pastel_pill_colors(&env.key);
                        let ver_label = self
                            .storage
                            .last_succeeded_deployment(&project.id, &env.key)
                            .map(|d| format!("{badge} v{}", d.version))
                            .unwrap_or_else(|| format!("{badge} —"));
                        paint_pill_left_aligned(ui.painter(), Pos2::new(px, py), &ver_label, bg, fg)
                    };
                    px += w + 6.0;
                }

                if chk_resp
                    .on_hover_text("Select or deselect this project for bulk deployment")
                    .clicked()
                {
                    toggled_bulk_proj = Some(project.id.clone());
                } else if resp.clicked() {
                    clicked_proj = Some(project.id.clone());
                }
            }
        });

        if let Some(tid) = toggled_bulk_proj {
            if self.bulk_selected_projects.contains(&tid) {
                self.bulk_selected_projects.remove(&tid);
            } else {
                self.bulk_selected_projects.insert(tid);
            }
        } else if let Some(pid) = clicked_proj {
            self.selection = SidebarSelection::Project(pid.clone());
            self.project_switch_loading_id = Some(pid);
            self.project_switch_loading_start = Some(Instant::now());
            self.refresh_selected_project_metadata(true);
            return;
        }

        ui.add_space(SP_16);
        self.render_onboarding_guideline_card(ui);
        ui.add_space(SP_16);
        self.render_os_credentials_guideline_card(ui);
        ui.add_space(SP_20);
        self.render_history_and_logs_view(ui, None);
    }
}
