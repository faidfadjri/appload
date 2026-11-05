//! Deploy and rollback action bar.

use super::*;

impl ShiplyApp {
    pub(crate) fn render_deploy_action_bar(
        &mut self,
        ui: &mut egui::Ui,
        env_display_name: &str,
        rollback_candidate: Option<&DeploymentRecord>,
        sync_trigger_rect_out: &mut Option<Rect>,
    ) {
        let mut trigger_deploy = false;
        let mut trigger_rollback = None;
        let avail_w = ui.available_width();

        let project_id_owned = match &self.selection {
            SidebarSelection::Project(pid) => pid.clone(),
            SidebarSelection::AllDeployments | SidebarSelection::SecretManagement => String::new(),
        };
        let project_id = project_id_owned.as_str();

        let is_busy = self.is_env_deploying(project_id, &self.selected_env_key);
        let btn_text = if is_busy {
            "Deploying...".to_string()
        } else {
            let ver = self.deploy_version_input.trim();
            if ver.is_empty() {
                format!("Deploy to {env_display_name}")
            } else {
                format!("Deploy to {env_display_name} (v{ver})")
            }
        };

        let is_two_rows = avail_w < 960.0;
        let bar_h = if is_two_rows { 116.0 } else { 68.0 };

        let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, bar_h), Sense::hover());
        draw_elevated_card(
            ui.painter(),
            bar_rect,
            CARD_BG,
            Stroke::new(1.0_f32, CARD_BORDER),
        );

        let row1_rect = if is_two_rows {
            Rect::from_min_size(
                Pos2::new(bar_rect.left() + 16.0, bar_rect.top() + 16.0),
                Vec2::new(bar_rect.width() - 32.0, 36.0),
            )
        } else {
            Rect::from_min_size(
                Pos2::new(bar_rect.left() + 16.0, bar_rect.center().y - 18.0),
                Vec2::new(bar_rect.width() - 32.0, 36.0),
            )
        };

        // Render Left & Middle Controls in row1_rect
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("deploy_bar_left_controls")
                .max_rect(row1_rect)
                .layout(Layout::left_to_right(Align::Center)),
        );

        // 1. Version input (width 110, Typo::Mono, height 36)
        styled_text_edit(
            &mut child,
            "deploy_version_input",
            &mut self.deploy_version_input,
            110.0,
            Some("1.0.0"),
        );
        let ver_id = child.make_persistent_id("deploy_version_input");
        self.is_version_input_focused = child.memory(|m| m.has_focus(ver_id));
        child.add_space(8.0);

        // 2. Segmented buttons [+Patch | +Minor | +Major] (height 36, attached)
        let seg_h = 36.0;
        let seg_w = 54.0;
        let bumps = [
            ("+Patch", VersionBump::Patch),
            ("+Minor", VersionBump::Minor),
            ("+Major", VersionBump::Major),
        ];
        for (i, (lbl, bump)) in bumps.iter().enumerate() {
            let (rect, resp) = child.allocate_exact_size(Vec2::new(seg_w, seg_h), Sense::click());
            let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.clicked() {
                self.deploy_version_input = bump_semver(&self.deploy_version_input, *bump);
            }
            let bg = if resp.is_pointer_button_down_on() {
                CARD_BORDER
            } else if resp.hovered() {
                SUBTLE_BG
            } else {
                CARD_BG
            };
            let radius = if i == 0 {
                CornerRadius {
                    nw: R_MD as u8,
                    ne: 0,
                    sw: R_MD as u8,
                    se: 0,
                }
            } else if i == bumps.len() - 1 {
                CornerRadius {
                    nw: 0,
                    ne: R_MD as u8,
                    sw: 0,
                    se: R_MD as u8,
                }
            } else {
                CornerRadius::ZERO
            };
            child.painter().rect_filled(rect, radius, bg);
            child.painter().rect_stroke(
                rect,
                radius,
                Stroke::new(1.0_f32, CARD_BORDER),
                StrokeKind::Inside,
            );
            let galley = child.painter().layout_no_wrap(
                lbl.to_string(),
                Typo::SmallMedium.font(),
                TEXT_PRIMARY,
            );
            let text_pos = Pos2::new(
                rect.center().x - galley.size().x / 2.0,
                rect.center().y - galley.size().y / 2.0,
            );
            child.painter().galley(text_pos, galley, TEXT_PRIMARY);
        }
        child.add_space(8.0);

        // 3. Sync button (36x36 OutlineIcon::Refresh)
        let sync_candidates = self.sync_version_candidates(project_id);
        let sync_tooltip = if sync_candidates.is_empty() {
            "Sync version across environments (no other deployed versions found yet)".to_string()
        } else if let Some(latest) = sync_candidates.iter().find(|c| c.is_latest) {
            format!(
                "Sync version across environments (latest: v{} from {})",
                latest.version, latest.source_name
            )
        } else {
            "Sync version across environments".to_string()
        };

        let (sync_rect, sync_resp) =
            child.allocate_exact_size(Vec2::new(36.0, 36.0), Sense::click());
        let sync_resp = sync_resp
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(sync_tooltip);
        *sync_trigger_rect_out = Some(sync_rect);

        if sync_resp.clicked() {
            if sync_candidates.len() == 1 {
                let cand = &sync_candidates[0];
                self.apply_synced_version(&cand.version, &cand.source_name);
            } else {
                self.sync_version_dropdown_open = !self.sync_version_dropdown_open;
            }
        }
        let sync_bg = if self.sync_version_dropdown_open {
            BLUE_PILL_BG
        } else if sync_resp.hovered() {
            SUBTLE_BG
        } else {
            CARD_BG
        };
        let sync_border = if self.sync_version_dropdown_open {
            PRIMARY_BLUE
        } else {
            CARD_BORDER
        };
        let sync_icon_color = if self.sync_version_dropdown_open {
            PRIMARY_BLUE
        } else {
            TEXT_PRIMARY
        };
        child
            .painter()
            .rect_filled(sync_rect, CornerRadius::same(R_MD as u8), sync_bg);
        child.painter().rect_stroke(
            sync_rect,
            CornerRadius::same(R_MD as u8),
            Stroke::new(1.0_f32, sync_border),
            StrokeKind::Inside,
        );
        draw_outline_icon(
            child.painter(),
            sync_rect.center(),
            OutlineIcon::Refresh,
            sync_icon_color,
        );

        // 4. Vertical Separator
        child.add_space(12.0);
        let sep_x = child.cursor().min.x;
        child.painter().line_segment(
            [
                Pos2::new(sep_x, row1_rect.top() + 6.0),
                Pos2::new(sep_x, row1_rect.bottom() - 6.0),
            ],
            Stroke::new(1.0_f32, CARD_BORDER),
        );
        child.add_space(12.0);

        // 5. Checkboxes (using kit::checkbox)
        kit::checkbox(&mut child, &mut self.bump_package_json, "Bump package.json");
        child.add_space(12.0);
        kit::checkbox(&mut child, &mut self.create_git_tag, "Create git tag");

        // Now calculate right buttons (Rollback + Deploy)
        let deploy_galley =
            ui.painter()
                .layout_no_wrap(btn_text, Typo::Label.font(), Color32::WHITE);
        let deploy_w = (deploy_galley.size().x + 14.0 + 8.0 + 32.0).max(160.0);

        let rb_data = rollback_candidate.map(|prev_dep| {
            let label = format!("Rollback to v{}", prev_dep.version);
            let galley = ui
                .painter()
                .layout_no_wrap(label, Typo::Label.font(), TEXT_PRIMARY);
            let w = (galley.size().x + 14.0 + 8.0 + 24.0).max(120.0);
            (prev_dep.clone(), galley, w)
        });

        let rb_w = rb_data.as_ref().map(|(_, _, w)| *w).unwrap_or(0.0);

        let (deploy_rect, rb_rect_opt) = if is_two_rows {
            // Row 2: aligned right (or full width if very narrow < 480)
            let row2_y = bar_rect.top() + 64.0;
            if avail_w < 480.0 {
                let d_rect = Rect::from_min_size(
                    Pos2::new(bar_rect.left() + 16.0, row2_y),
                    Vec2::new(bar_rect.width() - 32.0, 36.0),
                );
                (d_rect, None)
            } else {
                let d_rect = Rect::from_min_size(
                    Pos2::new(bar_rect.right() - 16.0 - deploy_w, row2_y),
                    Vec2::new(deploy_w, 36.0),
                );
                let r_rect = if rb_w > 0.0 {
                    Some(Rect::from_min_size(
                        Pos2::new(d_rect.left() - 10.0 - rb_w, row2_y),
                        Vec2::new(rb_w, 36.0),
                    ))
                } else {
                    None
                };
                (d_rect, r_rect)
            }
        } else {
            // Single row: aligned right
            let d_rect = Rect::from_min_size(
                Pos2::new(
                    bar_rect.right() - 16.0 - deploy_w,
                    bar_rect.center().y - 18.0,
                ),
                Vec2::new(deploy_w, 36.0),
            );
            let r_rect = if rb_w > 0.0 {
                Some(Rect::from_min_size(
                    Pos2::new(d_rect.left() - 10.0 - rb_w, bar_rect.center().y - 18.0),
                    Vec2::new(rb_w, 36.0),
                ))
            } else {
                None
            };
            (d_rect, r_rect)
        };

        // Render Rollback Button if present
        if let (Some(r_rect), Some((prev_dep, rb_galley, _))) = (rb_rect_opt, rb_data) {
            let rb_resp = ui
                .interact(r_rect, egui::Id::new("quick_rollback_btn"), Sense::click())
                .on_hover_cursor(if is_busy {
                    egui::CursorIcon::NotAllowed
                } else {
                    egui::CursorIcon::PointingHand
                })
                .on_hover_text(format!(
                    "Rollback {} to previous successful release v{}",
                    env_display_name, prev_dep.version
                ));
            let rb_bg = if rb_resp.is_pointer_button_down_on() {
                CARD_BORDER
            } else if rb_resp.hovered() && !is_busy {
                SUBTLE_BG
            } else {
                CARD_BG
            };
            ui.painter()
                .rect_filled(r_rect, CornerRadius::same(R_MD as u8), rb_bg);
            ui.painter().rect_stroke(
                r_rect,
                CornerRadius::same(R_MD as u8),
                Stroke::new(1.0_f32, CARD_BORDER),
                StrokeKind::Inside,
            );
            let content_w = 14.0 + 8.0 + rb_galley.size().x;
            let start_x = r_rect.center().x - content_w / 2.0;
            draw_outline_icon(
                ui.painter(),
                Pos2::new(start_x + 7.0, r_rect.center().y),
                OutlineIcon::RotateCcw,
                TEXT_PRIMARY,
            );
            let text_pos = Pos2::new(
                start_x + 14.0 + 8.0,
                r_rect.center().y - rb_galley.size().y / 2.0,
            );
            ui.painter().galley(text_pos, rb_galley, TEXT_PRIMARY);

            if !is_busy && rb_resp.clicked() {
                trigger_rollback = Some(prev_dep);
            }
        }

        // Render Primary Deploy Button
        let deploy_resp = ui
            .interact(
                deploy_rect,
                egui::Id::new("primary_deploy_btn"),
                Sense::click(),
            )
            .on_hover_cursor(if is_busy {
                egui::CursorIcon::NotAllowed
            } else {
                egui::CursorIcon::PointingHand
            });

        let deploy_bg = if is_busy {
            TEXT_MUTED
        } else if deploy_resp.is_pointer_button_down_on() {
            PRIMARY_BLUE_ACTIVE
        } else if deploy_resp.hovered() {
            PRIMARY_BLUE_HOVER
        } else {
            PRIMARY_BLUE
        };

        ui.painter()
            .rect_filled(deploy_rect, CornerRadius::same(R_MD as u8), deploy_bg);

        let d_content_w = 14.0 + 8.0 + deploy_galley.size().x;
        let d_start_x = deploy_rect.center().x - d_content_w / 2.0;
        draw_icon_send(
            ui.painter(),
            Pos2::new(d_start_x + 7.0, deploy_rect.center().y),
            Color32::WHITE,
        );
        let d_text_pos = Pos2::new(
            d_start_x + 14.0 + 8.0,
            deploy_rect.center().y - deploy_galley.size().y / 2.0,
        );
        ui.painter()
            .galley(d_text_pos, deploy_galley, Color32::WHITE);

        if !is_busy && deploy_resp.clicked() {
            trigger_deploy = true;
        }

        if let Some(rb_record) = trigger_rollback {
            self.trigger_redeploy_from_record(&rb_record, true);
        } else if trigger_deploy {
            self.trigger_current_environment_deploy();
        }
    }
}
