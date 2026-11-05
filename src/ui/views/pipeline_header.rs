//! Pipeline header and variable chips row.

use super::*;

impl ShiplyApp {
    pub(crate) fn render_pipeline_header(
        &mut self,
        ui: &mut egui::Ui,
        proj_idx: usize,
        env_name: &str,
        env_key: &str,
        total_steps: usize,
        config_changed: &mut bool,
    ) {
        let mut add_local = false;
        let mut add_verify = false;
        let mut add_scp = false;
        let mut add_remote = false;

        let header_avail_w = ui.available_width();

        // 1. Section Header: H2 Typo::H2 "Pipeline steps" + count badge + subtitle, Right: single Primary Sm "+ Add step" menu button
        horizontal_row(ui, 36.0, |ui| {
            ui.label(
                RichText::new("Pipeline steps")
                    .typo(Typo::H2)
                    .color(TEXT_PRIMARY),
            );
            ui.add_space(8.0);
            kit::count_badge(ui, total_steps);
            ui.add_space(8.0);
            if header_avail_w >= 640.0 {
                ui.label(
                    RichText::new(format!("for {env_name} — Sequential deployment stages"))
                        .typo(Typo::Small)
                        .color(TEXT_MUTED),
                );
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let add_menu_id = ui.make_persistent_id("pipeline_add_step_menu");
                let add_btn = egui::Button::new(
                    RichText::new("+ Add step")
                        .typo(Typo::SmallMedium)
                        .color(Color32::WHITE),
                )
                .fill(PRIMARY_BLUE)
                .corner_radius(CornerRadius::same(6))
                .min_size(Vec2::new(96.0, 30.0));

                let add_resp = ui
                    .add(add_btn)
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if add_resp.clicked() {
                    ui.memory_mut(|m| m.toggle_popup(add_menu_id));
                }

                egui::popup_below_widget(
                    ui,
                    add_menu_id,
                    &add_resp,
                    egui::PopupCloseBehavior::CloseOnClick,
                    |ui| {
                        ui.set_min_width(220.0);
                        ui.spacing_mut().item_spacing = Vec2::new(0.0, 4.0);

                        for (lbl, desc, accent, trigger) in [
                            (
                                "Local shell command",
                                "Run build or test scripts locally",
                                STEP_LOCAL_ACCENT,
                                &mut add_local,
                            ),
                            (
                                "Verify path guard",
                                "Assert built file/folder exists",
                                STEP_VERIFY_ACCENT,
                                &mut add_verify,
                            ),
                            (
                                "SCP file transfer",
                                "Upload files & .env to remote server",
                                STEP_SCP_ACCENT,
                                &mut add_scp,
                            ),
                            (
                                "Remote SSH command",
                                "Execute restart/pm2 command on target",
                                STEP_SSH_ACCENT,
                                &mut add_remote,
                            ),
                        ] {
                            let item_resp = ui
                                .horizontal(|ui| {
                                    let (dot_rect, _) = ui
                                        .allocate_exact_size(Vec2::new(12.0, 24.0), Sense::hover());
                                    ui.painter().circle_filled(dot_rect.center(), 4.0, accent);
                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new(lbl)
                                                .typo(Typo::SmallMedium)
                                                .color(TEXT_PRIMARY),
                                        );
                                        ui.label(
                                            RichText::new(desc)
                                                .typo(Typo::Caption)
                                                .color(TEXT_MUTED),
                                        );
                                    });
                                })
                                .response;

                            let item_resp = ui
                                .interact(
                                    item_resp.rect,
                                    ui.make_persistent_id(lbl),
                                    Sense::click(),
                                )
                                .on_hover_cursor(egui::CursorIcon::PointingHand);
                            if item_resp.clicked() {
                                *trigger = true;
                                ui.memory_mut(|m| m.close_popup());
                            }
                        }
                    },
                );
            });
        });

        if add_local || add_verify || add_scp || add_remote {
            let project = &mut self.storage.projects[proj_idx];
            if let Some(env_cfg) = project.get_env_mut(env_key) {
                if add_local {
                    env_cfg.steps.push(PipelineStep::new(
                        "Build Application",
                        StepKind::LocalCommand {
                            command: "npm run build".to_string(),
                            allow_failure: false,
                            shell: None,
                        },
                    ));
                }
                if add_verify {
                    env_cfg.steps.push(PipelineStep::new(
                        "Verify Build Output",
                        StepKind::VerifyLocalPath {
                            path: "./dist/server.js".to_string(),
                        },
                    ));
                }
                if add_scp {
                    env_cfg.steps.push(PipelineStep::new(
                        "Upload Artifacts via SCP",
                        StepKind::TransferFiles {
                            sources: vec![
                                "./dist".to_string(),
                                "./package.json".to_string(),
                                "./package-lock.json".to_string(),
                            ],
                            stage_env_file: true,
                        },
                    ));
                }
                if add_remote {
                    env_cfg.steps.push(PipelineStep::new(
                        "Remote SSH Command",
                        StepKind::RemoteCommand {
                            command: "pm2 restart {app_name} --update-env".to_string(),
                        },
                    ));
                }
                *config_changed = true;
            }
        }
    }

    pub(crate) fn render_pipeline_variables_row(&mut self, ui: &mut egui::Ui) {
        horizontal_row(ui, 26.0, |ui| {
            ui.label(
                RichText::new("Variables:")
                    .typo(Typo::Small)
                    .color(TEXT_MUTED),
            );
            ui.add_space(6.0);
            for var_name in [
                "${{ version }}",
                "${{ branch }}",
                "${{ commit }}",
                "${{ timestamp }}",
                "${{ env }}",
                "${{ project }}",
                "${{ remote_dir }}",
                "${{ secrets.KEY }}",
            ] {
                let galley = ui.painter().layout_no_wrap(
                    var_name.to_string(),
                    Typo::MonoSm.font(),
                    TEXT_PRIMARY,
                );
                let pad_x = 6.0;
                let tag_w = galley.size().x + pad_x * 2.0;
                let tag_h = 22.0;
                let (tag_rect, tag_resp) =
                    ui.allocate_exact_size(Vec2::new(tag_w, tag_h), Sense::click());
                let tag_resp = tag_resp
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("Click to copy");
                let bg = if tag_resp.hovered() {
                    BLUE_PILL_HOVER_BG
                } else {
                    SUBTLE_BG
                };
                ui.painter()
                    .rect_filled(tag_rect, CornerRadius::same(4), bg);
                ui.painter().rect_stroke(
                    tag_rect,
                    CornerRadius::same(4),
                    Stroke::new(1.0_f32, CARD_BORDER),
                    StrokeKind::Inside,
                );
                let pos = Pos2::new(
                    tag_rect.left() + pad_x,
                    tag_rect.center().y - galley.size().y / 2.0,
                );
                ui.painter().galley(pos, galley, TEXT_PRIMARY);
                if tag_resp.clicked() {
                    ui.ctx().copy_text(var_name.to_string());
                    self.set_status_banner(format!("Copied {var_name} to clipboard"), false);
                }
            }
        });
    }
}
