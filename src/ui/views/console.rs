//! Terminal log output viewer.

use super::*;

impl AppLoadApp {
    pub(crate) fn render_terminal_window(
        &mut self,
        ui: &mut egui::Ui,
        dep: &DeploymentRecord,
    ) -> Rect {
        let is_running =
            dep.status == DeploymentStatus::Running || self.is_deployment_active(&dep.id);
        let screen_h = ui.ctx().screen_rect().height();
        let expanded_h = (screen_h * 0.6).clamp(320.0, 720.0);
        let term_h = if self.console_expanded_height {
            expanded_h
        } else {
            320.0
        };

        #[cfg(debug_assertions)]
        let is_before_mode = app_env_is_ok("APPLOAD_CONSOLE_BEFORE");
        #[cfg(not(debug_assertions))]
        let is_before_mode = false;

        let frame_res = egui::Frame::new()
            .fill(SB_BG)
            .stroke(Stroke::new(1.0_f32, SB_BORDER_LINE))
            .corner_radius(CornerRadius::same(8))
            .shadow(Shadow {
                offset: [0, 4],
                blur: 12,
                spread: 0,
                color: Color32::from_black_alpha(25),
            })
            .show(ui, |ui| {
                let avail_w = ui.available_width();

                // Header (36px): "Deployment log" + controls
                let (top_rect, _) =
                    ui.allocate_exact_size(Vec2::new(avail_w, 36.0), Sense::hover());
                ui.painter().rect_filled(
                    top_rect,
                    CornerRadius {
                        nw: 8,
                        ne: 8,
                        sw: 0,
                        se: 0,
                    },
                    TERM_HEADER_BG,
                );

                if is_before_mode {
                    // Decorative macOS dots (before mode only)
                    let mut dot_x = top_rect.left() + 16.0;
                    for dot_color in [TERM_RED, TERM_YELLOW, TERM_GREEN] {
                        ui.painter().circle_filled(
                            Pos2::new(dot_x, top_rect.center().y),
                            4.5,
                            dot_color,
                        );
                        dot_x += 14.0;
                    }

                    // Title: Deployment log
                    ui.painter().text(
                        Pos2::new(dot_x + 8.0, top_rect.center().y),
                        Align2::LEFT_CENTER,
                        "Deployment log",
                        Typo::Label.font(),
                        Color32::WHITE,
                    );
                } else {
                    // Title: Deployment log
                    ui.painter().text(
                        Pos2::new(top_rect.left() + 16.0, top_rect.center().y),
                        Align2::LEFT_CENTER,
                        "Deployment log",
                        Typo::Label.font(),
                        Color32::WHITE,
                    );
                }

                // Right controls in header
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("term_header_controls")
                        .max_rect(top_rect.shrink2(Vec2::new(12.0, 0.0)))
                        .layout(Layout::right_to_left(Align::Center)),
                );

                if is_before_mode {
                    // Old height toggle
                    let h_btn = egui::Button::new(
                        RichText::new("Compact 260px")
                            .typo(Typo::Small)
                            .color(SB_TEXT_MUTED),
                    )
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE);
                    let _ = child.add(h_btn);

                    child.add_space(8.0);

                    // Old clear button
                    let clr_btn = egui::Button::new(
                        RichText::new("Clear")
                            .typo(Typo::Small)
                            .color(SB_TEXT_MUTED),
                    )
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE);
                    let _ = child.add(clr_btn);

                    child.add_space(8.0);
                } else {
                    // Close button
                    let close_resp = outline_icon_button(
                        &mut child,
                        "term_close_btn",
                        OutlineIcon::X,
                        None,
                        IconButtonVisuals {
                            fg: SB_TEXT_MUTED,
                            bg: Color32::TRANSPARENT,
                            hover_bg: SB_SURFACE_HOVER,
                            border: Color32::TRANSPARENT,
                        },
                        true,
                    )
                    .on_hover_text("Close log console");
                    if close_resp.clicked() {
                        self.selected_deployment_id = None;
                        self.console_collapsed = true;
                    }

                    child.add_space(4.0);

                    // Expand / Collapse height toggle
                    let (expand_icon, expand_tooltip) = if self.console_expanded_height {
                        (OutlineIcon::ChevronDown, "Shrink log")
                    } else {
                        (OutlineIcon::ChevronUp, "Expand log")
                    };
                    let expand_resp = outline_icon_button(
                        &mut child,
                        "term_expand_btn",
                        expand_icon,
                        None,
                        IconButtonVisuals {
                            fg: SB_TEXT_MUTED,
                            bg: Color32::TRANSPARENT,
                            hover_bg: SB_SURFACE_HOVER,
                            border: Color32::TRANSPARENT,
                        },
                        true,
                    )
                    .on_hover_text(expand_tooltip);
                    if expand_resp.clicked() {
                        self.console_expanded_height = !self.console_expanded_height;
                    }

                    child.add_space(8.0);
                }

                // Copy log button
                let copy_btn = egui::Button::new(
                    RichText::new("Copy log")
                        .typo(Typo::Small)
                        .color(Color32::WHITE),
                )
                .fill(SB_SURFACE_HOVER)
                .stroke(Stroke::new(1.0_f32, SB_BORDER_LINE))
                .corner_radius(CornerRadius::same(4));

                if child
                    .add(copy_btn)
                    .on_hover_text("Copy full log to clipboard")
                    .clicked()
                {
                    let mut full_log = String::new();
                    for step in &dep.step_logs {
                        full_log.push_str(&format!(
                            "[{}] {}\n",
                            step.status.label(),
                            step.step_name
                        ));
                        full_log.push_str(&format!("$ {}\n", step.command_summary));
                        full_log.push_str(&step.output);
                        full_log.push('\n');
                    }
                    child.ctx().copy_text(full_log);
                    self.set_status_banner("Copied deployment log to clipboard".to_string(), false);
                }

                child.add_space(8.0);

                // Running spinner or status pill
                if is_running {
                    child.ctx().request_repaint();
                    let time = child.input(|i| i.time);
                    let (spin_r, _) =
                        child.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
                    draw_circle_loading_spinner(
                        child.painter(),
                        spin_r.center(),
                        4.5,
                        SB_ACCENT,
                        time,
                    );
                    child.label(
                        RichText::new("Running...")
                            .typo(Typo::Small)
                            .color(SB_ACCENT),
                    );
                } else {
                    let (s_bg, s_fg) = status_pill_colors(dep.status);
                    render_pill(&mut child, dep.status.label(), s_bg, s_fg);
                }

                // Terminal Body: Monospace, auto-scroll when running, height fixed to term_h
                egui::Frame::new()
                    .fill(SB_BG)
                    .corner_radius(CornerRadius {
                        nw: 0,
                        ne: 0,
                        sw: 8,
                        se: 8,
                    })
                    .inner_margin(Margin::symmetric(16, 12))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());

                        let mut scroll = egui::ScrollArea::vertical().stick_to_bottom(is_running);
                        if is_before_mode {
                            scroll = scroll.max_height(260.0);
                        } else {
                            scroll = scroll
                                .min_scrolled_height(term_h)
                                .max_height(term_h)
                                .auto_shrink([false, false]);
                        }

                        let scroll_out = scroll.show(ui, |ui| {
                            if dep.step_logs.is_empty() {
                                ui.label(
                                    RichText::new("Initializing deployment pipeline...")
                                        .typo(Typo::Code)
                                        .color(SIDEBAR_TEXT_MUTED),
                                );
                            }

                            for (i, step_log) in dep.step_logs.iter().enumerate() {
                                let is_step_collapsed = self.collapsed_step_indices.contains(&i);
                                let color = match step_log.status {
                                    DeploymentStatus::Succeeded => TERM_GREEN,
                                    DeploymentStatus::Failed => TERM_RED,
                                    DeploymentStatus::Running => TERM_YELLOW,
                                    DeploymentStatus::Cancelled => SIDEBAR_TEXT_MUTED,
                                };

                                let step_resp = horizontal_row(ui, 24.0, |ui| {
                                    let (ic_r, _) = ui
                                        .allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
                                    if step_log.status == DeploymentStatus::Succeeded {
                                        draw_icon_check(ui.painter(), ic_r.center(), color);
                                    } else if step_log.status == DeploymentStatus::Running {
                                        ui.ctx().request_repaint();
                                        let time = ui.input(|i| i.time);
                                        draw_circle_loading_spinner(
                                            ui.painter(),
                                            ic_r.center(),
                                            4.5,
                                            color,
                                            time,
                                        );
                                    } else {
                                        ui.painter().circle_filled(ic_r.center(), 4.0, color);
                                    }

                                    ui.label(
                                        RichText::new(format!(
                                            "[{}/{}] {}",
                                            i + 1,
                                            dep.step_logs.len(),
                                            step_log.step_name
                                        ))
                                        .typo(Typo::SmallMedium)
                                        .color(color),
                                    );
                                    ui.label(
                                        RichText::new(format!("({} ms)", step_log.duration_ms))
                                            .typo(Typo::Code)
                                            .color(SB_TEXT_FAINT),
                                    );

                                    let (ch_r, _) = ui
                                        .allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
                                    draw_icon_chevron(
                                        ui.painter(),
                                        ch_r.center(),
                                        !is_step_collapsed,
                                        SB_TEXT_FAINT,
                                    );
                                })
                                .response
                                .interact(Sense::click())
                                .on_hover_cursor(egui::CursorIcon::PointingHand);

                                if step_resp
                                    .on_hover_text("Click to collapse or expand this step")
                                    .clicked()
                                {
                                    if is_step_collapsed {
                                        self.collapsed_step_indices.remove(&i);
                                    } else {
                                        self.collapsed_step_indices.insert(i);
                                    }
                                }

                                if !is_step_collapsed {
                                    ui.indent(format!("term_step_indent_{i}"), |ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "$ {}",
                                                step_log.command_summary
                                            ))
                                            .typo(Typo::Code)
                                            .color(SB_TEXT_FAINT),
                                        );
                                        for line in step_log.output.lines() {
                                            let trimmed = line.trim();
                                            let line_color = if trimmed.starts_with("[OK]")
                                                || trimmed.starts_with("Updated ")
                                            {
                                                TERM_GREEN
                                            } else if trimmed.starts_with("[FAILED]")
                                                || trimmed.starts_with("[ERROR]")
                                            {
                                                TERM_RED
                                            } else {
                                                SB_TEXT
                                            };
                                            ui.label(
                                                RichText::new(trimmed)
                                                    .typo(Typo::Code)
                                                    .color(line_color),
                                            );
                                        }
                                    });
                                }
                                ui.add_space(8.0);
                            }

                            if is_running {
                                ui.add_space(4.0);
                                horizontal_row(ui, 24.0, |ui| {
                                    let (ic_r, _) = ui
                                        .allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
                                    ui.ctx().request_repaint();
                                    let time = ui.input(|i| i.time);
                                    draw_circle_loading_spinner(
                                        ui.painter(),
                                        ic_r.center(),
                                        4.5,
                                        SB_ACCENT,
                                        time,
                                    );
                                    let active_step_name = dep
                                        .step_logs
                                        .last()
                                        .map(|s| s.step_name.as_str())
                                        .unwrap_or("Running pipeline");
                                    ui.label(
                                        RichText::new(format!("Executing: {active_step_name}..."))
                                            .typo(Typo::Code)
                                            .color(SB_ACCENT),
                                    );
                                });
                            }
                        });
                        scroll_out.inner_rect
                    })
                    .inner
            });
        frame_res.inner
    }
}
