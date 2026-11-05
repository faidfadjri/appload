//! Pipeline step configuration editor.

use super::*;

impl ShiplyApp {
    /// Feature #6: Redesigned Pipeline Steps Tab with non-overlapping 2-row step cards!
    pub(crate) fn render_pipeline_steps_tab(&mut self, ui: &mut egui::Ui, proj_idx: usize) {
        let env_key = self.selected_env_key.clone();
        let mut config_changed = false;

        let env_name = self.storage.projects[proj_idx]
            .get_env(&env_key)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| env_key.clone());

        let total_steps = self.storage.projects[proj_idx]
            .get_env(&env_key)
            .map(|e| e.steps.len())
            .unwrap_or(0);

        self.render_pipeline_header(
            ui,
            proj_idx,
            &env_name,
            &env_key,
            total_steps,
            &mut config_changed,
        );
        ui.add_space(8.0);
        self.render_pipeline_variables_row(ui);
        ui.add_space(12.0);

        let project = &mut self.storage.projects[proj_idx];
        let Some(env_cfg) = project.get_env_mut(&env_key) else {
            return;
        };

        let mut move_up_idx = None;
        let mut move_down_idx = None;
        let mut delete_idx = None;
        let mut drag_reorder = None;
        let total_steps = env_cfg.steps.len();

        let pointer_pos = ui.input(|i| i.pointer.hover_pos());
        let is_pointer_released = ui.input(|i| i.pointer.any_released());

        if self.dragged_step_idx.is_some() {
            ui.ctx().request_repaint();
        }

        for (idx, step) in env_cfg.steps.iter_mut().enumerate() {
            let is_being_dragged = self.dragged_step_idx == Some(idx);
            let card_fill = if is_being_dragged {
                BLUE_PILL_BG
            } else if !step.enabled {
                SUBTLE_BG
            } else {
                CARD_BG
            };
            let step_border_color = if is_being_dragged {
                PRIMARY_BLUE
            } else {
                CARD_BORDER
            };
            let step_accent_color = match &step.kind {
                StepKind::LocalCommand { .. } => STEP_LOCAL_ACCENT,
                StepKind::VerifyLocalPath { .. } => STEP_VERIFY_ACCENT,
                StepKind::TransferFiles { .. } => STEP_SCP_ACCENT,
                StepKind::RemoteCommand { .. } => STEP_SSH_ACCENT,
            };

            let frame_resp = egui::Frame::new()
                .fill(card_fill)
                .stroke(Stroke::new(
                    if is_being_dragged { 2.0_f32 } else { 1.0_f32 },
                    step_border_color,
                ))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(16, 14))
                .show(ui, |ui| {
                    // Row 1: Header (36px)
                    // Left: Drag handle + Enable checkbox + Step # + Step-type tag with colored dot and chevron + Step name
                    // Right: Shell/options pills + Arrow Up / Down / Trash
                    horizontal_row(ui, 36.0, |ui| {
                        // Drag Handle
                        let handle_size = Vec2::new(18.0, 26.0);
                        let (handle_rect, handle_resp) =
                            ui.allocate_exact_size(handle_size, Sense::drag());
                        let handle_resp = handle_resp.on_hover_cursor(if is_being_dragged {
                            egui::CursorIcon::Grabbing
                        } else {
                            egui::CursorIcon::Grab
                        });

                        if handle_resp.hovered() || is_being_dragged {
                            ui.painter().rect_filled(
                                handle_rect,
                                CornerRadius::same(4),
                                if is_being_dragged {
                                    BLUE_PILL_BG
                                } else {
                                    SUBTLE_BG
                                },
                            );
                        }

                        if handle_resp.drag_started() || handle_resp.dragged() {
                            self.dragged_step_idx = Some(idx);
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                        }

                        let handle_color = if is_being_dragged || handle_resp.hovered() {
                            PRIMARY_BLUE
                        } else {
                            TEXT_MUTED
                        };
                        draw_outline_icon(
                            ui.painter(),
                            handle_rect.center(),
                            OutlineIcon::GripVertical,
                            handle_color,
                        );
                        handle_resp.on_hover_text("Drag to reorder this pipeline step");
                        ui.add_space(4.0);

                        // Checkbox
                        if ui.checkbox(&mut step.enabled, "").changed() {
                            config_changed = true;
                        }
                        ui.add_space(2.0);

                        // Step Number in MONO (e.g. 01)
                        ui.label(
                            RichText::new(format!("{:02}", idx + 1))
                                .typo(Typo::Mono)
                                .color(TEXT_MUTED),
                        );
                        ui.add_space(6.0);

                        // Step-type tag with colored dot and chevron
                        let (type_label, fg) = match &step.kind {
                            StepKind::LocalCommand { .. } => ("Local", BLUE_PILL_TEXT),
                            StepKind::VerifyLocalPath { .. } => ("Verify", WARN_PILL_TEXT),
                            StepKind::TransferFiles { .. } => ("SCP upload", PURPLE_PILL_TEXT),
                            StepKind::RemoteCommand { .. } => ("Remote SSH", TEAL_PILL_TEXT),
                        };

                        let (dot_rect, _) =
                            ui.allocate_exact_size(Vec2::new(10.0, 20.0), Sense::hover());
                        ui.painter()
                            .circle_filled(dot_rect.center(), 3.5, step_accent_color);
                        ui.add_space(2.0);

                        egui::ComboBox::from_id_salt(("step_kind_selector", &step.id))
                            .selected_text(
                                RichText::new(type_label).typo(Typo::SmallMedium).color(fg),
                            )
                            .width(110.0)
                            .show_ui(ui, |ui| {
                                let is_local = matches!(step.kind, StepKind::LocalCommand { .. });
                                if ui.selectable_label(is_local, "Local command").clicked()
                                    && !is_local
                                {
                                    step.kind = StepKind::LocalCommand {
                                        command: "npm run build".to_string(),
                                        allow_failure: false,
                                        shell: None,
                                    };
                                    config_changed = true;
                                }

                                let is_verify =
                                    matches!(step.kind, StepKind::VerifyLocalPath { .. });
                                if ui.selectable_label(is_verify, "Verify path").clicked()
                                    && !is_verify
                                {
                                    step.kind = StepKind::VerifyLocalPath {
                                        path: "./dist/server.js".to_string(),
                                    };
                                    config_changed = true;
                                }

                                let is_scp = matches!(step.kind, StepKind::TransferFiles { .. });
                                if ui.selectable_label(is_scp, "SCP transfer").clicked() && !is_scp
                                {
                                    step.kind = StepKind::TransferFiles {
                                        sources: vec![
                                            "./dist".to_string(),
                                            "./package.json".to_string(),
                                            "./package-lock.json".to_string(),
                                        ],
                                        stage_env_file: true,
                                    };
                                    config_changed = true;
                                }

                                let is_remote = matches!(step.kind, StepKind::RemoteCommand { .. });
                                if ui.selectable_label(is_remote, "Remote SSH").clicked()
                                    && !is_remote
                                {
                                    step.kind = StepKind::RemoteCommand {
                                        command: "pm2 restart {app_name} --update-env".to_string(),
                                    };
                                    config_changed = true;
                                }
                            });

                        ui.add_space(8.0);

                        // Step Name Input: inline editable, borderless until hover/focus
                        let rem_w = ui.available_width();
                        let reserved_for_opts_and_actions = match &step.kind {
                            StepKind::LocalCommand { .. } => {
                                if rem_w < 500.0 {
                                    280.0
                                } else {
                                    340.0
                                }
                            }
                            StepKind::TransferFiles { .. } => 240.0,
                            _ => 130.0,
                        };
                        let name_input_w =
                            (rem_w - reserved_for_opts_and_actions).clamp(100.0, 320.0);

                        let name_id = ui.make_persistent_id(("step_name_input", &step.id));
                        let has_focus = ui.memory(|m| m.has_focus(name_id));
                        let (outer_rect, outer_resp) =
                            ui.allocate_exact_size(Vec2::new(name_input_w, 28.0), Sense::click());
                        let is_hovered = outer_resp.hovered();
                        let bg = if has_focus {
                            INPUT_BG
                        } else if is_hovered {
                            SUBTLE_BG
                        } else {
                            Color32::TRANSPARENT
                        };
                        let stroke = if has_focus {
                            Stroke::new(1.0_f32, PRIMARY_BLUE)
                        } else if is_hovered {
                            Stroke::new(1.0_f32, INPUT_BORDER)
                        } else {
                            Stroke::NONE
                        };
                        ui.painter()
                            .rect_filled(outer_rect, CornerRadius::same(4), bg);
                        if stroke != Stroke::NONE {
                            ui.painter().rect_stroke(
                                outer_rect,
                                CornerRadius::same(4),
                                stroke,
                                StrokeKind::Inside,
                            );
                        }
                        let inner_rect = outer_rect.shrink2(Vec2::new(6.0, 2.0));
                        let mut child = ui.new_child(
                            egui::UiBuilder::new()
                                .id_salt(("step_name_child", &step.id))
                                .max_rect(inner_rect),
                        );
                        let resp = child.add(
                            egui::TextEdit::singleline(&mut step.name)
                                .id_salt(name_id)
                                .font(Typo::SmallMedium.font())
                                .frame(false)
                                .desired_width(inner_rect.width())
                                .hint_text(
                                    RichText::new("Step name")
                                        .typo(Typo::Small)
                                        .color(TEXT_PLACEHOLDER),
                                ),
                        );
                        if resp.changed() {
                            config_changed = true;
                        }

                        // Right: Shell / Options pills
                        match &mut step.kind {
                            StepKind::LocalCommand {
                                allow_failure,
                                shell,
                                ..
                            } => {
                                ui.add_space(6.0);
                                let current_shell = shell.unwrap_or_default();
                                egui::ComboBox::from_id_salt(("step_shell_selector", &step.id))
                                    .selected_text(
                                        RichText::new(current_shell.display_name())
                                            .typo(Typo::Caption)
                                            .color(TEXT_SECONDARY),
                                    )
                                    .width(88.0)
                                    .show_ui(ui, |ui| {
                                        for &s in LocalShell::all() {
                                            let is_selected =
                                                shell.map_or(s == current_shell, |cur| cur == s);
                                            if ui
                                                .selectable_label(is_selected, s.display_name())
                                                .clicked()
                                            {
                                                *shell = Some(s);
                                                config_changed = true;
                                            }
                                        }
                                    });

                                ui.add_space(6.0);
                                if ui
                                    .checkbox(
                                        allow_failure,
                                        RichText::new("Continue on error")
                                            .typo(Typo::Small)
                                            .color(TEXT_SECONDARY),
                                    )
                                    .changed()
                                {
                                    config_changed = true;
                                }
                            }
                            StepKind::TransferFiles { stage_env_file, .. } => {
                                ui.add_space(6.0);
                                if ui
                                    .checkbox(
                                        stage_env_file,
                                        RichText::new("Stage .env file")
                                            .typo(Typo::Small)
                                            .color(TEXT_SECONDARY),
                                    )
                                    .changed()
                                {
                                    config_changed = true;
                                }
                            }
                            _ => {}
                        }

                        // Right-aligned Reorder & Delete actions
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
                            if outline_icon_button(
                                ui,
                                ("step_del_btn", &step.id),
                                OutlineIcon::Trash,
                                None,
                                IconButtonVisuals::new(
                                    FAIL_PILL_TEXT,
                                    FAIL_PILL_BG,
                                    DANGER_SOFT,
                                    DANGER_SOFT,
                                ),
                                true,
                            )
                            .on_hover_text("Delete step")
                            .clicked()
                            {
                                delete_idx = Some(idx);
                            }

                            let can_down = idx + 1 < total_steps;
                            if outline_icon_button(
                                ui,
                                ("step_down_btn", &step.id),
                                OutlineIcon::ArrowDown,
                                None,
                                IconButtonVisuals::new(
                                    if can_down {
                                        TEXT_SECONDARY
                                    } else {
                                        TEXT_PLACEHOLDER
                                    },
                                    SUBTLE_BG,
                                    INPUT_BG_HOVER,
                                    CARD_BORDER,
                                ),
                                can_down,
                            )
                            .on_hover_text("Move step down")
                            .clicked()
                                && can_down
                            {
                                move_down_idx = Some(idx);
                            }

                            let can_up = idx > 0;
                            if outline_icon_button(
                                ui,
                                ("step_up_btn", &step.id),
                                OutlineIcon::ArrowUp,
                                None,
                                IconButtonVisuals::new(
                                    if can_up {
                                        TEXT_SECONDARY
                                    } else {
                                        TEXT_PLACEHOLDER
                                    },
                                    SUBTLE_BG,
                                    INPUT_BG_HOVER,
                                    CARD_BORDER,
                                ),
                                can_up,
                            )
                            .on_hover_text("Move step up")
                            .clicked()
                                && can_up
                            {
                                move_up_idx = Some(idx);
                            }
                        });
                    });

                    ui.add_space(10.0);

                    // Row 2: Form labels above inputs, full-width CODE input, SCP in 2-column grid
                    match &mut step.kind {
                        StepKind::TransferFiles { sources, .. } => {
                            ui.columns(2, |cols| {
                                cols[0].vertical(|ui| {
                                    ui.label(
                                        RichText::new("Local source files / directories:")
                                            .typo(Typo::SmallMedium)
                                            .color(TEXT_SECONDARY),
                                    );
                                    ui.add_space(4.0);
                                    let col_w = ui.available_width();
                                    let sources_text = match &mut self.step_scp_sources_buffer {
                                        Some((id, buf)) if id == &step.id => buf,
                                        _ => {
                                            let joined = sources.join(", ");
                                            let slot = self.step_scp_sources_buffer.insert((step.id.clone(), joined));
                                            &mut slot.1
                                        }
                                    };
                                    if syntax_highlighted_text_edit(
                                        ui,
                                        ("step_transfer_sources", &step.id),
                                        sources_text,
                                        col_w,
                                        Some("e.g. ./dist, ./package.json, ./package-lock.json"),
                                    ) {
                                        *sources = sources_text
                                            .split(',')
                                            .map(|s| s.trim().to_string())
                                            .filter(|s| !s.is_empty())
                                            .collect();
                                        config_changed = true;
                                    }
                                });

                                cols[1].vertical(|ui| {
                                    ui.label(
                                        RichText::new("Remote target directory:")
                                            .typo(Typo::SmallMedium)
                                            .color(TEXT_SECONDARY),
                                    );
                                    ui.add_space(4.0);
                                    let col_w = ui.available_width();
                                    egui::Frame::new()
                                        .fill(SUBTLE_BG)
                                        .corner_radius(CornerRadius::same(6))
                                        .stroke(Stroke::new(1.0_f32, CARD_BORDER))
                                        .inner_margin(Margin::symmetric(10, 8))
                                        .show(ui, |ui| {
                                            ui.set_min_width(col_w - 20.0);
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new("{remote_dir}")
                                                        .typo(Typo::Code)
                                                        .color(TEXT_PRIMARY),
                                                );
                                                ui.label(
                                                    RichText::new("— configured in Settings")
                                                        .typo(Typo::Small)
                                                        .color(TEXT_MUTED),
                                                );
                                            });
                                        });
                                });
                            });
                        }
                        StepKind::LocalCommand { command, .. } => {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new("Shell command:")
                                        .typo(Typo::SmallMedium)
                                        .color(TEXT_SECONDARY),
                                );
                                ui.add_space(4.0);
                                let input_w = ui.available_width();
                                if syntax_highlighted_text_edit(
                                    ui,
                                    ("step_local_cmd", &step.id),
                                    command,
                                    input_w,
                                    Some("e.g. npm run build"),
                                ) {
                                    config_changed = true;
                                }
                            });
                        }
                        StepKind::VerifyLocalPath { path } => {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new("Verify path guard:")
                                        .typo(Typo::SmallMedium)
                                        .color(TEXT_SECONDARY),
                                );
                                ui.add_space(4.0);
                                let input_w = ui.available_width();
                                if syntax_highlighted_text_edit(
                                    ui,
                                    ("step_verify_path", &step.id),
                                    path,
                                    input_w,
                                    Some("e.g. ./dist/server.js or ./dist"),
                                ) {
                                    config_changed = true;
                                }
                            });
                        }
                        StepKind::RemoteCommand { command } => {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new("Remote SSH command:")
                                        .typo(Typo::SmallMedium)
                                        .color(TEXT_SECONDARY),
                                );
                                ui.add_space(4.0);
                                let input_w = ui.available_width();
                                if syntax_highlighted_text_edit(
                                    ui,
                                    ("step_remote_cmd", &step.id),
                                    command,
                                    input_w,
                                    Some("e.g. pm2 restart {app_name} --update-env"),
                                ) {
                                    config_changed = true;
                                }
                            });
                        }
                    }
                });

            let card_rect = frame_resp.response.rect;
            let bar_rect = Rect::from_min_max(
                Pos2::new(card_rect.left(), card_rect.top() + 4.0),
                Pos2::new(card_rect.left() + 3.0, card_rect.bottom() - 4.0),
            );
            ui.painter().rect_filled(
                bar_rect,
                CornerRadius {
                    nw: 0,
                    ne: 3,
                    se: 3,
                    sw: 0,
                },
                if step.enabled {
                    step_accent_color
                } else {
                    TEXT_MUTED
                },
            );
            if is_being_dragged {
                let shadow_rect = card_rect.translate(Vec2::new(0.0, 4.0)).expand(1.0);
                ui.painter()
                    .rect_filled(shadow_rect, CornerRadius::same(8), OVERLAY_SCRIM_LIGHT);
            }
            if let Some(from_idx) = self.dragged_step_idx {
                if from_idx != idx {
                    if let Some(pos) = pointer_pos {
                        if card_rect.contains(pos) {
                            let is_top_half = pos.y < card_rect.center().y;
                            let line_y = if is_top_half {
                                card_rect.top() - 3.0
                            } else {
                                card_rect.bottom() + 3.0
                            };
                            let insert_line = [
                                Pos2::new(card_rect.left() + 6.0, line_y),
                                Pos2::new(card_rect.right() - 6.0, line_y),
                            ];
                            // Animated drop insertion glow & line
                            ui.painter().line_segment(
                                insert_line,
                                Stroke::new(5.0_f32, PRIMARY_BLUE.gamma_multiply(0.4)),
                            );
                            ui.painter()
                                .line_segment(insert_line, Stroke::new(2.5_f32, PRIMARY_BLUE));
                            ui.painter().circle_filled(
                                Pos2::new(card_rect.left() + 6.0, line_y),
                                4.0,
                                PRIMARY_BLUE,
                            );
                            ui.painter().rect_filled(
                                card_rect,
                                CornerRadius::same(8),
                                PRIMARY_BLUE.gamma_multiply(0.08),
                            );
                            if is_pointer_released {
                                drag_reorder = Some((from_idx, idx));
                            }
                        }
                    }
                }
            }

            ui.add_space(8.0);
        }

        if let Some((from, to)) = drag_reorder {
            env_cfg.reorder_step(from, to);
            self.dragged_step_idx = None;
            config_changed = true;
        } else if is_pointer_released {
            self.dragged_step_idx = None;
        }

        if let Some(i) = move_up_idx {
            env_cfg.move_step_up(i);
            config_changed = true;
        }
        if let Some(i) = move_down_idx {
            env_cfg.move_step_down(i);
            config_changed = true;
        }
        if let Some(i) = delete_idx {
            env_cfg.steps.remove(i);
            config_changed = true;
        }

        if config_changed {
            self.mark_config_dirty();
        }
    }
}
