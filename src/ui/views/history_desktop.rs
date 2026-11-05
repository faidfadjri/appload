//! Desktop table rows for deployment history.

use super::*;

impl ShiplyApp {
    pub(crate) fn render_history_desktop_rows(
        &mut self,
        ui: &mut egui::Ui,
        filtered: &[DeploymentRecord],
        visible_count: usize,
        _project_filter: Option<&str>,
        widths: &DesktopColWidths,
        actions: &mut HistoryActions,
    ) {
        // Desktop table: Status: 90, Version: 80, Branch: 120, Commit: 70, Deployed: 140, Trigger: 110, Actions: remaining
        for (idx, dep) in filtered.iter().take(visible_count).enumerate() {
            if idx > 0 {
                draw_divider(ui, SUBTLE_BG);
            }
            ui.add_space(4.0);
            let is_selected = self.selected_deployment_id.as_deref() == Some(&dep.id);
            let row_h = 46.0;

            horizontal_row(ui, row_h, |ui| {
                // 1. Status (90px)
                let (s_rect, _) =
                    ui.allocate_exact_size(Vec2::new(widths.col_status, row_h), Sense::hover());
                let (s_bg, s_fg) = status_pill_colors(dep.status);
                let s_pos = Pos2::new(s_rect.left(), s_rect.center().y);
                if dep.status == DeploymentStatus::Running {
                    ui.ctx().request_repaint();
                    let time = ui.input(|i| i.time);
                    paint_running_pill_left_aligned(
                        ui.painter(),
                        s_pos,
                        dep.status.label(),
                        s_bg,
                        s_fg,
                        time,
                    );
                } else {
                    paint_pill_left_aligned(ui.painter(), s_pos, dep.status.label(), s_bg, s_fg);
                }

                // 2. Version (75px, MONO)
                let (v_rect, _) =
                    ui.allocate_exact_size(Vec2::new(widths.col_version, row_h), Sense::hover());
                ui.painter().text(
                    Pos2::new(v_rect.left(), v_rect.center().y),
                    Align2::LEFT_CENTER,
                    format!("v{}", dep.version),
                    Typo::Mono.font(),
                    TEXT_PRIMARY,
                );

                // 3. Changes column: Commit message in BODY (truncated) + branch @ commit in MonoSm muted
                let (ch_rect, ch_resp) =
                    ui.allocate_exact_size(Vec2::new(widths.col_changes, row_h), Sense::hover());
                let raw_branch = dep.git_branch.as_deref().unwrap_or("main");
                let full_sha = dep.git_commit.as_deref().unwrap_or("-");
                let short_sha = if full_sha.len() > 7 {
                    &full_sha[..7]
                } else {
                    full_sha
                };

                let commit_msg = if let Some(first_change) = dep.git_changes.iter().find(|l| {
                    !l.starts_with("M ")
                        && !l.starts_with("A ")
                        && !l.starts_with("D ")
                        && !l.starts_with("?? ")
                        && !l.starts_with("? ")
                }) {
                    if let Some(space_pos) = first_change.find(' ') {
                        let rest = &first_change[space_pos + 1..];
                        if let Some(paren_pos) = rest.rfind('(') {
                            rest[..paren_pos].trim()
                        } else {
                            rest.trim()
                        }
                    } else {
                        first_change.as_str()
                    }
                } else if dep.is_rollback {
                    "Rollback deployment"
                } else {
                    "Deploy release"
                };

                let meta_line = format!("{raw_branch} @ {short_sha}");
                ch_resp.on_hover_text(format!("{commit_msg}\n{meta_line}"));

                let ch_painter = ui.painter().with_clip_rect(ch_rect);
                ch_painter.text(
                    Pos2::new(ch_rect.left(), ch_rect.top() + 15.0),
                    Align2::LEFT_CENTER,
                    commit_msg,
                    Typo::Body.font(),
                    TEXT_PRIMARY,
                );
                ch_painter.text(
                    Pos2::new(ch_rect.left(), ch_rect.top() + 33.0),
                    Align2::LEFT_CENTER,
                    meta_line,
                    Typo::MonoSm.font(),
                    TEXT_MUTED,
                );

                // 4. Deployed (130px, relative time + duration below + full timestamp tooltip)
                let (t_rect, t_resp) =
                    ui.allocate_exact_size(Vec2::new(widths.col_deployed, row_h), Sense::hover());
                let rel_time = helpers::format_relative_time(dep.started_at);
                let dur_str = if dep.status == DeploymentStatus::Running {
                    "running...".to_string()
                } else if let Some(ms) = dep.duration_ms {
                    format!("took {:.1}s", ms as f64 / 1000.0)
                } else {
                    "—".to_string()
                };

                let t_painter = ui.painter().with_clip_rect(t_rect);
                t_painter.text(
                    Pos2::new(t_rect.left(), t_rect.top() + 15.0),
                    Align2::LEFT_CENTER,
                    rel_time,
                    Typo::Small.font(),
                    TEXT_SECONDARY,
                );
                t_painter.text(
                    Pos2::new(t_rect.left(), t_rect.top() + 33.0),
                    Align2::LEFT_CENTER,
                    dur_str,
                    Typo::Small.font(),
                    TEXT_MUTED,
                );

                let mut hover_msg = format!(
                    "Started: {}",
                    dep.format_started_at_local("%Y-%m-%d %H:%M:%S %:z")
                );
                if let Some(fin) = dep.format_finished_at_local("%Y-%m-%d %H:%M:%S %:z") {
                    hover_msg.push_str(&format!("\nFinished: {fin}"));
                }
                if let Some(ms) = dep.duration_ms {
                    hover_msg.push_str(&format!("\nDuration: {:.1}s", ms as f64 / 1000.0));
                }
                t_resp.on_hover_text(hover_msg);

                // 5. Trigger / User (only when show_trigger is true)
                if widths.show_trigger {
                    let (u_rect, _) = ui
                        .allocate_exact_size(Vec2::new(widths.col_trigger, row_h), Sense::hover());
                    let trigger_lbl = if dep.is_rollback {
                        "Rollback"
                    } else {
                        "Manual"
                    };
                    ui.painter().text(
                        Pos2::new(u_rect.left(), u_rect.center().y),
                        Align2::LEFT_CENTER,
                        trigger_lbl,
                        Typo::Small.font(),
                        TEXT_MUTED,
                    );
                }

                // 7. Actions (remaining width, right-aligned)
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
                    let is_busy = self.is_env_deploying(&dep.project_id, &dep.environment);
                    let can_delete = dep.status != DeploymentStatus::Running;

                    // Row overflow menu "⋯"
                    let row_more_id = ui.make_persistent_id(("row_more_menu", &dep.id));
                    let row_more_resp = outline_icon_button(
                        ui,
                        ("row_more_btn", &dep.id),
                        OutlineIcon::MoreHorizontal,
                        None,
                        IconButtonVisuals::new(TEXT_PRIMARY, CARD_BG, SUBTLE_BG, CARD_BORDER),
                        true,
                    )
                    .on_hover_text("Row actions");

                    if row_more_resp.clicked() {
                        ui.memory_mut(|m| m.toggle_popup(row_more_id));
                    }

                    egui::popup_below_widget(
                        ui,
                        row_more_id,
                        &row_more_resp,
                        egui::PopupCloseBehavior::CloseOnClick,
                        |ui| {
                            ui.set_min_width(140.0);
                            if ui.button("Copy version").clicked() {
                                ui.ctx().copy_text(dep.version.clone());
                                ui.memory_mut(|m| m.close_popup());
                            }
                            let del_btn = egui::Button::new(
                                RichText::new("Delete record")
                                    .typo(Typo::SmallMedium)
                                    .color(if can_delete {
                                        FAIL_PILL_TEXT
                                    } else {
                                        TEXT_PLACEHOLDER
                                    }),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE);
                            if ui.add_enabled(can_delete, del_btn).clicked() {
                                actions.delete_target = Some(dep.clone());
                                ui.memory_mut(|m| m.close_popup());
                            }
                        },
                    );

                    // Rollback / Redeploy button
                    if dep.status == DeploymentStatus::Succeeded {
                        if outline_icon_button(
                            ui,
                            ("hist_rollback_btn", &dep.id),
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
                    } else {
                        if outline_icon_button(
                            ui,
                            ("hist_rerun_btn", &dep.id),
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
                    }

                    if dep.status == DeploymentStatus::Running
                        && outline_icon_button(
                            ui,
                            ("hist_cancel_btn", &dep.id),
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

                    // View log button
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
            ui.add_space(4.0);
        }
    }
}
