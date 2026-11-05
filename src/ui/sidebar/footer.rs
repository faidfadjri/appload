//! Sidebar footer actions, updates, and installation status.

use super::*;

impl AppLoadApp {
    pub(crate) fn render_sidebar_footer(
        &mut self,
        footer_ui: &mut egui::Ui,
        full_w: f32,
        footer_rect: Rect,
        theme: &Theme,
    ) {
        footer_ui.painter().line_segment(
            [
                Pos2::new(footer_rect.left() - SB_PAD_X, footer_rect.top()),
                Pos2::new(footer_rect.right() + SB_PAD_X, footer_rect.top()),
            ],
            Stroke::new(1.0_f32, SB_BORDER_LINE),
        );

        // 3 Ghost Buttons (32px high, radius 6px) with SP_2 gaps
        // Action 1: Sync App Directory Files
        let sync_rect = Rect::from_min_size(
            Pos2::new(footer_rect.left(), footer_rect.top() + SP_8),
            Vec2::new(full_w, 32.0),
        );
        let sync_resp = footer_ui
            .interact(sync_rect, egui::Id::new("sidebar_sync_btn"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let sync_bg = if sync_resp.hovered() {
            SB_SURFACE_HOVER
        } else {
            Color32::TRANSPARENT
        };
        let sync_fg = if sync_resp.hovered() {
            SB_TEXT
        } else {
            SB_TEXT_MUTED
        };
        footer_ui
            .painter()
            .rect_filled(sync_rect, CornerRadius::same(R_MD as u8), sync_bg);
        draw_icon_refresh(
            footer_ui.painter(),
            Pos2::new(sync_rect.left() + 14.0, sync_rect.center().y),
            sync_fg,
        );
        footer_ui.painter().text(
            Pos2::new(sync_rect.left() + 30.0, sync_rect.center().y),
            Align2::LEFT_CENTER,
            "Sync App Files",
            Typo::Label.font(),
            sync_fg,
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

        // Action 2: Open App Folder
        let dir_rect = Rect::from_min_size(
            Pos2::new(footer_rect.left(), sync_rect.bottom() + SP_2),
            Vec2::new(full_w, 32.0),
        );
        let dir_resp = footer_ui
            .interact(
                dir_rect,
                egui::Id::new("sidebar_folder_btn"),
                Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let dir_bg = if dir_resp.hovered() {
            SB_SURFACE_HOVER
        } else {
            Color32::TRANSPARENT
        };
        let dir_fg = if dir_resp.hovered() {
            SB_TEXT
        } else {
            SB_TEXT_MUTED
        };
        footer_ui
            .painter()
            .rect_filled(dir_rect, CornerRadius::same(R_MD as u8), dir_bg);
        draw_icon_folder(
            footer_ui.painter(),
            Pos2::new(dir_rect.left() + 14.0, dir_rect.center().y),
            dir_fg,
        );
        footer_ui.painter().text(
            Pos2::new(dir_rect.left() + 30.0, dir_rect.center().y),
            Align2::LEFT_CENTER,
            "Open App Folder",
            Typo::Label.font(),
            dir_fg,
        );
        if dir_resp
            .on_hover_text(
                "Open the app folder where you can paste appload_state.json or deploy-*.ps1 files",
            )
            .clicked()
        {
            let dir = self.storage.app_data_dir();
            let _ = fs::create_dir_all(&dir);
            open_directory_in_os(&dir);
        }

        // Action 3: Check for Updates
        let upd_rect = Rect::from_min_size(
            Pos2::new(footer_rect.left(), dir_rect.bottom() + SP_2),
            Vec2::new(full_w, 32.0),
        );
        let upd_resp = footer_ui
            .interact(
                upd_rect,
                egui::Id::new("sidebar_update_btn"),
                Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let (upd_label, upd_fg) = if let Some(rel) = &self.available_update {
            (format!("Update: {}!", rel.tag_name), theme.warning)
        } else if self.update_checking {
            ("Checking Releases...".to_string(), SB_ACCENT)
        } else if upd_resp.hovered() {
            ("Check for Updates".to_string(), SB_TEXT)
        } else {
            ("Check for Updates".to_string(), SB_TEXT_MUTED)
        };
        let upd_bg = if upd_resp.hovered() {
            SB_SURFACE_HOVER
        } else {
            Color32::TRANSPARENT
        };
        footer_ui
            .painter()
            .rect_filled(upd_rect, CornerRadius::same(R_MD as u8), upd_bg);
        draw_icon_refresh(
            footer_ui.painter(),
            Pos2::new(upd_rect.left() + 14.0, upd_rect.center().y),
            upd_fg,
        );
        footer_ui.painter().text(
            Pos2::new(upd_rect.left() + 30.0, upd_rect.center().y),
            Align2::LEFT_CENTER,
            upd_label,
            Typo::Label.font(),
            upd_fg,
        );
        if upd_resp
            .on_hover_text(format!("Check GitHub Releases ({GITHUB_REPO}) for updates"))
            .clicked()
        {
            if self.available_update.is_some() {
                self.show_update_modal = true;
            } else if !self.update_checking {
                spawn_update_check(self.updater_tx.clone(), true);
            }
        }

        // Status Row: 28px high, green dot + "v0.11.0" on left, "Reinstall · Donate" right-aligned
        let stat_rect = Rect::from_min_size(
            Pos2::new(footer_rect.left(), upd_rect.bottom() + SP_8),
            Vec2::new(full_w, 28.0),
        );
        let install_state = check_install_state(&default_install_dir(), APP_VERSION);
        let (dot_color, status_text, action_text) = match &install_state {
            InstallState::NotInstalled => {
                (SB_ACCENT, "Not installed".to_string(), "Install (.exe)")
            }
            InstallState::InstalledCurrent { version, .. } => {
                (theme.success, format!("v{version}"), "Reinstall")
            }
            InstallState::UpdateReady {
                running_version, ..
            } => (theme.warning, format!("v{running_version}"), "Apply"),
        };

        footer_ui.painter().circle_filled(
            Pos2::new(stat_rect.left() + 4.0, stat_rect.center().y),
            3.5,
            dot_color,
        );

        let st_galley =
            footer_ui
                .painter()
                .layout_no_wrap(status_text, Typo::MonoSm.font(), SB_TEXT_MUTED);
        let st_rect = Rect::from_min_size(
            Pos2::new(
                stat_rect.left() + 14.0,
                stat_rect.center().y - st_galley.size().y / 2.0,
            ),
            st_galley.size(),
        );
        footer_ui
            .interact(st_rect, egui::Id::new("sidebar_status_lbl"), Sense::hover())
            .on_hover_text(format!("Installed AppLoad v{APP_VERSION}"));
        footer_ui
            .painter()
            .galley(st_rect.min, st_galley, SB_TEXT_MUTED);

        // Right-aligned actions: "Reinstall · Donate", dropping "Donate" at narrow widths before clipping
        let action_galley = footer_ui.painter().layout_no_wrap(
            action_text.to_string(),
            Typo::Small.font(),
            SB_TEXT_MUTED,
        );
        let sep_galley = footer_ui.painter().layout_no_wrap(
            " · ".to_string(),
            Typo::Small.font(),
            SB_TEXT_FAINT,
        );
        let don_galley = footer_ui.painter().layout_no_wrap(
            "Donate".to_string(),
            Typo::Small.font(),
            SB_TEXT_MUTED,
        );

        let total_combined_w = action_galley.size().x + sep_galley.size().x + don_galley.size().x;
        let avail_w = stat_rect.right() - (st_rect.right() + SP_8);
        let show_donate = avail_w >= total_combined_w;

        if show_donate {
            // Donate link (right-most)
            let don_rect = Rect::from_min_size(
                Pos2::new(
                    stat_rect.right() - don_galley.size().x,
                    stat_rect.center().y - don_galley.size().y / 2.0,
                ),
                don_galley.size(),
            );
            let don_resp = footer_ui
                .interact(
                    don_rect,
                    egui::Id::new("sidebar_donate_link"),
                    Sense::click(),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(
                    "Support AppLoad development on Trakteer (https://trakteer.id/faidfadjri)",
                );
            let don_color = if don_resp.hovered() {
                HEART_PINK
            } else {
                SB_TEXT_MUTED
            };
            footer_ui
                .painter()
                .galley(don_rect.min, don_galley, don_color);
            if don_resp.clicked() {
                open_url_in_browser(DONATION_URL);
            }

            // Middle separator " · "
            let sep_x = don_rect.left() - sep_galley.size().x;
            footer_ui.painter().galley(
                Pos2::new(sep_x, stat_rect.center().y - sep_galley.size().y / 2.0),
                sep_galley,
                SB_TEXT_FAINT,
            );

            // Reinstall link
            let link_x = sep_x - action_galley.size().x;
            let link_rect = Rect::from_min_size(
                Pos2::new(link_x, stat_rect.center().y - action_galley.size().y / 2.0),
                action_galley.size(),
            );
            let link_resp = footer_ui
                .interact(
                    link_rect,
                    egui::Id::new("sidebar_reinstall_link"),
                    Sense::click(),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            let link_color = if link_resp.hovered() {
                SB_ACCENT
            } else {
                SB_TEXT_MUTED
            };
            footer_ui
                .painter()
                .galley(link_rect.min, action_galley, link_color);
            if link_resp.clicked() {
                match install_or_update_self() {
                    Ok(msg) => self.status_banner = Some((msg, true)),
                    Err(err) => self.status_banner = Some((err, false)),
                }
            }
        } else {
            // Only reinstall link right-aligned
            let link_rect = Rect::from_min_size(
                Pos2::new(
                    stat_rect.right() - action_galley.size().x,
                    stat_rect.center().y - action_galley.size().y / 2.0,
                ),
                action_galley.size(),
            );
            let link_resp = footer_ui
                .interact(
                    link_rect,
                    egui::Id::new("sidebar_reinstall_link"),
                    Sense::click(),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            let link_color = if link_resp.hovered() {
                SB_ACCENT
            } else {
                SB_TEXT_MUTED
            };
            footer_ui
                .painter()
                .galley(link_rect.min, action_galley, link_color);
            if link_resp.clicked() {
                match install_or_update_self() {
                    Ok(msg) => self.status_banner = Some((msg, true)),
                    Err(err) => self.status_banner = Some((err, false)),
                }
            }
        }
    }
}
