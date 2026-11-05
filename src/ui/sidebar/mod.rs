//! Sidebar drawer, panel, and navigation tree.

use super::*;

pub(crate) mod context_menu;
pub(crate) mod footer;
pub(crate) mod nav;
pub(crate) mod project_list;

impl ShiplyApp {
    pub(crate) fn render_sidebar_drawer(&mut self, ctx: &egui::Context) {
        let screen_rect = ctx.screen_rect();

        egui::Area::new(egui::Id::new("sidebar_drawer_overlay"))
            .fixed_pos(screen_rect.min)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let (backdrop_rect, backdrop_resp) =
                    ui.allocate_exact_size(screen_rect.size(), Sense::click());
                ui.painter()
                    .rect_filled(backdrop_rect, CornerRadius::ZERO, MD_OVERLAY);
                if backdrop_resp.clicked() {
                    self.sidebar_drawer_open = false;
                }

                let drawer_rect = Rect::from_min_size(
                    screen_rect.min,
                    Vec2::new(SIDEBAR_EXPANDED_WIDTH, screen_rect.height()),
                );
                let mut drawer_ui = ui.new_child(egui::UiBuilder::new().max_rect(drawer_rect));
                drawer_ui
                    .painter()
                    .rect_filled(drawer_rect, CornerRadius::ZERO, SB_BG);
                drawer_ui.painter().line_segment(
                    [
                        Pos2::new(drawer_rect.right(), drawer_rect.top()),
                        Pos2::new(drawer_rect.right(), drawer_rect.bottom()),
                    ],
                    Stroke::new(1.0_f32, SB_BORDER_LINE),
                );

                let mut inner_ui = drawer_ui.new_child(
                    egui::UiBuilder::new().max_rect(drawer_rect.shrink2(Vec2::new(SB_PAD_X, 0.0))),
                );
                inner_ui.visuals_mut().override_text_color = Some(SB_TEXT);
                let mut sort_trigger_rect_out = None;
                self.render_sidebar_content(&mut inner_ui, false, ctx, &mut sort_trigger_rect_out);
                if let Some(r) = sort_trigger_rect_out {
                    self.render_sidebar_sort_dropdown(ctx, r);
                }
                self.render_sidebar_custom_context_menu(ctx);
            });
    }

    pub(crate) fn render_sidebar(&mut self, ctx: &egui::Context, anim_t: f32) {
        let is_rail = anim_t < 0.35;
        let target_expanded_w = SIDEBAR_EXPANDED_WIDTH;
        let width = if is_rail {
            SIDEBAR_COLLAPSED_WIDTH
        } else {
            SIDEBAR_COLLAPSED_WIDTH
                + (target_expanded_w - SIDEBAR_COLLAPSED_WIDTH)
                    * ((anim_t - 0.35) / 0.65).clamp(0.0, 1.0)
        };
        let pad_x = if is_rail { 8.0 } else { SB_PAD_X };
        let mut sort_trigger_rect_out = None;

        egui::SidePanel::left("shiply_sidebar")
            .resizable(false)
            .show_separator_line(false)
            .exact_width(width)
            .frame(
                egui::Frame::new()
                    .fill(SB_BG)
                    .inner_margin(Margin::symmetric(pad_x as i8, 0))
                    .stroke(Stroke::NONE),
            )
            .show(ctx, |ui| {
                // Hairline right border separating sidebar from central workspace
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [
                        Pos2::new(r.right(), r.top()),
                        Pos2::new(r.right(), r.bottom()),
                    ],
                    Stroke::new(1.0_f32, SB_BORDER_LINE),
                );
                ui.visuals_mut().override_text_color = Some(SB_TEXT);
                self.render_sidebar_content(ui, is_rail, ctx, &mut sort_trigger_rect_out);
            });

        if let Some(r) = sort_trigger_rect_out {
            self.render_sidebar_sort_dropdown(ctx, r);
        }
        self.render_sidebar_custom_context_menu(ctx);
    }

    pub(crate) fn render_sidebar_content(
        &mut self,
        ui: &mut egui::Ui,
        is_rail: bool,
        _ctx: &egui::Context,
        sort_trigger_rect_out: &mut Option<Rect>,
    ) {
        let inner_rect = ui.available_rect_before_wrap();
        let full_w = inner_rect.width();
        let theme = self.theme();

        if is_rail {
            // ==================== COLLAPSED RAIL MODE (64px) ====================
            // 1. Pinned Brand Top Bar (56px)
            let brand_rect = Rect::from_min_size(inner_rect.min, Vec2::new(full_w, 56.0));
            let brand_ui = ui.new_child(egui::UiBuilder::new().max_rect(brand_rect));
            let logo_rect = Rect::from_center_size(brand_rect.center(), Vec2::new(26.0, 26.0));
            draw_shiply_logo(
                brand_ui.painter(),
                logo_rect,
                6,
                Typo::H2.size(),
                self.logo_texture.as_ref(),
            );
            brand_ui.painter().line_segment(
                [
                    Pos2::new(brand_rect.left() - 8.0, brand_rect.bottom()),
                    Pos2::new(brand_rect.right() + 8.0, brand_rect.bottom()),
                ],
                Stroke::new(1.0_f32, SB_BORDER_LINE),
            );

            // 2. Pinned Footer (160px)
            let footer_h = 160.0_f32;
            let footer_rect = Rect::from_min_size(
                Pos2::new(inner_rect.left(), inner_rect.bottom() - footer_h),
                Vec2::new(full_w, footer_h),
            );
            let mut footer_ui = ui.new_child(egui::UiBuilder::new().max_rect(footer_rect));
            footer_ui.spacing_mut().item_spacing = Vec2::ZERO;
            footer_ui.painter().line_segment(
                [
                    Pos2::new(footer_rect.left() - 8.0, footer_rect.top()),
                    Pos2::new(footer_rect.right() + 8.0, footer_rect.top()),
                ],
                Stroke::new(1.0_f32, SB_BORDER_LINE),
            );

            let btn_size = Vec2::new(32.0, 32.0);
            let center_x = footer_rect.center().x;

            // Sync Button
            let sync_rect = Rect::from_center_size(
                Pos2::new(center_x, footer_rect.top() + SP_8 + 16.0),
                btn_size,
            );
            let sync_resp = footer_ui
                .interact(sync_rect, egui::Id::new("rail_sync_btn"), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text("Sync App Directory Files");
            if sync_resp.hovered() {
                footer_ui.painter().rect_filled(
                    sync_rect,
                    CornerRadius::same(R_MD as u8),
                    SB_SURFACE_HOVER,
                );
            }
            draw_icon_refresh(
                footer_ui.painter(),
                sync_rect.center(),
                if sync_resp.hovered() {
                    SB_TEXT
                } else {
                    SB_TEXT_MUTED
                },
            );
            if sync_resp.clicked() {
                self.sidebar_refreshing_until = Some(Instant::now() + Duration::from_millis(400));
                self.storage.reset_sync_cache();
                if let Some(msg) = self.storage.sync_from_app_directory() {
                    self.refresh_selected_project_metadata(true);
                    self.status_banner = Some((msg, true));
                } else {
                    self.status_banner = Some((
                        format!(
                            "Synced app directory ({})",
                            self.storage.app_data_dir().display()
                        ),
                        true,
                    ));
                }
            }

            // Folder Button
            let dir_rect = Rect::from_center_size(
                Pos2::new(center_x, footer_rect.top() + SP_8 + 32.0 + SP_2 + 16.0),
                btn_size,
            );
            let dir_resp = footer_ui
                .interact(dir_rect, egui::Id::new("rail_folder_btn"), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text("Open App Folder (JSON / .ps1)");
            if dir_resp.hovered() {
                footer_ui.painter().rect_filled(
                    dir_rect,
                    CornerRadius::same(R_MD as u8),
                    SB_SURFACE_HOVER,
                );
            }
            draw_icon_folder(
                footer_ui.painter(),
                dir_rect.center(),
                if dir_resp.hovered() {
                    SB_TEXT
                } else {
                    SB_TEXT_MUTED
                },
            );
            if dir_resp.clicked() {
                let dir = self.storage.app_data_dir();
                let _ = fs::create_dir_all(&dir);
                open_directory_in_os(&dir);
            }

            // Updates Button
            let upd_rect = Rect::from_center_size(
                Pos2::new(center_x, footer_rect.top() + SP_8 + 64.0 + 4.0 + 16.0),
                btn_size,
            );
            let upd_resp = footer_ui
                .interact(upd_rect, egui::Id::new("rail_upd_btn"), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(if self.available_update.is_some() {
                    "Update Available! Click to view"
                } else {
                    "Check for Updates"
                });
            if upd_resp.hovered() {
                footer_ui.painter().rect_filled(
                    upd_rect,
                    CornerRadius::same(R_MD as u8),
                    SB_SURFACE_HOVER,
                );
            }
            let upd_color = if self.available_update.is_some() {
                theme.warning
            } else if upd_resp.hovered() {
                SB_TEXT
            } else {
                SB_TEXT_MUTED
            };
            draw_icon_refresh(footer_ui.painter(), upd_rect.center(), upd_color);
            if upd_resp.clicked() {
                if self.available_update.is_some() {
                    self.show_update_modal = true;
                } else if !self.update_checking {
                    spawn_update_check(self.updater_tx.clone(), true);
                }
            }

            // Status Green Dot
            let dot_pos = Pos2::new(center_x, footer_rect.top() + 116.0 + 14.0);
            let dot_color = if self.available_update.is_some() {
                theme.warning
            } else {
                theme.success
            };
            let status_msg = if let Some(rel) = &self.available_update {
                format!("Update Available: {}", rel.tag_name)
            } else {
                format!("Up to date (v{APP_VERSION})")
            };
            let dot_rect = Rect::from_center_size(dot_pos, Vec2::new(14.0, 14.0));
            footer_ui
                .interact(dot_rect, egui::Id::new("rail_status_dot"), Sense::hover())
                .on_hover_text(status_msg);
            footer_ui.painter().circle_filled(dot_pos, 3.5, dot_color);

            // 3. Middle Scrollable Rail Content
            let middle_rect = Rect::from_min_max(
                Pos2::new(inner_rect.left(), brand_rect.bottom() + 1.0),
                Pos2::new(inner_rect.right(), footer_rect.top() - 1.0),
            );
            let mut middle_ui = ui.new_child(egui::UiBuilder::new().max_rect(middle_rect));
            ScrollArea::vertical()
                .id_salt("sidebar_rail_middle")
                .auto_shrink([false, false])
                .show(&mut middle_ui, |ui| {
                    let full_w = ui.available_width();
                    let center_x = full_w / 2.0;

                    // Nav item 1: All Deployments
                    let is_overview = matches!(self.selection, SidebarSelection::AllDeployments);
                    let (slot_rect, _) = ui.allocate_exact_size(Vec2::new(full_w, 32.0), Sense::hover());
                    let all_rect = Rect::from_center_size(slot_rect.center(), btn_size);
                    let all_resp = ui
                        .interact(all_rect, egui::Id::new("rail_all_deploy_btn"), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text("All Deployments");
                    let all_bg = if is_overview || all_resp.hovered() {
                        SB_SURFACE_HOVER
                    } else {
                        Color32::TRANSPARENT
                    };
                    let all_fg = if is_overview {
                        Color32::WHITE
                    } else if all_resp.hovered() {
                        SB_TEXT
                    } else {
                        SB_TEXT_MUTED
                    };
                    ui.painter().rect_filled(all_rect, CornerRadius::same(R_MD as u8), all_bg);
                    if is_overview {
                        let bar_rect = Rect::from_min_max(
                            Pos2::new(all_rect.left(), all_rect.top() + 6.0),
                            Pos2::new(all_rect.left() + 2.0, all_rect.bottom() - 6.0),
                        );
                        ui.painter().rect_filled(
                            bar_rect,
                            CornerRadius { nw: 0, ne: 2, se: 2, sw: 0 },
                            SB_ACCENT,
                        );
                    }
                    draw_icon_layers(ui.painter(), all_rect.center(), all_fg);
                    if all_resp.clicked() {
                        self.selection = SidebarSelection::AllDeployments;
                    }
                    ui.add_space(SP_8);

                    // Nav item 2: Bulk Deploy
                    if !self.storage.projects.is_empty() {
                        let is_bulk = self.show_bulk_deploy_modal;
                        let (slot_rect, _) = ui.allocate_exact_size(Vec2::new(full_w, 32.0), Sense::hover());
                        let bulk_rect = Rect::from_center_size(slot_rect.center(), btn_size);
                        let bulk_resp = ui
                            .interact(bulk_rect, egui::Id::new("rail_bulk_deploy_btn"), Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text("Bulk Deploy...");
                        let bulk_bg = if is_bulk || bulk_resp.hovered() {
                            SB_SURFACE_HOVER
                        } else {
                            Color32::TRANSPARENT
                        };
                        let bulk_fg = if is_bulk {
                            Color32::WHITE
                        } else if bulk_resp.hovered() {
                            SB_TEXT
                        } else {
                            SB_TEXT_MUTED
                        };
                        ui.painter().rect_filled(bulk_rect, CornerRadius::same(R_MD as u8), bulk_bg);
                        if is_bulk {
                            let bar_rect = Rect::from_min_max(
                                Pos2::new(bulk_rect.left(), bulk_rect.top() + 6.0),
                                Pos2::new(bulk_rect.left() + 2.0, bulk_rect.bottom() - 6.0),
                            );
                            ui.painter().rect_filled(
                                bar_rect,
                                CornerRadius { nw: 0, ne: 2, se: 2, sw: 0 },
                                SB_ACCENT,
                            );
                        }
                        draw_icon_send(ui.painter(), bulk_rect.center(), bulk_fg);
                        if bulk_resp.clicked() {
                            self.open_bulk_deploy_modal();
                        }
                        ui.add_space(SP_8);
                    }

                    // Nav item 3: Secret Management (OS Credential Manager)
                    let is_secrets = matches!(self.selection, SidebarSelection::SecretManagement);
                    let (slot_rect, _) = ui.allocate_exact_size(Vec2::new(full_w, 32.0), Sense::hover());
                    let sec_rect = Rect::from_center_size(slot_rect.center(), btn_size);
                    let sec_resp = ui
                        .interact(
                            sec_rect,
                            egui::Id::new("rail_secret_mgmt_btn"),
                            Sense::click(),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text("Secret Management (OS Credential Manager)");
                    let sec_bg = if is_secrets || sec_resp.hovered() {
                        SB_SURFACE_HOVER
                    } else {
                        Color32::TRANSPARENT
                    };
                    let sec_fg = if is_secrets {
                        Color32::WHITE
                    } else if sec_resp.hovered() {
                        SB_TEXT
                    } else {
                        SB_TEXT_MUTED
                    };
                    ui.painter()
                        .rect_filled(sec_rect, CornerRadius::same(R_MD as u8), sec_bg);
                    if is_secrets {
                        let bar_rect = Rect::from_min_max(
                            Pos2::new(sec_rect.left(), sec_rect.top() + 6.0),
                            Pos2::new(sec_rect.left() + 2.0, sec_rect.bottom() - 6.0),
                        );
                        ui.painter().rect_filled(
                            bar_rect,
                            CornerRadius { nw: 0, ne: 2, se: 2, sw: 0 },
                            SB_ACCENT,
                        );
                    }
                    draw_outline_icon(ui.painter(), sec_rect.center(), OutlineIcon::Key, sec_fg);
                    if sec_resp.clicked() {
                        self.selection = SidebarSelection::SecretManagement;
                    }
                    ui.add_space(SP_8);

                    // Separator
                    let (sep_rect, _) = ui.allocate_exact_size(Vec2::new(full_w, 1.0), Sense::hover());
                    let sep_y = sep_rect.center().y;
                    ui.painter().line_segment(
                        [Pos2::new(center_x - 14.0, sep_y), Pos2::new(center_x + 14.0, sep_y)],
                        Stroke::new(1.0_f32, SB_BORDER_LINE),
                    );
                    ui.add_space(SP_12);

                    // Add Project Button (+)
                    let (slot_rect, _) = ui.allocate_exact_size(Vec2::new(full_w, 32.0), Sense::hover());
                    let add_rect = Rect::from_center_size(slot_rect.center(), btn_size);
                    let add_resp = ui
                        .interact(add_rect, egui::Id::new("rail_add_proj_btn"), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text("Add Project");
                    let add_bg = if add_resp.hovered() { SB_SURFACE_HOVER } else { Color32::TRANSPARENT };
                    ui.painter().rect_filled(add_rect, CornerRadius::same(R_MD as u8), add_bg);
                    draw_icon_plus(ui.painter(), add_rect.center(), if add_resp.hovered() { SB_TEXT } else { SB_TEXT_MUTED });
                    if add_resp.clicked() {
                        self.open_new_project_modal();
                    }
                    ui.add_space(SP_8);

                    // Project Avatars
                    let mut clicked_project_id = None;
                    for project in &self.storage.projects {
                        let is_selected = matches!(&self.selection, SidebarSelection::Project(id) if id == &project.id);
                        let (slot_rect, _) = ui.allocate_exact_size(Vec2::new(full_w, 32.0), Sense::hover());
                        let p_avatar_rect = Rect::from_center_size(slot_rect.center(), Vec2::new(32.0, 32.0));
                        let p_resp = ui
                            .interact(p_avatar_rect, egui::Id::new(("rail_proj", &project.id)), Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand);

                        let bg_fill = if is_selected {
                            SB_ACCENT_SOFT
                        } else if p_resp.hovered() {
                            SB_SURFACE_HOVER
                        } else {
                            SB_SURFACE
                        };
                        let stroke = if is_selected {
                            Stroke::new(2.0_f32, SB_ACCENT)
                        } else {
                            Stroke::new(1.0_f32, SB_BORDER_LINE)
                        };

                        ui.painter().rect_filled(p_avatar_rect, CornerRadius::same(8), bg_fill);
                        ui.painter().rect_stroke(p_avatar_rect, CornerRadius::same(8), stroke, StrokeKind::Inside);

                        let initial = project.name.chars().next().unwrap_or('P').to_uppercase().to_string();
                        let fg_color = if is_selected { SB_ACCENT } else { SB_TEXT };
                        ui.painter().text(
                            p_avatar_rect.center(),
                            Align2::CENTER_CENTER,
                            initial,
                            Typo::Label.font(),
                            fg_color,
                        );

                        let mut env_summary = String::new();
                        for env in &project.environments {
                            if let Some(dep) = self.storage.last_succeeded_deployment(&project.id, &env.key) {
                                env_summary.push_str(&format!("\n• {}: v{}", env.name, dep.version));
                            } else {
                                env_summary.push_str(&format!("\n• {}: —", env.name));
                            }
                        }
                        let tooltip = format!("{}{}", project.name, env_summary);
                        let p_resp = p_resp.on_hover_text(tooltip);

                        if p_resp.secondary_clicked() {
                            if let Some(pos) = ui.input(|i| i.pointer.latest_pos()) {
                                self.sidebar_context_menu = Some(SidebarContextMenuState {
                                    project_id: project.id.clone(),
                                    click_pos: pos,
                                    submenu_open: false,
                                });
                            }
                        }

                        if p_resp.clicked() {
                            clicked_project_id = Some(project.id.clone());
                        }

                        ui.add_space(SP_8);
                    }

                    if let Some(pid) = clicked_project_id {
                        self.sidebar_context_menu = None;
                        let was_different = match &self.selection {
                            SidebarSelection::Project(cur) => cur != &pid,
                            SidebarSelection::AllDeployments
                            | SidebarSelection::SecretManagement => true,
                        };
                        self.selection = SidebarSelection::Project(pid.clone());
                        if was_different {
                            self.project_switch_loading_id = Some(pid);
                            self.project_switch_loading_start = Some(Instant::now());
                        }
                        self.refresh_selected_project_metadata(true);
                    }
                });
        } else {
            // ==================== EXPANDED SIDEBAR MODE (272px) ====================
            // 1. Pinned Brand Top Bar (56px)
            let brand_rect = Rect::from_min_size(inner_rect.min, Vec2::new(full_w, 56.0));
            let brand_ui = ui.new_child(egui::UiBuilder::new().max_rect(brand_rect));
            {
                let logo_rect = Rect::from_min_size(
                    Pos2::new(brand_rect.left(), brand_rect.center().y - 13.0),
                    Vec2::new(26.0, 26.0),
                );
                draw_shiply_logo(
                    brand_ui.painter(),
                    logo_rect,
                    6,
                    Typo::H2.size(),
                    self.logo_texture.as_ref(),
                );
                brand_ui.painter().text(
                    Pos2::new(logo_rect.right() + SP_8, brand_rect.center().y),
                    Align2::LEFT_CENTER,
                    "Shiply",
                    Typo::H2.font(),
                    Color32::WHITE,
                );

                // Hairline bottom border
                brand_ui.painter().line_segment(
                    [
                        Pos2::new(brand_rect.left() - SB_PAD_X, brand_rect.bottom()),
                        Pos2::new(brand_rect.right() + SB_PAD_X, brand_rect.bottom()),
                    ],
                    Stroke::new(1.0_f32, SB_BORDER_LINE),
                );
            }

            // 2. Pinned Footer (160px) with zero-spacing to guarantee no bottom overflow
            let footer_h = 160.0_f32;
            let footer_rect = Rect::from_min_size(
                Pos2::new(inner_rect.left(), inner_rect.bottom() - footer_h),
                Vec2::new(full_w, footer_h),
            );
            let mut footer_ui = ui.new_child(egui::UiBuilder::new().max_rect(footer_rect));
            footer_ui.spacing_mut().item_spacing = Vec2::ZERO;
            self.render_sidebar_footer(&mut footer_ui, full_w, footer_rect, &theme);

            // 3. Middle Scrollable Content (Nav items + Projects header + Search + Projects list)
            let middle_rect = Rect::from_min_max(
                Pos2::new(inner_rect.left(), brand_rect.bottom() + 1.0),
                Pos2::new(inner_rect.right(), footer_rect.top() - 1.0),
            );
            let mut middle_ui = ui.new_child(egui::UiBuilder::new().max_rect(middle_rect));
            middle_ui.spacing_mut().scroll.floating = true;
            middle_ui.spacing_mut().scroll.bar_width = 4.0;
            middle_ui.spacing_mut().scroll.bar_inner_margin = 1.0;

            ScrollArea::vertical()
                .id_salt("sidebar_scrollable_middle")
                .auto_shrink([false, false])
                .show(&mut middle_ui, |ui| {
                    let full_w = ui.available_width();
                    self.render_sidebar_nav(ui, full_w, sort_trigger_rect_out);
                    self.render_sidebar_project_list(ui, full_w);
                });
        }
    }
}
