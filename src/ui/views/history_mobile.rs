//! Mobile stacked card view for deployment history.

use super::*;

impl AppLoadApp {
    pub(crate) fn render_history_mobile_rows(
        &mut self,
        ui: &mut egui::Ui,
        filtered: &[DeploymentRecord],
        visible_count: usize,
        _project_filter: Option<&str>,
        actions: &mut HistoryActions,
    ) {
        // Mobile / narrow stacked card view (< 720px)
        for (idx, dep) in filtered.iter().take(visible_count).enumerate() {
            if idx > 0 {
                draw_divider(ui, SUBTLE_BG);
            }
            ui.add_space(6.0);
            let is_selected = self.selected_deployment_id.as_deref() == Some(&dep.id);

            // Line 1: Status + Env + Version (left), Deployed relative time (right)
            horizontal_row(ui, 24.0, |ui| {
                let (s_rect, _) = ui.allocate_exact_size(Vec2::new(96.0, 22.0), Sense::hover());
                let (s_bg, s_fg) = status_pill_colors(dep.status);
                if dep.status == DeploymentStatus::Running {
                    ui.ctx().request_repaint();
                    let time = ui.input(|i| i.time);
                    paint_running_pill_left_aligned(
                        ui.painter(),
                        Pos2::new(s_rect.left(), s_rect.center().y),
                        dep.status.label(),
                        s_bg,
                        s_fg,
                        time,
                    );
                } else {
                    paint_pill_left_aligned(
                        ui.painter(),
                        Pos2::new(s_rect.left(), s_rect.center().y),
                        dep.status.label(),
                        s_bg,
                        s_fg,
                    );
                }

                let (_, env_fg) = env_pastel_pill_colors(&dep.environment);
                ui.label(
                    RichText::new(format_env_badge(&dep.environment))
                        .typo(Typo::SmallMedium)
                        .color(env_fg),
                );

                ui.label(
                    RichText::new(format!("v{}", dep.version))
                        .typo(Typo::Mono)
                        .color(TEXT_PRIMARY),
                );

                if dep.is_rollback {
                    ui.label(
                        RichText::new("Rollback")
                            .typo(Typo::Caption)
                            .color(WARN_PILL_TEXT),
                    );
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let rel = helpers::format_relative_time(dep.started_at);
                    let ts_resp = ui.label(RichText::new(rel).typo(Typo::Small).color(TEXT_MUTED));
                    let mut hover_msg = format!(
                        "Started (Local): {}",
                        dep.format_started_at_local("%Y-%m-%d %H:%M:%S %:z")
                    );
                    if let Some(fin) = dep.format_finished_at_local("%Y-%m-%d %H:%M:%S %:z") {
                        hover_msg.push_str(&format!("\nFinished (Local): {fin}"));
                    }
                    ts_resp.on_hover_text(hover_msg);
                });
            });

            ui.add_space(4.0);

            // Line 2: Git branch @ commit
            horizontal_row(ui, 20.0, |ui| {
                let (icon_r, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
                draw_icon_git_branch(
                    ui.painter(),
                    Pos2::new(icon_r.left(), icon_r.top() + 3.0),
                    TEXT_MUTED,
                );
                let git_line = match (&dep.git_branch, &dep.git_commit) {
                    (Some(b), Some(c)) => {
                        let short = if c.len() > 7 { &c[..7] } else { c };
                        format!("{b} @ {short}")
                    }
                    _ => dep.project_name.clone(),
                };
                ui.label(
                    RichText::new(git_line)
                        .typo(Typo::MonoSm)
                        .color(TEXT_SECONDARY),
                );
            });

            ui.add_space(6.0);

            // Line 3: Duration (left), Action buttons (right)
            horizontal_row(ui, 28.0, |ui| {
                let dur_str = dep
                    .duration_ms
                    .map(|ms| format!("took {:.1}s", ms as f64 / 1000.0))
                    .unwrap_or_else(|| "running...".to_string());
                ui.label(RichText::new(dur_str).typo(Typo::Small).color(TEXT_MUTED));

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
                    let is_busy = self.is_env_deploying(&dep.project_id, &dep.environment);
                    let can_delete = dep.status != DeploymentStatus::Running;

                    if outline_icon_button(
                        ui,
                        ("stacked_del_dep_btn", &dep.id),
                        OutlineIcon::Trash,
                        None,
                        IconButtonVisuals::new(
                            FAIL_PILL_TEXT,
                            SUBTLE_BG,
                            FAIL_PILL_BG,
                            CARD_BORDER,
                        ),
                        can_delete,
                    )
                    .on_hover_text("Delete this deployment record")
                    .clicked()
                        && can_delete
                    {
                        actions.delete_target = Some(dep.clone());
                    }

                    if outline_icon_button(
                        ui,
                        ("stacked_rerun_btn", &dep.id),
                        OutlineIcon::Refresh,
                        None,
                        IconButtonVisuals::new(
                            TEXT_SECONDARY,
                            SUBTLE_BG,
                            PILL_CONTAINER_BG,
                            CARD_BORDER,
                        ),
                        !is_busy,
                    )
                    .on_hover_text("Re-run this deployment")
                    .clicked()
                        && !is_busy
                    {
                        actions.redeploy_target = Some((dep.clone(), false));
                    }

                    if dep.status == DeploymentStatus::Running
                        && outline_icon_button(
                            ui,
                            ("stacked_cancel_btn", &dep.id),
                            OutlineIcon::X,
                            Some("Cancel"),
                            IconButtonVisuals::new(
                                FAIL_PILL_TEXT,
                                FAIL_PILL_BG,
                                DANGER_SOFT,
                                DANGER_SOFT,
                            ),
                            true,
                        )
                        .on_hover_text("Cancel this running deployment")
                        .clicked()
                    {
                        actions.cancel_target = Some(dep.id.clone());
                    }

                    if dep.status == DeploymentStatus::Succeeded
                        && outline_icon_button(
                            ui,
                            ("stacked_rollback_btn", &dep.id),
                            OutlineIcon::RotateCcw,
                            Some("Rollback"),
                            IconButtonVisuals::new(
                                WARN_PILL_TEXT,
                                WARN_PILL_BG,
                                WARNING_SOFT,
                                WARNING,
                            ),
                            !is_busy,
                        )
                        .on_hover_text(format!("Rollback {} to v{}", dep.environment, dep.version))
                        .clicked()
                        && !is_busy
                    {
                        actions.redeploy_target = Some((dep.clone(), true));
                    }

                    let logs_btn = egui::Button::new(
                        RichText::new(if is_selected {
                            "Viewing log"
                        } else {
                            "View log"
                        })
                        .typo(Typo::SmallMedium)
                        .color(if is_selected {
                            PRIMARY_BLUE
                        } else {
                            TEXT_PRIMARY
                        }),
                    )
                    .fill(if is_selected { BLUE_PILL_BG } else { SUBTLE_BG })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if is_selected {
                            PRIMARY_BLUE
                        } else {
                            CARD_BORDER
                        },
                    ))
                    .corner_radius(CornerRadius::same(6));

                    if ui.add(logs_btn).clicked() {
                        self.selected_deployment_id = Some(dep.id.clone());
                        self.console_collapsed = false;
                    }
                });
            });
            ui.add_space(6.0);
        }
    }
}
