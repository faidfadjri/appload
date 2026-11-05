//! Application update availability and download progress modal.

use super::*;

impl ShiplyApp {
    pub(crate) fn render_github_update_modal(&mut self, ctx: &egui::Context) {
        let Some(anim_t) =
            prepare_animated_modal(ctx, "github_update", &mut self.show_update_modal)
        else {
            return;
        };

        if let Some(release) = self.available_update.clone() {
            let (window, inner_w) =
                animated_modal_window(ctx, "Software Update Available", anim_t, None);
            window.show(ctx, |ui| {
                ui.set_width(inner_w);
                    if render_modal_header(
                        ui,
                        "close_github_update_modal",
                        "Software Update Available",
                        &format!(
                            "A newer release of Shiply is available on GitHub Releases ({GITHUB_REPO})."
                        ),
                    ) {
                        self.show_update_modal = false;
                    }

                    ui.add_space(SP_12);

                    // Version Comparison Card (Current Version vs. New Version Tag)
                    egui::Frame::new()
                        .fill(MD_PANEL_BG)
                        .stroke(Stroke::new(1.0_f32, MD_BORDER))
                        .corner_radius(R_LG)
                        .inner_margin(Margin::symmetric(14, 12))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            horizontal_row(ui, 26.0, |ui| {
                                ui.label(
                                    RichText::new("Current Version:")
                                        .typo(Typo::Small)
                                        .color(MD_TEXT_MUTED),
                                );
                                render_pill(
                                    ui,
                                    &format!("v{APP_VERSION}"),
                                    MD_INPUT_BG,
                                    MD_TEXT_MUTED,
                                );
                                ui.label(
                                    RichText::new("→").typo(Typo::Body)
                                        .color(ACCENT),
                                );
                                ui.label(
                                    RichText::new("New Release:")
                                        .typo(Typo::Small)
                                        .color(MD_TEXT_MUTED),
                                );
                                render_pill(
                                    ui,
                                    &release.tag_name,
                                    BADGE_DEV_BG,
                                    BADGE_DEV_TEXT,
                                );
                            });
                            ui.add_space(SP_4);
                            ui.label(
                                RichText::new(format!(
                                    "{}  •  Published: {}",
                                    release.name, release.published_at
                                ))
                                .typo(Typo::Caption)
                                .color(MD_TEXT_MUTED),
                            );
                        });

                    if self.has_active_deployments() {
                        ui.add_space(SP_8);
                        egui::Frame::new()
                            .fill(BLUE_PILL_BG)
                            .stroke(Stroke::new(1.0_f32, INFO_SOFT))
                            .corner_radius(R_SM)
                            .inner_margin(Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                let active_count = self.active_deployments.len();
                                ui.label(
                                    RichText::new(format!(
                                        "{active_count} active deployment{} in progress. Deployments will continue running uninterrupted and update will apply once completed.",
                                        if active_count > 1 { "s are" } else { " is" }
                                    ))
                                    .typo(Typo::Small)
                                    .color(BLUE_PILL_TEXT),
                                );
                            });
                    }

                    ui.add_space(SP_12);

                    // Release Notes / Changelog section
                    ui.label(
                        RichText::new("Release notes / changelog")
                            .typo(Typo::Caption)
                            .color(MD_TEXT_MUTED),
                    );
                    ui.add_space(SP_4);
                    egui::Frame::new()
                        .fill(MD_INPUT_BG)
                        .stroke(Stroke::new(1.0_f32, MD_BORDER))
                        .corner_radius(R_MD)
                        .inner_margin(Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ScrollArea::vertical()
                                .id_salt("github_release_notes_scroll")
                                .max_height(190.0)
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    for line in release.body.lines() {
                                        let trimmed = line.trim();
                                        if let Some(h) = trimmed.strip_prefix("### ") {
                                            ui.add_space(SP_4);
                                            ui.label(
                                                RichText::new(h)
                                                    .typo(Typo::Label).color(MD_TEXT),
                                            );
                                        } else if let Some(h) = trimmed.strip_prefix("## ") {
                                            ui.add_space(SP_4);
                                            ui.label(
                                                RichText::new(h)
                                                    .typo(Typo::Label).color(PRIMARY_BLUE),
                                            );
                                        } else {
                                            ui.label(
                                                RichText::new(line)
                                                    .typo(Typo::Small)
                                                    .color(MD_TEXT_MUTED),
                                            );
                                        }
                                    }
                                });
                        });

                    // Download Progress Bar / Status Indicator
                    if self.update_downloading || self.update_progress > 0.0 {
                        ui.add_space(SP_12);
                        let pct = (self.update_progress * 100.0).clamp(0.0, 100.0);
                        ui.label(
                            RichText::new(format!(
                                "{} ({pct:.0}%)",
                                if self.update_status_text.is_empty() {
                                    "Downloading update..."
                                } else {
                                    &self.update_status_text
                                }
                            ))
                            .typo(Typo::SmallMedium)
                            .color(ACCENT),
                        );
                        ui.add_space(SP_4);
                        let bar_w = ui.available_width();
                        let (bar_rect, _) =
                            ui.allocate_exact_size(Vec2::new(bar_w, 8.0), Sense::hover());
                        ui.painter()
                            .rect_filled(bar_rect, CornerRadius::same(4), MD_PANEL_BG);
                        let fill_w = (bar_w * self.update_progress.clamp(0.02, 1.0)).min(bar_w);
                        let fill_rect = Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, 8.0));
                        ui.painter()
                            .rect_filled(fill_rect, CornerRadius::same(4), ACCENT);
                    }

                    if let Some(err) = &self.update_error {
                        ui.add_space(SP_8);
                        egui::Frame::new()
                            .fill(FAIL_PILL_BG)
                            .stroke(Stroke::new(1.0_f32, DANGER_SOFT))
                            .corner_radius(R_SM)
                            .inner_margin(Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(err)
                                        .typo(Typo::Small)
                                        .color(FAIL_PILL_TEXT),
                                );
                            });
                    }

                    ui.add_space(SP_16);
                    draw_divider(ui, MD_BORDER);
                    ui.add_space(SP_12);

                    horizontal_row(ui, 40.0, |ui| {
                        if ui.add(modal_secondary_button("View on GitHub")).clicked() {
                            let url = if release.html_url.is_empty() {
                                GITHUB_RELEASES_PAGE_URL
                            } else {
                                &release.html_url
                            };
                            open_url_in_browser(url);
                        }

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if let Some((_ver, exe_path)) = self.pending_update_restart.clone() {
                                if ui.add(modal_primary_button("Restart Now", true)).clicked() {
                                    if let Err(err) = restart_application(&exe_path) {
                                        self.update_error = Some(err);
                                    } else {
                                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                    }
                                }
                                if ui.add(modal_secondary_button("Dismiss (Restart on Finish)")).clicked() {
                                    self.show_update_modal = false;
                                }
                            } else {
                                let update_btn_label = if self.update_downloading {
                                    "Updating in Background..."
                                } else if self.has_active_deployments() {
                                    "Update (Safe Restart)"
                                } else {
                                    "Update & Restart"
                                };

                                let btn_resp = ui.add(modal_primary_button(update_btn_label, !self.update_downloading));

                                if btn_resp.clicked() && !self.update_downloading {
                                    self.update_error = None;
                                    if release.download_url.trim().is_empty() {
                                        let url = if release.html_url.is_empty() {
                                            GITHUB_RELEASES_PAGE_URL
                                        } else {
                                            &release.html_url
                                        };
                                        open_url_in_browser(url);
                                    } else {
                                        self.update_downloading = true;
                                        self.update_progress = 0.02;
                                        self.update_status_text =
                                            format!("Starting download of {}...", release.asset_name);
                                        spawn_download_and_apply_update(release, self.updater_tx.clone());
                                    }
                                }

                                let cancel_label = if self.update_downloading {
                                    "Dismiss / Background"
                                } else {
                                    "Remind Me Later"
                                };
                                if ui.add(modal_secondary_button(cancel_label)).clicked() {
                                    self.show_update_modal = false;
                                }
                            }
                        });
                    });
                });
        } else {
            // Up-to-date Update Information Dialog (when clicking notification bell or checking)
            let (window, inner_w) = animated_modal_window(ctx, "Update Information", anim_t, None);
            window.show(ctx, |ui| {
                ui.set_width(inner_w);
                    if render_modal_header(
                        ui,
                        "close_update_info_modal",
                        "Update Information",
                        "Check for newer versions published on GitHub Releases.",
                    ) {
                        self.show_update_modal = false;
                    }

                    ui.add_space(SP_12);

                    egui::Frame::new()
                        .fill(SUCCESS_PILL_BG)
                        .stroke(Stroke::new(1.0_f32, SUCCESS_SOFT))
                        .corner_radius(R_LG)
                        .inner_margin(Margin::symmetric(14, 12))
                        .show(ui, |ui| {
                            horizontal_row(ui, 24.0, |ui| {
                                kit::status_badge(ui, StatusKind::Success, "Up to date");
                                ui.label(
                                    RichText::new(format!("Shiply v{APP_VERSION}"))
                                        .typo(Typo::Label).color(SUCCESS_PILL_TEXT),
                                );
                            });
                            ui.add_space(SP_4);
                            ui.label(
                                RichText::new(format!(
                                    "You are running the latest version of Shiply. Update checks query {GITHUB_REPO} on startup."
                                ))
                                .typo(Typo::Small)
                                .color(SUCCESS),
                            );
                        });

                    if self.update_checking {
                        ui.add_space(SP_8);
                        ui.label(
                            RichText::new("Checking GitHub Releases...")
                                .typo(Typo::Small)
                                .color(ACCENT),
                        );
                    }

                    ui.add_space(SP_16);
                    draw_divider(ui, MD_BORDER);
                    ui.add_space(SP_12);

                    horizontal_row(ui, 40.0, |ui| {
                        if ui.add(modal_secondary_button("View on GitHub")).clicked() {
                            open_url_in_browser(GITHUB_RELEASES_PAGE_URL);
                        }

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.add(modal_secondary_button("Close")).clicked() {
                                self.show_update_modal = false;
                            }

                            if ui
                                .add(modal_primary_button("Check Again", !self.update_checking))
                                .clicked()
                            {
                                spawn_update_check(self.updater_tx.clone(), true);
                            }
                        });
                    });
                });
        }
    }
}
