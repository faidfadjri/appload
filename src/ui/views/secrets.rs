//! Secrets management page and helpers.

use super::*;

impl ShiplyApp {
    pub(crate) fn render_secret_management_view(&mut self, ui: &mut egui::Ui) {
        // Vault-status caching: refresh only if dirty, empty, or 10s elapsed
        let now = Instant::now();
        let needs_refresh = self.secret_vault_cache_dirty
            || (self.secret_vault_cache.is_empty() && !self.storage.secrets.is_empty())
            || self
                .secret_vault_last_refresh
                .is_none_or(|t| now.duration_since(t) >= Duration::from_secs(10));
        if needs_refresh {
            self.refresh_secret_vault_cache();
        }

        let avail_w = ui.available_width();
        let is_compact_hdr = avail_w < 780.0;
        let os_vault_badge = if cfg!(windows) {
            "Windows Credential Manager"
        } else if cfg!(target_os = "macos") {
            "macOS Keychain"
        } else {
            "Linux Secret Service"
        };

        // 1. Page Header
        horizontal_row(ui, 38.0, |ui| {
            ui.label(
                RichText::new("Secret Management")
                    .typo(Typo::H1)
                    .color(TEXT_PRIMARY),
            );
            if !is_compact_hdr {
                ui.add_space(SP_8);
                kit::status_badge(ui, StatusKind::Success, os_vault_badge);
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(SP_8, 0.0);

                let new_btn_txt = if is_compact_hdr {
                    "+ New"
                } else {
                    "+ New secret"
                };
                if kit::button(
                    ui,
                    ButtonVariant::Primary,
                    ButtonSize::Md,
                    new_btn_txt,
                    None,
                )
                .on_hover_text("Open form to add a new secret to the OS Credential Manager")
                .clicked()
                {
                    self.open_new_secret_modal();
                }

                #[cfg(windows)]
                {
                    let open_lbl = if is_compact_hdr {
                        "OS Vault"
                    } else {
                        "Open Credential Manager"
                    };
                    if kit::button(
                        ui,
                        ButtonVariant::Secondary,
                        ButtonSize::Md,
                        open_lbl,
                        Some(OutlineIcon::Key),
                    )
                    .on_hover_text("Open Windows Control Panel Credential Manager")
                    .clicked()
                    {
                        open_windows_credential_manager();
                    }
                }
            });
        });

        ui.label(
            RichText::new("Credentials and tokens for your pipelines, stored in your OS vault, never in Shiply's database.")
                .typo(Typo::Small)
                .color(TEXT_SECONDARY),
        );
        ui.add_space(SP_16);

        // 2. Summary Strip (Stat Tiles)
        let total_secrets = self.storage.secrets.len();
        let stored_count = self
            .storage
            .secrets
            .iter()
            .filter(|s| {
                self.secret_vault_cache
                    .get(&s.key)
                    .is_some_and(|c| c.in_vault)
            })
            .count();
        let missing_count = total_secrets.saturating_sub(stored_count);

        let mut referenced_envs = HashSet::new();
        for s in &self.storage.secrets {
            for env_ref in self.storage.environments_using_secret(&s.key) {
                referenced_envs.insert(env_ref);
            }
        }
        let used_envs_count = referenced_envs.len();

        let is_narrow_tiles = avail_w < 640.0;
        let num_cols = if is_narrow_tiles { 2 } else { 4 };
        let tile_gap = SP_12;
        let tile_w = ((avail_w - (num_cols as f32 - 1.0) * tile_gap) / num_cols as f32).max(120.0);

        let stats = [
            ("Total secrets", total_secrets, TEXT_PRIMARY),
            ("Stored in vault", stored_count, SUCCESS),
            (
                "Missing in vault",
                missing_count,
                if missing_count > 0 {
                    WARNING
                } else {
                    TEXT_MUTED
                },
            ),
            ("Used by environments", used_envs_count, TEXT_PRIMARY),
        ];

        if is_narrow_tiles {
            for chunk in stats.chunks(2) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(tile_gap, 0.0);
                    for (lbl, val, col) in chunk {
                        render_secret_stat_tile(ui, tile_w, lbl, *val, *col);
                    }
                });
                ui.add_space(SP_8);
            }
        } else {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(tile_gap, 0.0);
                for (lbl, val, col) in &stats {
                    render_secret_stat_tile(ui, tile_w, lbl, *val, *col);
                }
            });
        }

        ui.add_space(SP_16);

        // 3. Secrets Table Card
        let query = self.secret_search_query.trim().to_lowercase();
        let filtered_secrets: Vec<SecretEntry> = self
            .storage
            .secrets
            .iter()
            .filter(|s| {
                // Filter by search query
                let matches_query = query.is_empty()
                    || s.key.to_lowercase().contains(&query)
                    || s.description.to_lowercase().contains(&query)
                    || s.username
                        .as_deref()
                        .is_some_and(|u| u.to_lowercase().contains(&query));
                if !matches_query {
                    return false;
                }

                // Filter by status tab
                let in_vault = self
                    .secret_vault_cache
                    .get(&s.key)
                    .is_some_and(|c| c.in_vault);
                match self.secret_filter_status {
                    SecretFilterStatus::All => true,
                    SecretFilterStatus::Stored => in_vault,
                    SecretFilterStatus::Missing => !in_vault,
                }
            })
            .cloned()
            .collect();

        let is_stacked_view =
            ui.ctx().screen_rect().width() < 1050.0 || ui.available_width() < 720.0;
        let mut edit_target: Option<(String, String, String)> = None;
        let mut delete_target_key: Option<String> = None;
        let mut copied_expr: Option<String> = None;

        kit::card(ui, |ui| {
            // Card Toolbar
            horizontal_row(ui, 36.0, |ui| {
                let search_w = if is_compact_hdr { 140.0 } else { 200.0 };
                styled_text_edit(
                    ui,
                    "secret_mgmt_search_input",
                    &mut self.secret_search_query,
                    search_w,
                    Some("Search secrets..."),
                );

                ui.add_space(SP_12);

                let mut filter_idx = match self.secret_filter_status {
                    SecretFilterStatus::All => 0,
                    SecretFilterStatus::Stored => 1,
                    SecretFilterStatus::Missing => 2,
                };
                if kit::segmented(ui, &mut filter_idx, &["All", "Stored", "Missing"]) {
                    self.secret_filter_status = match filter_idx {
                        0 => SecretFilterStatus::All,
                        1 => SecretFilterStatus::Stored,
                        _ => SecretFilterStatus::Missing,
                    };
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(SP_8, 0.0);

                    if kit::icon_button(
                        ui,
                        OutlineIcon::Refresh,
                        ButtonSize::Sm,
                        "Refresh OS vault status",
                    )
                    .clicked()
                    {
                        self.refresh_secret_vault_cache();
                    }

                    let count_str = if filtered_secrets.len() == 1 {
                        "1 secret".to_string()
                    } else {
                        format!("{} secrets", filtered_secrets.len())
                    };
                    ui.label(Typo::Small.rich(count_str).color(TEXT_MUTED));
                });
            });

            ui.add_space(SP_12);

            let table_avail_w = ui.available_width();
            let key_col_w = (table_avail_w - (120.0 + 140.0 + 190.0 + 130.0 + 110.0)).max(150.0);

            // Table Column Header
            if !is_stacked_view {
                egui::Frame::new()
                    .fill(SUBTLE_BG)
                    .inner_margin(Margin::symmetric(12, 8))
                    .corner_radius(CornerRadius::same(R_SM as u8))
                    .show(ui, |ui| {
                        horizontal_row(ui, 20.0, |ui| {
                            ui.add_sized(
                                [key_col_w, 20.0],
                                egui::Label::new(
                                    Typo::SmallMedium.rich("Name").color(TEXT_SECONDARY),
                                ),
                            );
                            ui.add_sized(
                                [120.0, 20.0],
                                egui::Label::new(
                                    Typo::SmallMedium.rich("Username").color(TEXT_SECONDARY),
                                ),
                            );
                            ui.add_sized(
                                [140.0, 20.0],
                                egui::Label::new(
                                    Typo::SmallMedium.rich("Status").color(TEXT_SECONDARY),
                                ),
                            );
                            ui.add_sized(
                                [190.0, 20.0],
                                egui::Label::new(
                                    Typo::SmallMedium.rich("Used by").color(TEXT_SECONDARY),
                                ),
                            );
                            ui.add_sized(
                                [130.0, 20.0],
                                egui::Label::new(
                                    Typo::SmallMedium.rich("Updated").color(TEXT_SECONDARY),
                                ),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(Typo::SmallMedium.rich("Actions").color(TEXT_SECONDARY));
                            });
                        });
                    });
                ui.add_space(4.0);
            }

            if self.storage.secrets.is_empty() {
                if kit::empty_state(
                    ui,
                    OutlineIcon::Key,
                    "No secrets yet",
                    "Credentials and tokens are saved directly into your OS vault, never touching Shiply's database.",
                    Some("+ Create your first secret"),
                ) {
                    self.open_new_secret_modal();
                }
            } else if filtered_secrets.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(SP_24);
                    let no_match_msg = if !query.is_empty() {
                        format!("No secrets match '{query}'")
                    } else {
                        "No secrets match the selected filter".to_string()
                    };
                    ui.label(Typo::Body.rich(no_match_msg).color(TEXT_MUTED));
                    ui.add_space(SP_8);
                    if kit::button(
                        ui,
                        ButtonVariant::Ghost,
                        ButtonSize::Sm,
                        "Clear filters",
                        None,
                    )
                    .clicked()
                    {
                        self.secret_search_query.clear();
                        self.secret_filter_status = SecretFilterStatus::All;
                    }
                    ui.add_space(SP_24);
                });
            }

            for (idx, sec) in filtered_secrets.iter().enumerate() {
                if idx > 0 {
                    draw_divider(ui, SUBTLE_BG);
                }
                ui.add_space(2.0);

                let cached = self.secret_vault_cache.get(&sec.key);
                let in_vault = cached.is_some_and(|c| c.in_vault);
                let effective_user = cached
                    .and_then(|c| c.username.clone())
                    .or_else(|| sec.username.clone())
                    .unwrap_or_else(|| "—".to_string());

                // Gather referencing environments as concise badges with project context
                let mut referencing_envs: Vec<(String, String)> = Vec::new();
                for proj in &self.storage.projects {
                    for env in &proj.environments {
                        if env.vault_target.trim().eq_ignore_ascii_case(&sec.key) {
                            let badge = crate::domain::format_env_badge(&env.key);
                            let ctx = format!("{}: {}", proj.name, env.name);
                            referencing_envs.push((badge, ctx));
                        }
                    }
                }
                let full_used_tip = if referencing_envs.is_empty() {
                    "Not used in any project environment".to_string()
                } else {
                    referencing_envs
                        .iter()
                        .map(|(_, ctx)| ctx.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                if is_stacked_view {
                    horizontal_row(ui, 24.0, |ui| {
                        ui.label(Typo::Mono.rich(&sec.key).color(TEXT_PRIMARY));
                        ui.add_space(SP_8);
                        kit::status_badge(
                            ui,
                            if in_vault {
                                StatusKind::Success
                            } else {
                                StatusKind::Warning
                            },
                            if in_vault { "Stored" } else { "Missing" },
                        );
                    });
                    if !sec.description.is_empty() {
                        ui.label(Typo::Small.rich(&sec.description).color(TEXT_SECONDARY));
                    }
                    horizontal_row(ui, 28.0, |ui| {
                        ui.label(
                            Typo::Caption
                                .rich(format!("User: {effective_user}"))
                                .color(TEXT_MUTED),
                        );
                        if !referencing_envs.is_empty() {
                            ui.add_space(SP_8);
                            for (badge, _) in referencing_envs.iter().take(2) {
                                kit::tag(ui, badge, env_pastel_pill_colors(badge));
                                ui.add_space(2.0);
                            }
                            if referencing_envs.len() > 2 {
                                let rem = referencing_envs.len() - 2;
                                kit::count_badge(ui, rem);
                            }
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(SP_8, 0.0);
                            if kit::danger_icon_button(
                                ui,
                                OutlineIcon::Trash,
                                ButtonSize::Sm,
                                "Delete secret from OS Credential Manager",
                            )
                            .clicked()
                            {
                                delete_target_key = Some(sec.key.clone());
                            }
                            let copy_tip =
                                format!("Copy ${{{{ secrets.{} }}}} to clipboard", sec.key);
                            if kit::icon_button(ui, OutlineIcon::Copy, ButtonSize::Sm, &copy_tip)
                                .clicked()
                            {
                                copied_expr = Some(format!("${{{{ secrets.{} }}}}", sec.key));
                            }
                            if kit::icon_button(
                                ui,
                                OutlineIcon::Edit,
                                ButtonSize::Sm,
                                "Edit / Rotate secret in OS Credential Manager",
                            )
                            .clicked()
                            {
                                edit_target = Some((
                                    sec.key.clone(),
                                    sec.username.clone().unwrap_or_default(),
                                    sec.description.clone(),
                                ));
                            }
                        });
                    });
                } else {
                    let row_h = if sec.description.is_empty() {
                        48.0_f32
                    } else {
                        56.0_f32
                    };
                    let (row_rect, row_resp) =
                        ui.allocate_exact_size(Vec2::new(table_avail_w, row_h), Sense::click());
                    let is_hovered = row_resp.hovered();

                    if is_hovered {
                        ui.painter().rect_filled(
                            row_rect,
                            CornerRadius::same(R_SM as u8),
                            SUBTLE_BG,
                        );
                    }

                    if row_resp.clicked() {
                        edit_target = Some((
                            sec.key.clone(),
                            sec.username.clone().unwrap_or_default(),
                            sec.description.clone(),
                        ));
                    }

                    let row_ui = &mut ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(row_rect)
                            .layout(Layout::left_to_right(Align::Center)),
                    );

                    horizontal_row(row_ui, row_h, |ui| {
                        ui.add_space(8.0);

                        // 1. NAME column
                        let (k_rect, _) = ui
                            .allocate_exact_size(Vec2::new(key_col_w - 8.0, row_h), Sense::hover());
                        let name_top = if sec.description.is_empty() {
                            k_rect.center().y
                        } else {
                            k_rect.top() + 15.0
                        };
                        ui.painter().with_clip_rect(k_rect).text(
                            Pos2::new(k_rect.left(), name_top),
                            Align2::LEFT_CENTER,
                            &sec.key,
                            Typo::Mono.font(),
                            TEXT_PRIMARY,
                        );
                        if !sec.description.is_empty() {
                            ui.painter().with_clip_rect(k_rect).text(
                                Pos2::new(k_rect.left(), k_rect.top() + 37.0),
                                Align2::LEFT_CENTER,
                                &sec.description,
                                Typo::Small.font(),
                                TEXT_MUTED,
                            );
                        }

                        // 2. USERNAME column
                        let (u_rect, _) =
                            ui.allocate_exact_size(Vec2::new(120.0, row_h), Sense::hover());
                        ui.painter().with_clip_rect(u_rect).text(
                            Pos2::new(u_rect.left(), u_rect.center().y),
                            Align2::LEFT_CENTER,
                            &effective_user,
                            Typo::Small.font(),
                            TEXT_SECONDARY,
                        );

                        // 3. STATUS column
                        let (s_rect, _) =
                            ui.allocate_exact_size(Vec2::new(140.0, row_h), Sense::hover());
                        let mut s_child = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(s_rect)
                                .layout(Layout::left_to_right(Align::Center)),
                        );
                        kit::status_badge(
                            &mut s_child,
                            if in_vault {
                                StatusKind::Success
                            } else {
                                StatusKind::Warning
                            },
                            if in_vault { "Stored" } else { "Missing" },
                        );

                        // 4. USED BY column
                        let (ub_rect, ub_resp) =
                            ui.allocate_exact_size(Vec2::new(190.0, row_h), Sense::hover());
                        let mut ub_child = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(ub_rect)
                                .layout(Layout::left_to_right(Align::Center)),
                        );
                        if referencing_envs.is_empty() {
                            ub_child.label(Typo::Small.rich("Not used").color(TEXT_MUTED));
                        } else {
                            for (badge, _) in referencing_envs.iter().take(2) {
                                kit::tag(&mut ub_child, badge, env_pastel_pill_colors(badge));
                                ub_child.add_space(4.0);
                            }
                            if referencing_envs.len() > 2 {
                                let rem = referencing_envs.len() - 2;
                                kit::count_badge(&mut ub_child, rem);
                            }
                        }
                        ub_resp.on_hover_text(&full_used_tip);

                        // 5. UPDATED column
                        let (t_rect, t_resp) =
                            ui.allocate_exact_size(Vec2::new(130.0, row_h), Sense::hover());
                        let rel_time = helpers::format_relative_time(sec.updated_at);
                        ui.painter().text(
                            Pos2::new(t_rect.left(), t_rect.center().y),
                            Align2::LEFT_CENTER,
                            rel_time,
                            Typo::Small.font(),
                            TEXT_MUTED,
                        );
                        t_resp.on_hover_text(sec.format_updated_at_local("%Y-%m-%d %H:%M"));

                        // 6. ACTIONS column
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(SP_8, 0.0);
                            if kit::danger_icon_button(
                                ui,
                                OutlineIcon::Trash,
                                ButtonSize::Sm,
                                "Delete secret from OS Credential Manager",
                            )
                            .clicked()
                            {
                                delete_target_key = Some(sec.key.clone());
                            }
                            let copy_tip =
                                format!("Copy ${{{{ secrets.{} }}}} to clipboard", sec.key);
                            if kit::icon_button(ui, OutlineIcon::Copy, ButtonSize::Sm, &copy_tip)
                                .clicked()
                            {
                                copied_expr = Some(format!("${{{{ secrets.{} }}}}", sec.key));
                            }
                            if kit::icon_button(
                                ui,
                                OutlineIcon::Edit,
                                ButtonSize::Sm,
                                "Edit / Rotate secret in OS Credential Manager",
                            )
                            .clicked()
                            {
                                edit_target = Some((
                                    sec.key.clone(),
                                    sec.username.clone().unwrap_or_default(),
                                    sec.description.clone(),
                                ));
                            }
                        });
                    });
                }
                ui.add_space(2.0);
            }
        });

        if let Some((k, u, d)) = edit_target {
            self.open_secret_management_for_target(&k, &u, &d);
        } else if let Some(del_key) = delete_target_key {
            self.pending_delete_confirm = Some(ConfirmDeleteAction::Secret { key: del_key });
        } else if let Some(expr) = copied_expr {
            ui.ctx().copy_text(expr.clone());
            self.status_banner = Some((format!("Copied '{expr}' to clipboard!"), true));
            self.status_banner_time = Some(Instant::now());
        }

        ui.add_space(SP_16);
        self.render_secret_storage_help(ui);
    }

    pub(crate) fn render_secret_storage_help(&mut self, ui: &mut egui::Ui) {
        let target_example = if self.cred_guide_target_input.trim().is_empty() {
            "prod-ssh-server".to_string()
        } else {
            self.cred_guide_target_input.trim().to_string()
        };
        let user_example = if self.cred_guide_user_input.trim().is_empty() {
            "deploy".to_string()
        } else {
            self.cred_guide_user_input.trim().to_string()
        };

        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                let (ic_r, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
                draw_outline_icon(ui.painter(), ic_r.center(), OutlineIcon::Key, PRIMARY_BLUE);
                ui.add_space(4.0);
                ui.label(Typo::H3.rich("How Shiply stores secrets"));
                ui.add_space(SP_8);
                ui.label(
                    Typo::Small
                        .rich("Encrypted in your operating system's native credential vault")
                        .color(TEXT_MUTED),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let toggle_lbl = if self.secret_help_expanded {
                        "Hide"
                    } else {
                        "Show"
                    };
                    if kit::button(ui, ButtonVariant::Ghost, ButtonSize::Sm, toggle_lbl, None)
                        .clicked()
                    {
                        self.secret_help_expanded = !self.secret_help_expanded;
                    }
                });
            });

            if !self.secret_help_expanded {
                return;
            }

            ui.add_space(SP_12);
            let line_w = ui.available_width();
            let (line_rect, _) = ui.allocate_exact_size(Vec2::new(line_w, 1.0), Sense::hover());
            ui.painter().line_segment(
                [line_rect.left_center(), line_rect.right_center()],
                Stroke::new(1.0_f32, CARD_BORDER),
            );
            ui.add_space(SP_12);

            self.render_os_credentials_guide_body(ui, &target_example, &user_example);
        });
    }
}

fn render_secret_stat_tile(
    ui: &mut egui::Ui,
    width: f32,
    label: &str,
    value: usize,
    val_color: Color32,
) {
    egui::Frame::new()
        .fill(CARD_BG)
        .stroke(Stroke::new(1.0_f32, CARD_BORDER))
        .corner_radius(CornerRadius::same(R_LG as u8))
        .inner_margin(Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.set_width(width - 32.0);
            ui.vertical(|ui| {
                ui.label(Typo::Small.rich(label).color(TEXT_MUTED));
                ui.add_space(4.0);
                ui.label(Typo::H2.rich(value.to_string()).color(val_color));
            });
        });
}
