//! Version synchronization candidates and dropdown.

use super::*;

impl ShiplyApp {
    pub fn sync_version_candidates(&self, project_id: &str) -> Vec<SyncVersionCandidate> {
        let mut candidates = Vec::new();
        let Some(proj) = self.storage.projects.iter().find(|p| p.id == project_id) else {
            return candidates;
        };

        // 1. Collect latest deployed versions from other environments in this project
        for env in &proj.environments {
            if env.key == self.selected_env_key {
                continue;
            }
            if let Some(dep) = self
                .storage
                .last_succeeded_deployment(project_id, &env.key)
                .or_else(|| self.storage.latest_deployment(project_id, &env.key))
            {
                let ver = dep.version.trim().to_string();
                if !ver.is_empty()
                    && !candidates.iter().any(|c: &SyncVersionCandidate| {
                        c.version == ver && c.source_name == env.name
                    })
                {
                    candidates.push(SyncVersionCandidate {
                        source_name: env.name.clone(),
                        env_key: Some(env.key.clone()),
                        version: ver,
                        is_latest: false,
                    });
                }
            }
        }

        // 2. Fall back to package.json if present and not already in candidates
        if let Some(pkg_ver) = &self.cached_pkg_version {
            let ver = pkg_ver.trim().to_string();
            if !ver.is_empty() && ver != "0.0.0" && !candidates.iter().any(|c| c.version == ver) {
                candidates.push(SyncVersionCandidate {
                    source_name: "package.json".to_string(),
                    env_key: None,
                    version: ver,
                    is_latest: false,
                });
            }
        }

        // 3. Mark the most recent candidate as is_latest
        if !candidates.is_empty() {
            let most_recent_ver = self
                .storage
                .deployments
                .iter()
                .filter(|d| d.project_id == project_id && d.environment != self.selected_env_key)
                .max_by_key(|d| d.started_at)
                .map(|d| d.version.clone());

            let mut matched = false;
            if let Some(recent_ver) = most_recent_ver {
                if let Some(cand) = candidates.iter_mut().find(|c| c.version == recent_ver) {
                    cand.is_latest = true;
                    matched = true;
                }
            }
            if !matched {
                candidates[0].is_latest = true;
            }
        }

        candidates
    }

    pub fn apply_synced_version(&mut self, version: &str, source_name: &str) {
        self.deploy_version_input = version.trim().to_string();
        self.status_banner = Some((
            format!(
                "Synced version to v{} from {}.",
                version.trim(),
                source_name
            ),
            true,
        ));
        self.sync_version_dropdown_open = false;
    }

    pub(crate) fn render_sync_version_dropdown(
        &mut self,
        ctx: &egui::Context,
        trigger_rect: Rect,
        project_id: &str,
    ) {
        if !self.sync_version_dropdown_open {
            return;
        }

        let candidates = self.sync_version_candidates(project_id);
        let item_h = 32.0_f32;
        let header_h = 28.0_f32;
        let pad = 6.0_f32;
        let row_count = if candidates.is_empty() {
            1
        } else {
            candidates.len()
        };
        let menu_w = 270.0_f32;
        let menu_h = pad * 2.0
            + header_h
            + (row_count as f32 * item_h)
            + ((row_count.saturating_sub(1)) as f32 * 4.0);

        let screen = ctx.screen_rect();
        let mut menu_pos = Pos2::new(trigger_rect.left(), trigger_rect.bottom() + 4.0);
        if menu_pos.x + menu_w > screen.right() - 10.0 {
            menu_pos.x = (trigger_rect.right() - menu_w).max(screen.left() + 10.0);
        }
        if menu_pos.y + menu_h > screen.bottom() - 10.0 {
            menu_pos.y = (trigger_rect.top() - menu_h - 4.0).max(screen.top() + 10.0);
        }
        let menu_rect = Rect::from_min_size(menu_pos, Vec2::new(menu_w, menu_h));

        // Outside click to close
        if ctx.input(|i| i.pointer.any_pressed()) {
            if let Some(ptr) = ctx.input(|i| i.pointer.latest_pos()) {
                if !menu_rect.contains(ptr) && !trigger_rect.contains(ptr) {
                    self.sync_version_dropdown_open = false;
                    return;
                }
            }
        }

        // Close on Esc / Tab
        if ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Tab)) {
            self.sync_version_dropdown_open = false;
            return;
        }

        let mut chosen = None;

        egui::Area::new(egui::Id::new("sync_version_dropdown_area"))
            .order(egui::Order::Foreground)
            .fixed_pos(menu_pos)
            .show(ctx, |ui| {
                let shadow_rect = menu_rect.expand(2.0);
                ui.painter().rect_filled(
                    shadow_rect,
                    CornerRadius::same(8),
                    Color32::from_black_alpha(45),
                );

                ui.painter()
                    .rect_filled(menu_rect, CornerRadius::same(8), CARD_BG);
                ui.painter().rect_stroke(
                    menu_rect,
                    CornerRadius::same(8),
                    Stroke::new(1.0_f32, CARD_BORDER),
                    StrokeKind::Inside,
                );

                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(menu_rect.shrink(pad))
                        .layout(Layout::top_down(Align::Min)),
                );

                horizontal_row(&mut child, 22.0, |ui| {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Sync version across environments")
                            .typo(Typo::Caption)
                            .color(TEXT_MUTED),
                    );
                });
                draw_divider(&mut child, SUBTLE_BG);
                child.add_space(4.0);

                if candidates.is_empty() {
                    child.add_space(4.0);
                    child.horizontal(|ui| {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("No other deployed versions found.")
                                .typo(Typo::Small)
                                .color(TEXT_MUTED),
                        );
                    });
                } else {
                    for cand in &candidates {
                        let is_current = self.deploy_version_input.trim() == cand.version.trim();
                        let (rect, resp) = child.allocate_exact_size(
                            Vec2::new(child.available_width(), 32.0),
                            Sense::click(),
                        );
                        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        let bg = if resp.hovered() {
                            SUBTLE_BG
                        } else if cand.is_latest {
                            INFO_SOFT
                        } else {
                            Color32::TRANSPARENT
                        };
                        child.painter().rect_filled(rect, CornerRadius::same(6), bg);

                        // Icon on left
                        let icon_c = Pos2::new(rect.left() + 16.0, rect.center().y);
                        draw_outline_icon(
                            child.painter(),
                            icon_c,
                            if cand.env_key.is_some() {
                                OutlineIcon::Refresh
                            } else {
                                OutlineIcon::Folder
                            },
                            if cand.is_latest {
                                PRIMARY_BLUE
                            } else {
                                TEXT_SECONDARY
                            },
                        );

                        // Source name
                        let name_text = if cand.is_latest {
                            format!("{} (Latest)", cand.source_name)
                        } else {
                            cand.source_name.clone()
                        };
                        child.painter().text(
                            Pos2::new(rect.left() + 32.0, rect.center().y),
                            Align2::LEFT_CENTER,
                            name_text,
                            Typo::Small.font(),
                            if cand.is_latest {
                                PRIMARY_BLUE
                            } else {
                                TEXT_PRIMARY
                            },
                        );

                        // Version badge on right
                        let ver_text = format!("v{}", cand.version);
                        let ver_color = if is_current { SUCCESS } else { TEXT_PRIMARY };
                        child.painter().text(
                            Pos2::new(rect.right() - 10.0, rect.center().y),
                            Align2::RIGHT_CENTER,
                            ver_text,
                            Typo::Small.font(),
                            ver_color,
                        );

                        if resp.clicked() {
                            chosen = Some((cand.version.clone(), cand.source_name.clone()));
                        }
                    }
                }
            });

        if let Some((v, s)) = chosen {
            self.apply_synced_version(&v, &s);
            ctx.request_repaint();
        }
    }
}
