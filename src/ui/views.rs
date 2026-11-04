use super::*;

impl ShiplyApp {
    pub(crate) fn render_first_time_empty_state(&mut self, ui: &mut egui::Ui) {
        ui.add_space(SP_32);
        let avail_w = ui.available_width();
        let content_w = avail_w.min(680.0);
        let pad_x = ((avail_w - content_w) / 2.0).max(0.0);

        ui.horizontal(|ui| {
            if pad_x > 0.0 {
                ui.add_space(pad_x);
            }
            ui.vertical(|ui| {
                ui.set_max_width(content_w);
                kit::card(ui, |ui| {
                    if kit::empty_state(
                        ui,
                        OutlineIcon::Plus,
                        "Welcome to Shiply",
                        "No projects or environments configured yet. Auto-import a PowerShell deployment script (.ps1) or create a project to get started.",
                        Some("Add your first project"),
                    ) {
                        self.open_new_project_modal();
                    }
                });
            });
        });

        ui.add_space(SP_16);
        self.render_onboarding_guideline_card(ui);
        ui.add_space(SP_16);
        self.render_os_credentials_guideline_card(ui);
    }
    pub(crate) fn render_all_deployments_overview(&mut self, ui: &mut egui::Ui) {
        let mut open_bulk_modal = false;
        let mut clear_selection = false;
        let all_project_ids: Vec<String> =
            self.storage.projects.iter().map(|p| p.id.clone()).collect();
        let selected_count = all_project_ids
            .iter()
            .filter(|id| self.bulk_selected_projects.contains(*id))
            .count();

        // Page Header
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(Typo::H1.rich("All deployments"));
                    if selected_count > 0 {
                        ui.add_space(SP_8);
                        kit::count_badge(ui, selected_count);
                        ui.label(Typo::Small.rich("selected").color(PRIMARY_BLUE));
                    }
                });
                ui.add_space(2.0);
                ui.label(
                    Typo::Small
                        .rich("Select projects to deploy together, or open one to manage it.")
                        .color(TEXT_SECONDARY),
                );
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(SP_8, 0.0);

                let deploy_label = if selected_count > 0 {
                    format!("Deploy selected ({selected_count})")
                } else {
                    "Bulk deploy...".to_string()
                };
                if kit::button(
                    ui,
                    ButtonVariant::Primary,
                    ButtonSize::Md,
                    &deploy_label,
                    Some(OutlineIcon::Play),
                )
                .on_hover_text("Deploy multiple selected projects sequentially with auto-bumped versions")
                .clicked()
                {
                    open_bulk_modal = true;
                }

                if selected_count > 0
                    && kit::button(
                        ui,
                        ButtonVariant::Ghost,
                        ButtonSize::Sm,
                        "Clear selection",
                        None,
                    )
                    .on_hover_text("Deselect all selected projects")
                    .clicked()
                {
                    clear_selection = true;
                }
            });
        });

        if clear_selection {
            self.bulk_selected_projects.clear();
        }
        if open_bulk_modal {
            self.open_bulk_deploy_modal();
        }

        ui.add_space(SP_16);

        // Live Bulk Deployment Queue Progress Banner (when a bulk deployment is running)
        if !self.bulk_deploy_queue.is_empty()
            || (self.bulk_total_count > 0 && self.has_active_deployments())
        {
            let remaining = self.bulk_deploy_queue.len();
            let current_idx = self.bulk_total_count.saturating_sub(remaining);
            let total = self.bulk_total_count.max(1);
            let mut cancel_queue = false;

            egui::Frame::new()
                .fill(INFO_SOFT)
                .stroke(Stroke::new(1.0_f32, PRIMARY_BLUE))
                .corner_radius(CornerRadius::same(R_MD as u8))
                .inner_margin(Margin::symmetric(16, 10))
                .show(ui, |ui| {
                    horizontal_row(ui, 30.0, |ui| {
                        ui.ctx().request_repaint();
                        let time = ui.input(|i| i.time);
                        let (sp_rect, _) =
                            ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
                        draw_circle_loading_spinner(
                            ui.painter(),
                            sp_rect.center(),
                            5.0,
                            PRIMARY_BLUE,
                            time,
                        );
                        ui.add_space(4.0);
                        ui.label(
                            Typo::SmallMedium
                                .rich(format!(
                                    "Bulk deployment in progress — Running {current_idx} of {total} ({remaining} queued)"
                                ))
                                .color(PRIMARY_BLUE),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if remaining > 0
                                && kit::button(
                                    ui,
                                    ButtonVariant::Danger,
                                    ButtonSize::Sm,
                                    "Cancel queue",
                                    Some(OutlineIcon::Trash),
                                )
                                .clicked()
                            {
                                cancel_queue = true;
                            }
                        });
                    });
                });
            if cancel_queue {
                self.bulk_deploy_queue.clear();
                self.bulk_total_count = 0;
                self.status_banner = Some((
                    "Cleared remaining projects from the Bulk Deployment queue.".to_string(),
                    false,
                ));
            }
            ui.add_space(SP_12);
        }

        // Project Cards Grid
        let proj_count = self.storage.projects.len().max(1);
        let spacing = SP_16;
        let grid_avail_w = ui.available_width();
        let max_cols = if grid_avail_w >= 1000.0 {
            3
        } else if grid_avail_w >= 640.0 {
            2
        } else {
            1
        };
        let cols = proj_count.min(max_cols).max(1);
        let card_w = if cols == 1 {
            grid_avail_w
        } else {
            ((grid_avail_w - spacing * (cols - 1) as f32) / cols as f32).floor()
        };

        let mut clicked_proj = None;
        let mut toggled_bulk_proj = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(spacing, SP_16);
            for project in &self.storage.projects {
                let is_checked = self.bulk_selected_projects.contains(&project.id);
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(card_w, 112.0), Sense::click());
                let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

                // Checkbox hit target in top-right of the project card
                let chk_rect = Rect::from_center_size(
                    Pos2::new(rect.right() - 24.0, rect.top() + 24.0),
                    Vec2::new(20.0, 20.0),
                );
                let chk_resp = ui
                    .interact(
                        chk_rect,
                        egui::Id::new(("bulk_chk_card", &project.id)),
                        Sense::click(),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand);

                // Card frame
                let card_bg = if is_checked {
                    Color32::from_rgba_premultiplied(20, 110, 245, 12)
                } else {
                    CARD_BG
                };
                let border_stroke = Stroke::new(
                    if is_checked { 1.5_f32 } else { 1.0_f32 },
                    if is_checked {
                        PRIMARY_BLUE
                    } else if resp.hovered() {
                        INPUT_BORDER_HOVER
                    } else {
                        CARD_BORDER
                    },
                );

                ui.painter().rect_filled(rect, CornerRadius::same(R_LG as u8), card_bg);
                ui.painter().rect_stroke(rect, CornerRadius::same(R_LG as u8), border_stroke, StrokeKind::Inside);

                // Checkbox paint
                let chk_radius = CornerRadius::same(4);
                if is_checked {
                    ui.painter().rect_filled(chk_rect, chk_radius, PRIMARY_BLUE);
                    let p1 = Pos2::new(chk_rect.left() + 4.5, chk_rect.top() + 10.0);
                    let p2 = Pos2::new(chk_rect.left() + 8.0, chk_rect.top() + 14.0);
                    let p3 = Pos2::new(chk_rect.left() + 15.5, chk_rect.top() + 6.0);
                    ui.painter().line_segment([p1, p2], Stroke::new(1.8_f32, Color32::WHITE));
                    ui.painter().line_segment([p2, p3], Stroke::new(1.8_f32, Color32::WHITE));
                } else {
                    let chk_border = if chk_resp.hovered() { INPUT_BORDER_HOVER } else { INPUT_BORDER };
                    ui.painter().rect_filled(chk_rect, chk_radius, CARD_BG);
                    ui.painter().rect_stroke(chk_rect, chk_radius, Stroke::new(1.0_f32, chk_border), StrokeKind::Inside);
                }

                // Project Name and Path
                let text_clip = Rect::from_min_max(
                    Pos2::new(rect.left() + 16.0, rect.top() + 10.0),
                    Pos2::new((chk_rect.left() - 8.0).max(rect.left() + 40.0), rect.top() + 66.0),
                );
                let clipped_painter = ui.painter().with_clip_rect(text_clip);
                clipped_painter.text(
                    Pos2::new(rect.left() + 18.0, rect.top() + 24.0),
                    Align2::LEFT_CENTER,
                    &project.name,
                    Typo::H3.font(),
                    TEXT_PRIMARY,
                );
                let local_path_display = if project.local_path.len() > 36 {
                    format!("{}…", &project.local_path[..35])
                } else {
                    project.local_path.clone()
                };
                clipped_painter.text(
                    Pos2::new(rect.left() + 18.0, rect.top() + 48.0),
                    Align2::LEFT_CENTER,
                    &local_path_display,
                    Typo::MonoSm.font(),
                    TEXT_MUTED,
                );

                // Environment version tags row at bottom
                let mut px = rect.left() + 18.0;
                let py = rect.top() + 80.0;
                for env in &project.environments {
                    let badge = env.badge_label();
                    let is_env_running = self.storage.deployments.iter().any(|d| {
                        d.project_id == project.id
                            && d.environment == env.key
                            && d.status == DeploymentStatus::Running
                    });
                    let w = if is_env_running {
                        ui.ctx().request_repaint();
                        let time = ui.input(|i| i.time);
                        paint_running_pill_left_aligned(
                            ui.painter(),
                            Pos2::new(px, py),
                            &format!("{badge}: Running"),
                            WARN_PILL_BG,
                            WARN_PILL_TEXT,
                            time,
                        )
                    } else {
                        let (bg, fg) = env_pastel_pill_colors(&env.key);
                        let ver_label = self
                            .storage
                            .last_succeeded_deployment(&project.id, &env.key)
                            .map(|d| format!("{badge} v{}", d.version))
                            .unwrap_or_else(|| format!("{badge} —"));
                        paint_pill_left_aligned(ui.painter(), Pos2::new(px, py), &ver_label, bg, fg)
                    };
                    px += w + 6.0;
                }

                if chk_resp
                    .on_hover_text("Select or deselect this project for bulk deployment")
                    .clicked()
                {
                    toggled_bulk_proj = Some(project.id.clone());
                } else if resp.clicked() {
                    clicked_proj = Some(project.id.clone());
                }
            }
        });

        if let Some(tid) = toggled_bulk_proj {
            if self.bulk_selected_projects.contains(&tid) {
                self.bulk_selected_projects.remove(&tid);
            } else {
                self.bulk_selected_projects.insert(tid);
            }
        } else if let Some(pid) = clicked_proj {
            self.selection = SidebarSelection::Project(pid.clone());
            self.project_switch_loading_id = Some(pid);
            self.project_switch_loading_start = Some(Instant::now());
            self.refresh_selected_project_metadata(true);
            return;
        }

        ui.add_space(SP_16);
        self.render_onboarding_guideline_card(ui);
        ui.add_space(SP_16);
        self.render_os_credentials_guideline_card(ui);
        ui.add_space(SP_20);
        self.render_history_and_logs_view(ui, None);
    }

    pub(crate) fn render_onboarding_guideline_card(&mut self, ui: &mut egui::Ui) {
        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(Typo::H3.rich("Get started"));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let toggle_lbl = if self.show_onboarding_guide {
                        "Hide"
                    } else {
                        "Show"
                    };
                    if kit::button(
                        ui,
                        ButtonVariant::Ghost,
                        ButtonSize::Sm,
                        toggle_lbl,
                        None,
                    )
                    .clicked()
                    {
                        self.show_onboarding_guide = !self.show_onboarding_guide;
                    }
                });
            });

            if !self.show_onboarding_guide {
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

            let steps = [
                (
                    "1",
                    "Add or import project",
                    "Click '+ Add' in the sidebar or auto-import a deploy.ps1 script to populate project directories and environments.",
                ),
                (
                    "2",
                    "Set up OS credentials",
                    "Store your SSH password securely in Windows Credential Manager, Linux libsecret, or macOS Keychain.",
                ),
                (
                    "3",
                    "Configure pipeline steps",
                    "Customize Local Command, Verify Path, SCP Upload, and Remote SSH steps with syntax highlighting and variables.",
                ),
                (
                    "4",
                    "Deploy & rollback",
                    "Select Git branches, auto-bump versions, run sequential bulk deployments, or 1-click rollback anytime.",
                ),
            ];

            let spacing = SP_12;
            let onboard_avail = ui.available_width();
            let onboard_cols = if onboard_avail >= 860.0 {
                4
            } else if onboard_avail >= 460.0 {
                2
            } else {
                1
            };
            let step_w = if onboard_cols == 1 {
                onboard_avail
            } else {
                ((onboard_avail - spacing * (onboard_cols - 1) as f32) / onboard_cols as f32).floor()
            };
            const ONBOARDING_CARD_HEIGHT: f32 = 140.0;

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(spacing, SP_12);
                for (num, title, desc) in steps {
                    egui::Frame::new()
                        .fill(SUBTLE_BG)
                        .stroke(Stroke::new(1.0_f32, CARD_BORDER))
                        .corner_radius(CornerRadius::same(R_MD as u8))
                        .inner_margin(Margin::same(14))
                        .show(ui, |ui| {
                            ui.set_width((step_w - 28.0).max(150.0));
                            ui.set_height(ONBOARDING_CARD_HEIGHT);
                            ui.vertical(|ui| {
                                let (num_rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::hover());
                                ui.painter().circle_filled(num_rect.center(), 12.0, INFO_SOFT);
                                ui.painter().text(
                                    num_rect.center(),
                                    Align2::CENTER_CENTER,
                                    num,
                                    Typo::Caption.font(),
                                    PRIMARY_BLUE,
                                );
                                ui.add_space(8.0);
                                ui.label(Typo::H3.rich(title));
                                ui.add_space(4.0);
                                ui.label(Typo::Small.rich(desc).color(TEXT_SECONDARY));
                            });
                        });
                }
            });
        });
    }

    pub(crate) fn render_os_credentials_guideline_card(&mut self, ui: &mut egui::Ui) {
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
        let vault_ready = read_os_vault_credential(&target_example).is_some();

        kit::card(ui, |ui| {
            ui.horizontal(|ui| {
                let (ic_r, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
                draw_outline_icon(ui.painter(), ic_r.center(), OutlineIcon::Key, PRIMARY_BLUE);
                ui.add_space(4.0);
                ui.label(Typo::H3.rich("Set up OS credentials"));
                ui.add_space(SP_8);
                ui.label(
                    Typo::Small
                        .rich("Store SSH passwords securely in your OS vault without plain-text configs")
                        .color(TEXT_MUTED),
                );
                if vault_ready {
                    ui.add_space(SP_8);
                    kit::status_badge(ui, StatusKind::Success, "Vault ready");
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let toggle_lbl = if self.show_credential_guide {
                        "Hide"
                    } else {
                        "Show"
                    };
                    if kit::button(
                        ui,
                        ButtonVariant::Ghost,
                        ButtonSize::Sm,
                        toggle_lbl,
                        None,
                    )
                    .clicked()
                    {
                        self.show_credential_guide = !self.show_credential_guide;
                    }
                });
            });

            if !self.show_credential_guide {
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

    pub(crate) fn render_os_credentials_guide_body(
        &mut self,
        ui: &mut egui::Ui,
        target_example: &str,
        user_example: &str,
    ) {
        ui.label(
            Typo::Small
                .rich("Shiply reads your SSH password securely from your operating system's native credential vault so you never hardcode passwords in scripts or config files.")
                .color(TEXT_SECONDARY),
        );
        ui.add_space(SP_12);

        // Interactive inputs
        ui.horizontal(|ui| {
            ui.label(Typo::SmallMedium.rich("Vault target:").color(TEXT_SECONDARY));
            styled_text_edit(
                ui,
                "cred_guide_target_input",
                &mut self.cred_guide_target_input,
                170.0,
                Some("e.g. prod-ssh-server"),
            );
            ui.add_space(SP_8);
            ui.label(Typo::SmallMedium.rich("SSH user:").color(TEXT_SECONDARY));
            styled_text_edit(
                ui,
                "cred_guide_user_input",
                &mut self.cred_guide_user_input,
                130.0,
                Some("e.g. deploy"),
            );
            ui.add_space(SP_8);
            if kit::button(
                ui,
                ButtonVariant::Secondary,
                ButtonSize::Md,
                "Verify in OS Vault",
                Some(OutlineIcon::Refresh),
            )
            .clicked()
            {
                if read_os_vault_credential(target_example).is_some() {
                    self.status_banner = Some((
                        format!("Verified! Credential target '{target_example}' is ready in your OS Vault."),
                        true,
                    ));
                    self.status_banner_time = Some(Instant::now());
                } else {
                    self.status_banner = Some((
                        format!("Target '{target_example}' was not found in your OS Vault yet. Follow the steps below to add it."),
                        false,
                    ));
                    self.status_banner_time = Some(Instant::now());
                }
            }
        });

        ui.add_space(SP_16);

        // Segmented OS Tabs
        let mut os_idx = match self.cred_guide_os_tab {
            CredentialOsTab::Windows => 0,
            CredentialOsTab::Linux => 1,
            CredentialOsTab::Mac => 2,
        };
        if kit::segmented(
            ui,
            &mut os_idx,
            &[
                "Windows (Credential Manager)",
                "Linux (libsecret / secret-tool)",
                "Mac (macOS Keychain)",
            ],
        ) {
            self.cred_guide_os_tab = match os_idx {
                0 => CredentialOsTab::Windows,
                1 => CredentialOsTab::Linux,
                _ => CredentialOsTab::Mac,
            };
        }

        ui.add_space(SP_12);

        egui::Frame::new()
            .fill(SUBTLE_BG)
            .stroke(Stroke::new(1.0_f32, CARD_BORDER))
            .corner_radius(CornerRadius::same(R_MD as u8))
            .inner_margin(Margin::same(16))
            .show(ui, |ui| match self.cred_guide_os_tab {
                CredentialOsTab::Windows => {
                    let cmdkey_cmd = format!(
                        "cmdkey /generic:\"{target_example}\" /user:\"{user_example}\" /pass:\"YOUR_SSH_PASSWORD\""
                    );
                    ui.horizontal(|ui| {
                        ui.label(Typo::SmallMedium.rich("Option 1 — Windows Credential Manager (GUI):"));
                        #[cfg(windows)]
                        {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if kit::button(
                                    ui,
                                    ButtonVariant::Primary,
                                    ButtonSize::Sm,
                                    "Open Windows Credential Manager",
                                    None,
                                )
                                .clicked()
                                {
                                    open_windows_credential_manager();
                                }
                            });
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(
                        Typo::Body.rich(format!(
                            "1. Open Control Panel -> User Accounts -> Credential Manager -> Windows Credentials.\n2. Under 'Generic Credentials', click 'Add a generic credential'.\n3. Set Internet or network address = '{target_example}', User name = '{user_example}', and Password = your SSH password.\n4. Click OK, then click 'Verify in OS Vault' above."
                        )).color(TEXT_SECONDARY),
                    );
                    ui.add_space(SP_12);
                    ui.label(Typo::SmallMedium.rich("Option 2 — PowerShell / CMD One-Line Command:"));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let mut preview = cmdkey_cmd.clone();
                        syntax_highlighted_text_edit(
                            ui,
                            "cred_cmdkey_preview",
                            &mut preview,
                            (ui.available_width() - 90.0).max(220.0),
                            None,
                        );
                        if kit::button(
                            ui,
                            ButtonVariant::Secondary,
                            ButtonSize::Md,
                            "Copy",
                            Some(OutlineIcon::Copy),
                        )
                        .clicked()
                        {
                            ui.ctx().copy_text(cmdkey_cmd);
                            self.status_banner = Some((
                                "Copied Windows cmdkey command to clipboard!".to_string(),
                                true,
                            ));
                            self.status_banner_time = Some(Instant::now());
                        }
                    });
                }
                CredentialOsTab::Linux => {
                    let linux_cmd = format!(
                        "secret-tool store --label=\"Shiply ({target_example})\" target \"{target_example}\" username \"{user_example}\""
                    );
                    ui.label(Typo::SmallMedium.rich("Linux Secret Service (libsecret / GNOME Keyring / KWallet):"));
                    ui.add_space(4.0);
                    ui.label(
                        Typo::Body.rich(format!(
                            "1. Ensure libsecret-tools is installed (Debian/Ubuntu: sudo apt install libsecret-tools | Fedora: sudo dnf install libsecret).\n2. Run the command below in your terminal and enter your SSH password when prompted.\n3. In your Project's Settings, set OS Credential Vault Target = '{target_example}'."
                        )).color(TEXT_SECONDARY),
                    );
                    ui.add_space(SP_12);
                    ui.horizontal(|ui| {
                        let mut preview = linux_cmd.clone();
                        syntax_highlighted_text_edit(
                            ui,
                            "cred_linux_preview",
                            &mut preview,
                            (ui.available_width() - 90.0).max(220.0),
                            None,
                        );
                        if kit::button(
                            ui,
                            ButtonVariant::Secondary,
                            ButtonSize::Md,
                            "Copy",
                            Some(OutlineIcon::Copy),
                        )
                        .clicked()
                        {
                            ui.ctx().copy_text(linux_cmd);
                            self.status_banner = Some((
                                "Copied Linux secret-tool command to clipboard!".to_string(),
                                true,
                            ));
                            self.status_banner_time = Some(Instant::now());
                        }
                    });
                }
                CredentialOsTab::Mac => {
                    let mac_cmd = format!(
                        "security add-generic-password -U -s \"{target_example}\" -a \"{user_example}\" -w \"YOUR_SSH_PASSWORD\""
                    );
                    ui.label(Typo::SmallMedium.rich("macOS Keychain Access (GUI or Terminal CLI):"));
                    ui.add_space(4.0);
                    ui.label(
                        Typo::Body.rich(format!(
                            "1. GUI: Open Keychain Access -> File -> New Password Item. Set Keychain Item Name = '{target_example}', Account Name = '{user_example}', and enter your SSH password.\n2. CLI: Or run the one-line macOS security command below in Terminal.\n3. In your Project's Settings, set OS Credential Vault Target = '{target_example}'."
                        )).color(TEXT_SECONDARY),
                    );
                    ui.add_space(SP_12);
                    ui.horizontal(|ui| {
                        let mut preview = mac_cmd.clone();
                        syntax_highlighted_text_edit(
                            ui,
                            "cred_mac_preview",
                            &mut preview,
                            (ui.available_width() - 90.0).max(220.0),
                            None,
                        );
                        if kit::button(
                            ui,
                            ButtonVariant::Secondary,
                            ButtonSize::Md,
                            "Copy",
                            Some(OutlineIcon::Copy),
                        )
                        .clicked()
                        {
                            ui.ctx().copy_text(mac_cmd);
                            self.status_banner = Some((
                                "Copied macOS Keychain security command to clipboard!".to_string(),
                                true,
                            ));
                            self.status_banner_time = Some(Instant::now());
                        }
                    });
                }
            });
    }

    pub(crate) fn render_project_workspace(&mut self, ui: &mut egui::Ui, project_id: &str) {
        if self.cached_project_id.as_deref() != Some(project_id) {
            self.refresh_selected_project_metadata(true);
        }

        let Some(proj_idx) = self
            .storage
            .projects
            .iter()
            .position(|p| p.id == project_id)
        else {
            ui.label("Project not found.");
            return;
        };

        let proj_name = self.storage.projects[proj_idx].name.clone();

        // If currently in the loading skeleton transition for switching active project:
        if self.project_switch_loading_id.as_deref() == Some(project_id) {
            if let Some(start) = self.project_switch_loading_start {
                if start.elapsed() < Duration::from_millis(300) {
                    render_project_workspace_skeleton(ui, &proj_name);
                    return;
                }
            }
            self.project_switch_loading_id = None;
            self.project_switch_loading_start = None;
        }

        if self.storage.projects[proj_idx]
            .get_env(&self.selected_env_key)
            .is_none()
        {
            self.selected_env_key = self.storage.projects[proj_idx]
                .environments
                .first()
                .map(|e| e.key.clone())
                .unwrap_or_default();
        }

        let proj_desc = self.storage.projects[proj_idx].description.clone();
        let proj_path = self.storage.projects[proj_idx].local_path.clone();
        let path_exists = PathBuf::from(&proj_path).exists();
        let avail_w = ui.available_width();

        // 1. Top Project Header Bar (Fixed 36px height — all right-side controls vertically centered at 32px height)
        let mut delete_project_clicked = false;
        let mut edit_project_clicked = false;
        let mut duplicate_project_clicked = false;
        let mut selected_branch_changed = None;

        horizontal_row(ui, 36.0, |ui| {
            let title_resp = ui.add(
                egui::Label::new(RichText::new(&proj_name).typo(Typo::H1).color(TEXT_PRIMARY))
                    .sense(Sense::click()),
            );
            title_resp.context_menu(|ui| {
                ui.set_min_width(180.0);
                ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);
                ui.spacing_mut().button_padding = Vec2::new(10.0, 6.0);

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("✎  Edit Project")
                                .typo(Typo::Label)
                                .color(TEXT_PRIMARY),
                        )
                        .fill(CARD_BG),
                    )
                    .clicked()
                {
                    edit_project_clicked = true;
                    ui.close_menu();
                }

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("⧉  Duplicate Project")
                                .typo(Typo::Label)
                                .color(TEXT_PRIMARY),
                        )
                        .fill(CARD_BG),
                    )
                    .clicked()
                {
                    duplicate_project_clicked = true;
                    ui.close_menu();
                }

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("↻  Refresh Metadata")
                                .typo(Typo::Label)
                                .color(TEXT_SECONDARY),
                        )
                        .fill(CARD_BG),
                    )
                    .clicked()
                {
                    self.refresh_selected_project_metadata(true);
                    ui.close_menu();
                }

                ui.separator();

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("🗑  Delete Project")
                                .typo(Typo::Label)
                                .color(FAIL_PILL_TEXT),
                        )
                        .fill(CARD_BG),
                    )
                    .clicked()
                {
                    delete_project_clicked = true;
                    ui.close_menu();
                }
            });

            if path_exists {
                status_badge(ui, StatusKind::Success, "Ready");
            } else {
                status_badge(ui, StatusKind::Danger, "Path missing");
            }

            let right_w = ui.available_width();
            ui.allocate_ui_with_layout(
                Vec2::new(right_w, 36.0),
                Layout::right_to_left(Align::Center),
                |ui| {
                    ui.set_min_height(36.0);
                    ui.set_max_height(36.0);
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

                    // 1. "⋯" Overflow Menu (furthest right)
                    let more_resp = outline_icon_button(
                        ui,
                        ("hdr_more_menu_btn", project_id),
                        OutlineIcon::MoreHorizontal,
                        None,
                        TEXT_MUTED,
                        CARD_BG,
                        SUBTLE_BG,
                        CARD_BORDER,
                        true,
                    )
                    .on_hover_text("More actions");

                    more_resp.context_menu(|ui| {
                        ui.set_min_width(170.0);
                        ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);
                        ui.spacing_mut().button_padding = Vec2::new(10.0, 6.0);

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("⧉  Duplicate project")
                                        .typo(Typo::Label)
                                        .color(TEXT_PRIMARY),
                                )
                                .fill(CARD_BG),
                            )
                            .clicked()
                        {
                            duplicate_project_clicked = true;
                            ui.close_menu();
                        }

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("↻  Refresh metadata")
                                        .typo(Typo::Label)
                                        .color(TEXT_SECONDARY),
                                )
                                .fill(CARD_BG),
                            )
                            .clicked()
                        {
                            self.refresh_selected_project_metadata(true);
                            ui.close_menu();
                        }

                        ui.separator();

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("🗑  Delete project")
                                        .typo(Typo::Label)
                                        .color(DANGER),
                                )
                                .fill(CARD_BG),
                            )
                            .clicked()
                        {
                            delete_project_clicked = true;
                            ui.close_menu();
                        }
                    });

                    // 2. Secondary "Edit" Button
                    if outline_icon_button(
                        ui,
                        ("hdr_edit_proj", project_id),
                        OutlineIcon::Edit,
                        Some("Edit Project"),
                        TEXT_PRIMARY,
                        CARD_BG,
                        SUBTLE_BG,
                        CARD_BORDER,
                        true,
                    )
                    .on_hover_text("Edit project settings")
                    .clicked()
                    {
                        edit_project_clicked = true;
                    }

                    // 3. Git Branch Selector
                    if !self.cached_git_branches.is_empty() {
                        let current_b = if self.selected_git_branch.is_empty() {
                            "main".to_string()
                        } else {
                            self.selected_git_branch.clone()
                        };
                        let commit_suffix = self
                            .cached_git_meta
                            .commit_short
                            .as_deref()
                            .map(|c| format!(" @ {c}"))
                            .unwrap_or_default();

                        ui.scope(|ui| {
                            ui.spacing_mut().interact_size.y = 32.0;
                            ui.spacing_mut().button_padding = Vec2::new(10.0, 6.0);
                            egui::ComboBox::from_id_salt("header_git_branch_selector")
                                .selected_text(
                                    RichText::new(format!("{current_b}{commit_suffix}"))
                                        .typo(Typo::SmallMedium)
                                        .color(TEXT_PRIMARY),
                                )
                                .width(160.0)
                                .show_ui(ui, |ui| {
                                    for branch in self.cached_git_branches.clone() {
                                        let is_sel = self.selected_git_branch == branch;
                                        if ui.selectable_label(is_sel, &branch).clicked() && !is_sel
                                        {
                                            selected_branch_changed = Some(branch);
                                        }
                                    }
                                });
                        });
                    }
                },
            );
        });

        if edit_project_clicked {
            self.open_edit_project_modal(proj_idx);
        }

        if duplicate_project_clicked {
            if let Some(new_proj) = self.storage.duplicate_project(project_id) {
                self.selection = SidebarSelection::Project(new_proj.id.clone());
                self.refresh_selected_project_metadata(true);
                self.status_banner = Some((
                    format!("Duplicated project successfully as '{}'", new_proj.name),
                    true,
                ));
            }
        }

        if delete_project_clicked {
            self.pending_delete_confirm = Some(ConfirmDeleteAction::Project {
                project_id: project_id.to_string(),
                project_name: proj_name.clone(),
            });
        }

        if let Some(new_branch) = selected_branch_changed {
            self.selected_git_branch = new_branch.clone();
            let path = PathBuf::from(&proj_path);
            match checkout_git_branch(&path, &new_branch) {
                Ok(msg) => {
                    self.refresh_selected_project_metadata(false);
                    self.status_banner = Some((msg, true));
                }
                Err(err) => {
                    self.status_banner = Some((
                        format!("Selected branch '{new_branch}' for deploy ({err})"),
                        false,
                    ));
                }
            }
        }

        ui.add_space(8.0);
        draw_divider(ui, CARD_BORDER);
        ui.add_space(6.0);

        // 2. Sub-header description & folder path row
        horizontal_row(ui, 24.0, |ui| {
            if !proj_desc.is_empty() {
                ui.label(
                    RichText::new(&proj_desc)
                        .typo(Typo::Small)
                        .color(TEXT_SECONDARY),
                );
                ui.label(RichText::new("·").typo(Typo::Small).color(TEXT_MUTED));
            }

            let truncated_path = if proj_path.len() > 55 {
                let start = &proj_path[..24];
                let end = &proj_path[proj_path.len() - 26..];
                format!("{start}...{end}")
            } else {
                proj_path.clone()
            };

            let path_resp = ui
                .horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                    draw_icon_folder(
                        ui.painter(),
                        Pos2::new(ui.cursor().min.x + 6.0, ui.cursor().center().y),
                        TEXT_MUTED,
                    );
                    ui.add_space(14.0);
                    ui.label(
                        RichText::new(truncated_path)
                            .typo(Typo::Mono)
                            .color(TEXT_MUTED),
                    )
                })
                .response;

            if path_resp
                .interact(Sense::click())
                .on_hover_text(format!("Click to copy: {proj_path}"))
                .clicked()
            {
                ui.ctx().copy_text(proj_path.clone());
                self.set_status_banner(format!("Copied project path: {proj_path}"), false);
            }
        });

        ui.add_space(14.0);

        // 3. Environment Cards Grid — Responsive columns (3 columns fill workspace width >= 650px, 2 columns >= 440px, 1 column < 440px)
        let env_summaries: Vec<(String, String)> = self.storage.projects[proj_idx]
            .environments
            .iter()
            .map(|e| (e.key.clone(), e.name.clone()))
            .collect();

        let mut new_selected_env_key = None;
        let card_spacing = 14.0;
        let cols = if avail_w >= 650.0 {
            3
        } else if avail_w >= 440.0 {
            2
        } else {
            1
        };
        let card_w = if cols == 1 {
            avail_w
        } else {
            ((avail_w - card_spacing * (cols - 1) as f32) / cols as f32).floor()
        };

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(card_spacing, 14.0);

            for (env_key, env_name) in &env_summaries {
                let is_active = &self.selected_env_key == env_key;
                let last_ok = self.storage.last_succeeded_deployment(project_id, env_key);
                let latest_any = self.storage.latest_deployment(project_id, env_key);

                let (card_rect, card_resp) =
                    ui.allocate_exact_size(Vec2::new(card_w, ENV_CARD_HEIGHT), Sense::click());
                let card_resp = card_resp.on_hover_cursor(egui::CursorIcon::PointingHand);

                let (bg_fill, stroke) = if is_active {
                    (
                        Color32::from_rgb(240, 246, 255),
                        Stroke::new(1.5_f32, PRIMARY_BLUE),
                    )
                } else if card_resp.hovered() {
                    (CARD_BG, Stroke::new(1.0_f32, INPUT_BORDER_HOVER))
                } else {
                    (CARD_BG, Stroke::new(1.0_f32, CARD_BORDER))
                };

                ui.painter()
                    .rect_filled(card_rect, CornerRadius::same(10), bg_fill);
                ui.painter().rect_stroke(
                    card_rect,
                    CornerRadius::same(10),
                    stroke,
                    StrokeKind::Inside,
                );

                // Top row: Environment Name as H3 (sentence case)
                ui.painter().text(
                    Pos2::new(card_rect.left() + 16.0, card_rect.top() + 20.0),
                    Align2::LEFT_CENTER,
                    env_name,
                    Typo::H3.font(),
                    TEXT_PRIMARY,
                );

                // Top-right: Status badge ("Succeeded", "Running", "Failed", "Not deployed")
                let right_badge_pos = Pos2::new(card_rect.right() - 16.0, card_rect.top() + 20.0);
                if let Some(latest) = latest_any {
                    let (s_bg, s_fg) = status_pill_colors(latest.status);
                    let label = latest.status.label();
                    if latest.status == DeploymentStatus::Running {
                        ui.ctx().request_repaint();
                        let time = ui.input(|i| i.time);
                        paint_running_pill_right_aligned(
                            ui.painter(),
                            right_badge_pos,
                            label,
                            s_bg,
                            s_fg,
                            time,
                        );
                    } else {
                        paint_pill_right_aligned(ui.painter(), right_badge_pos, label, s_bg, s_fg);
                    }
                } else {
                    ui.painter().text(
                        right_badge_pos,
                        Align2::RIGHT_CENTER,
                        "Not deployed",
                        Typo::Caption.font(),
                        TEXT_MUTED,
                    );
                }

                // Middle: Version in Typo::MonoLg (20px mono-medium)
                if let Some(ok_dep) = &last_ok {
                    ui.painter().text(
                        Pos2::new(card_rect.left() + 16.0, card_rect.top() + 52.0),
                        Align2::LEFT_CENTER,
                        format!("v{}", ok_dep.version),
                        Typo::MonoLg.font(),
                        TEXT_PRIMARY,
                    );
                    ui.painter().text(
                        Pos2::new(card_rect.left() + 16.0, card_rect.top() + 74.0),
                        Align2::LEFT_CENTER,
                        "Current version",
                        Typo::Small.font(),
                        TEXT_MUTED,
                    );

                    // Bottom: "Deployed 2h ago · main@852f02f"
                    let rel_time = format_relative_time(ok_dep.started_at);
                    let git_str = match (&ok_dep.git_branch, &ok_dep.git_commit) {
                        (Some(b), Some(c)) => format!(" · {b}@{}", &c[..7.min(c.len())]),
                        (Some(b), None) => format!(" · {b}"),
                        _ => String::new(),
                    };
                    ui.painter().text(
                        Pos2::new(card_rect.left() + 16.0, card_rect.top() + 94.0),
                        Align2::LEFT_CENTER,
                        format!("Deployed {rel_time}{git_str}"),
                        Typo::Small.font(),
                        TEXT_MUTED,
                    );
                } else {
                    ui.painter().text(
                        Pos2::new(card_rect.left() + 16.0, card_rect.top() + 56.0),
                        Align2::LEFT_CENTER,
                        "No active build",
                        Typo::H3.font(),
                        TEXT_MUTED,
                    );
                }

                if card_resp.clicked() {
                    new_selected_env_key = Some(env_key.clone());
                }
            }

            // "Add environment" tile: same size (card_w x 112px)
            let (add_rect, add_resp) =
                ui.allocate_exact_size(Vec2::new(card_w, ENV_CARD_HEIGHT), Sense::click());
            let add_resp = add_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            let (add_bg, add_stroke) = if add_resp.hovered() {
                (CARD_BG, Stroke::new(1.0_f32, PRIMARY_BLUE))
            } else {
                (SUBTLE_BG, Stroke::new(1.0_f32, CARD_BORDER))
            };
            ui.painter()
                .rect_filled(add_rect, CornerRadius::same(10), add_bg);
            ui.painter().rect_stroke(
                add_rect,
                CornerRadius::same(10),
                add_stroke,
                StrokeKind::Inside,
            );

            let plus_center = Pos2::new(add_rect.center().x, add_rect.center().y - 18.0);
            draw_icon_plus(
                ui.painter(),
                plus_center,
                if add_resp.hovered() {
                    PRIMARY_BLUE
                } else {
                    TEXT_MUTED
                },
            );
            ui.painter().text(
                Pos2::new(add_rect.center().x, add_rect.center().y + 4.0),
                Align2::CENTER_CENTER,
                "Add environment",
                Typo::Label.font(),
                if add_resp.hovered() {
                    PRIMARY_BLUE
                } else {
                    TEXT_PRIMARY
                },
            );
            ui.painter().text(
                Pos2::new(add_rect.center().x, add_rect.center().y + 24.0),
                Align2::CENTER_CENTER,
                "or import from .ps1",
                Typo::Small.font(),
                TEXT_MUTED,
            );

            if add_resp.clicked() {
                self.new_env_clone_from = self.selected_env_key.clone();
                self.show_new_env_modal = true;
            }
        });

        if let Some(env_key) = new_selected_env_key {
            self.servers_edit_buffer = None;
            self.selected_env_key = env_key;
            self.refresh_selected_project_metadata(true);
        }

        if self.storage.projects[proj_idx].environments.is_empty() {
            ui.add_space(16.0);
            egui::Frame::new()
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0_f32, CARD_BORDER))
                .corner_radius(10.0)
                .inner_margin(Margin::symmetric(24, 24))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("No Environments in This Project Yet")
                                .typo(Typo::H3)
                                .color(TEXT_PRIMARY),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(
                                "Click 'ADD / IMPORT ENVIRONMENT' above to import a deploy-dev.ps1 / deploy-prod.ps1 script or add a custom environment.",
                            )
                            .typo(Typo::Body)
                            .color(TEXT_SECONDARY),
                        );
                    });
                });
            return;
        }

        ui.add_space(16.0);

        // 4. Version Control, Rollback & Deploy Action Card (Fixed 78px height)
        let active_env_name = self.storage.projects[proj_idx]
            .get_env(&self.selected_env_key)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| self.selected_env_key.clone());
        let rollback_candidate = self
            .storage
            .previous_rollback_candidate(project_id, &self.selected_env_key)
            .cloned();
        let mut sync_trigger_rect = None;
        self.render_deploy_action_bar(
            ui,
            &active_env_name,
            rollback_candidate,
            &mut sync_trigger_rect,
        );

        ui.add_space(18.0);

        // 5. Section Tabs: Underline tabs with 1px baseline CARD_BORDER across avail_w
        let total_steps = self.storage.projects[proj_idx]
            .get_env(&self.selected_env_key)
            .map(|e| e.steps.len())
            .unwrap_or(0);
        let history_count = self
            .storage
            .filter_deployments(Some(project_id), Some(&self.selected_env_key), None)
            .len();

        let tab_bar_y = ui.cursor().min.y;
        ui.painter().line_segment(
            [
                Pos2::new(ui.cursor().min.x, tab_bar_y + TAB_HEIGHT - 1.0),
                Pos2::new(ui.cursor().min.x + avail_w, tab_bar_y + TAB_HEIGHT - 1.0),
            ],
            Stroke::new(1.0_f32, CARD_BORDER),
        );

        horizontal_row(ui, TAB_HEIGHT, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(12.0, 0.0);
            for (tab, label, count) in [
                (
                    WorkspaceTab::PipelineSteps,
                    "Pipeline steps",
                    Some(total_steps),
                ),
                (
                    WorkspaceTab::HistoryAndLogs,
                    "Deployments",
                    Some(history_count),
                ),
                (WorkspaceTab::ServerSettings, "Settings", None),
            ] {
                let is_active = self.active_tab == tab;
                let resp = kit::underline_tab(ui, is_active, label, count);
                if resp.clicked() {
                    self.active_tab = tab;
                }
            }
        });

        ui.add_space(14.0);

        // 6. Render Active Full-Width Tab Content
        match self.active_tab {
            WorkspaceTab::PipelineSteps => {
                self.render_pipeline_steps_tab(ui, proj_idx);
            }
            WorkspaceTab::HistoryAndLogs => {
                let pid = self.storage.projects[proj_idx].id.clone();
                self.render_history_and_logs_view(ui, Some(&pid));
            }
            WorkspaceTab::ServerSettings => {
                self.render_server_settings_tab(ui, proj_idx);
            }
        }

        if let Some(r) = sync_trigger_rect {
            self.render_sync_version_dropdown(ui.ctx(), r, project_id);
        }
    }

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

    pub(crate) fn render_deploy_action_bar(
        &mut self,
        ui: &mut egui::Ui,
        env_display_name: &str,
        rollback_candidate: Option<DeploymentRecord>,
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
                .layout_no_wrap(btn_text.clone(), Typo::Label.font(), Color32::WHITE);
        let deploy_w = (deploy_galley.size().x + 14.0 + 8.0 + 32.0).max(160.0);

        let rb_data = rollback_candidate.as_ref().map(|prev_dep| {
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
            if let Some(env_cfg) = project.get_env_mut(&env_key) {
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
                config_changed = true;
            }
        }

        ui.add_space(8.0);

        // 2. Variables row: Muted "Variables:" followed by clickable MONO tags with "Click to copy"
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
                                FAIL_PILL_TEXT,
                                FAIL_PILL_BG,
                                DANGER_SOFT,
                                DANGER_SOFT,
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
                                if can_down {
                                    TEXT_SECONDARY
                                } else {
                                    TEXT_PLACEHOLDER
                                },
                                SUBTLE_BG,
                                INPUT_BG_HOVER,
                                CARD_BORDER,
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
                                if can_up {
                                    TEXT_SECONDARY
                                } else {
                                    TEXT_PLACEHOLDER
                                },
                                SUBTLE_BG,
                                INPUT_BG_HOVER,
                                CARD_BORDER,
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
                                            self.step_scp_sources_buffer =
                                                Some((step.id.clone(), joined));
                                            &mut self.step_scp_sources_buffer.as_mut().unwrap().1
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

    pub(crate) fn render_server_settings_tab(&mut self, ui: &mut egui::Ui, proj_idx: usize) {
        let env_key = self.selected_env_key.clone();
        let mut config_changed = false;
        let mut delete_current_env = false;
        let mut verify_vault_clicked = false;
        let mut open_cred_guide_on_overview = false;

        let current_env_name = self.storage.projects[proj_idx]
            .get_env(&env_key)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| env_key.clone());

        // Header
        ui.horizontal(|ui| {
            ui.label(Typo::H2.rich("Environment settings"));
            ui.add_space(SP_8);
            let env_colors = env_pastel_pill_colors(&current_env_name);
            kit::tag(ui, &current_env_name, env_colors);
            ui.add_space(SP_8);
            if self.cached_vault_found {
                kit::status_badge(ui, StatusKind::Success, "Vault ready");
            } else {
                kit::status_badge(ui, StatusKind::Warning, "Vault missing");
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if kit::button(
                    ui,
                    ButtonVariant::Secondary,
                    ButtonSize::Sm,
                    "Manage secrets",
                    Some(OutlineIcon::Key),
                )
                .on_hover_text("Open Secret Management to configure OS vault credentials")
                .clicked()
                {
                    open_cred_guide_on_overview = true;
                }
            });
        });

        ui.add_space(SP_16);

        let total_avail = ui.available_width();
        let max_content_w = total_avail.min(880.0);
        let pad_x = ((total_avail - max_content_w) / 2.0).max(0.0);

        let mut guide_vault_target = String::new();
        let mut guide_remote_user = String::new();

        ui.horizontal(|ui| {
            if pad_x > 0.0 {
                ui.add_space(pad_x);
            }
            ui.vertical(|ui| {
                ui.set_max_width(max_content_w);
                let content_w = ui.available_width();
                let is_2col = content_w >= 600.0;
                let col_w = if is_2col {
                    (content_w - 40.0 - SP_16) / 2.0
                } else {
                    content_w - 40.0
                };
                let full_w = content_w - 40.0;

                let project = &mut self.storage.projects[proj_idx];

                // Card 1: General
                kit::card(ui, |ui| {
                    ui.label(Typo::H3.rich("General"));
                    ui.add_space(2.0);
                    ui.label(Typo::Small.rich("Environment name and local repository path").color(TEXT_SECONDARY));
                    ui.add_space(SP_12);

                    if let Some(env_cfg) = project.get_env_mut(&env_key) {
                        if is_2col {
                            ui.horizontal(|ui| {
                                kit::form_field(ui, "Environment name", None, true, |ui| {
                                    if styled_text_edit(
                                        ui,
                                        ("srv_env_name", &env_key),
                                        &mut env_cfg.name,
                                        col_w,
                                        Some("e.g. Development, Staging, Production"),
                                    ) {
                                        config_changed = true;
                                    }
                                });
                                ui.add_space(SP_16);
                                kit::form_field(ui, "Application name (PM2 / folder)", None, false, |ui| {
                                    if styled_text_edit(
                                        ui,
                                        ("srv_app_name", &env_key),
                                        &mut env_cfg.app_name,
                                        col_w,
                                        Some("e.g. my-web-service-dev"),
                                    ) {
                                        config_changed = true;
                                    }
                                });
                            });
                        } else {
                            kit::form_field(ui, "Environment name", None, true, |ui| {
                                if styled_text_edit(
                                    ui,
                                    ("srv_env_name", &env_key),
                                    &mut env_cfg.name,
                                    full_w,
                                    Some("e.g. Development, Staging, Production"),
                                ) {
                                    config_changed = true;
                                }
                            });
                            ui.add_space(SP_12);
                            kit::form_field(ui, "Application name (PM2 / folder)", None, false, |ui| {
                                if styled_text_edit(
                                    ui,
                                    ("srv_app_name", &env_key),
                                    &mut env_cfg.app_name,
                                    full_w,
                                    Some("e.g. my-web-service-dev"),
                                ) {
                                    config_changed = true;
                                }
                            });
                        }

                        ui.add_space(SP_12);
                        kit::form_field(ui, "Local project directory", None, true, |ui| {
                            ui.horizontal(|ui| {
                                let path_input_w = (full_w - 96.0).max(120.0);
                                if styled_text_edit(
                                    ui,
                                    "srv_local_path",
                                    &mut project.local_path,
                                    path_input_w,
                                    Some(r"e.g. C:\Projects\my-web-service"),
                                ) {
                                    config_changed = true;
                                }
                                ui.add_space(SP_8);
                                if kit::button(
                                    ui,
                                    ButtonVariant::Secondary,
                                    ButtonSize::Md,
                                    "Browse",
                                    Some(OutlineIcon::Folder),
                                )
                                .on_hover_text("Browse local folder")
                                .clicked()
                                {
                                    if let Some(folder) = helpers::pick_folder_dialog() {
                                        project.local_path = folder;
                                        config_changed = true;
                                    }
                                }
                            });
                        });
                    }
                });

                ui.add_space(SP_16);

                // Card 2: Servers
                kit::card(ui, |ui| {
                    ui.label(Typo::H3.rich("Servers"));
                    ui.add_space(2.0);
                    ui.label(Typo::Small.rich("Target destination and SSH access").color(TEXT_SECONDARY));
                    ui.add_space(SP_12);

                    if let Some(env_cfg) = project.get_env_mut(&env_key) {
                        let servers_text = match &mut self.servers_edit_buffer {
                            Some((k, buf)) if k == &env_key => buf,
                            _ => {
                                self.servers_edit_buffer =
                                    Some((env_key.clone(), env_cfg.servers.join(", ")));
                                &mut self.servers_edit_buffer.as_mut().unwrap().1
                            }
                        };

                        kit::form_field(
                            ui,
                            "Target servers",
                            Some("Comma-separated hosts or IPs"),
                            true,
                            |ui| {
                                if styled_text_edit(
                                    ui,
                                    ("srv_servers", &env_key),
                                    servers_text,
                                    full_w,
                                    Some("e.g. 192.168.1.10, 192.168.1.11"),
                                ) {
                                    env_cfg.servers = servers_text
                                        .split(',')
                                        .map(|s| s.trim().to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                    config_changed = true;
                                }
                            },
                        );

                        ui.add_space(SP_12);

                        if is_2col {
                            ui.horizontal(|ui| {
                                kit::form_field(ui, "Remote directory", None, true, |ui| {
                                    if styled_text_edit(
                                        ui,
                                        ("srv_remote_dir", &env_key),
                                        &mut env_cfg.remote_dir,
                                        col_w,
                                        Some("e.g. /var/www/my-web-service"),
                                    ) {
                                        config_changed = true;
                                    }
                                });
                                ui.add_space(SP_16);
                                kit::form_field(ui, "SSH username", None, true, |ui| {
                                    if styled_text_edit(
                                        ui,
                                        ("srv_remote_user", &env_key),
                                        &mut env_cfg.remote_user,
                                        col_w,
                                        Some("e.g. deploy or ubuntu"),
                                    ) {
                                        config_changed = true;
                                    }
                                });
                            });
                        } else {
                            kit::form_field(ui, "Remote directory", None, true, |ui| {
                                if styled_text_edit(
                                    ui,
                                    ("srv_remote_dir", &env_key),
                                    &mut env_cfg.remote_dir,
                                    full_w,
                                    Some("e.g. /var/www/my-web-service"),
                                ) {
                                    config_changed = true;
                                }
                            });
                            ui.add_space(SP_12);
                            kit::form_field(ui, "SSH username", None, true, |ui| {
                                if styled_text_edit(
                                    ui,
                                    ("srv_remote_user", &env_key),
                                    &mut env_cfg.remote_user,
                                    full_w,
                                    Some("e.g. deploy or ubuntu"),
                                ) {
                                    config_changed = true;
                                }
                            });
                        }
                    }
                });

                ui.add_space(SP_16);

                // Card 3: Authentication
                kit::card(ui, |ui| {
                    ui.label(Typo::H3.rich("Authentication"));
                    ui.add_space(2.0);
                    ui.label(Typo::Small.rich("OS Credential Vault integration and passwords").color(TEXT_SECONDARY));
                    ui.add_space(SP_12);

                    if let Some(env_cfg) = project.get_env_mut(&env_key) {
                        guide_vault_target = env_cfg.vault_target.trim().to_string();
                        guide_remote_user = env_cfg.remote_user.trim().to_string();

                        kit::form_field(
                            ui,
                            "Vault target",
                            Some("Key in OS Credential Manager (stored encrypted)"),
                            false,
                            |ui| {
                                ui.horizontal(|ui| {
                                    let btn_reserve = 190.0;
                                    let vault_input_w = (full_w - btn_reserve).max(120.0);
                                    if styled_text_edit(
                                        ui,
                                        ("srv_vault_target", &env_key),
                                        &mut env_cfg.vault_target,
                                        vault_input_w,
                                        Some("e.g. prod-ssh-server or my-vault-target"),
                                    ) {
                                        self.cached_vault_found =
                                            read_os_vault_credential(&env_cfg.vault_target).is_some();
                                        config_changed = true;
                                    }
                                    ui.add_space(SP_8);
                                    if kit::button(
                                        ui,
                                        ButtonVariant::Secondary,
                                        ButtonSize::Md,
                                        "Verify",
                                        Some(OutlineIcon::Refresh),
                                    )
                                    .on_hover_text("Check if this target exists in your OS Credential Manager / Keychain")
                                    .clicked()
                                    {
                                        verify_vault_clicked = true;
                                    }
                                    ui.add_space(SP_8);
                                    if kit::button(
                                        ui,
                                        ButtonVariant::Secondary,
                                        ButtonSize::Md,
                                        "Set secret",
                                        Some(OutlineIcon::Key),
                                    )
                                    .on_hover_text("Configure username & password for this target in Secret Management")
                                    .clicked()
                                    {
                                        open_cred_guide_on_overview = true;
                                    }
                                });
                            },
                        );

                        ui.add_space(SP_12);

                        kit::form_field(
                            ui,
                            "Password override",
                            Some("Leave blank to use the OS vault or an SSH key"),
                            false,
                            |ui| {
                                styled_password_edit(
                                    ui,
                                    ("srv_manual_pwd", &env_key),
                                    &mut self.manual_ssh_password,
                                    full_w,
                                    Some("Optional — leave blank to use OS Credential Vault or SSH Key"),
                                );
                            },
                        );
                    }
                });

                ui.add_space(SP_16);

                // Card 4: Runtime
                kit::card(ui, |ui| {
                    ui.label(Typo::H3.rich("Runtime"));
                    ui.add_space(2.0);
                    ui.label(Typo::Small.rich("Environment variables and network proxy").color(TEXT_SECONDARY));
                    ui.add_space(SP_12);

                    if let Some(env_cfg) = project.get_env_mut(&env_key) {
                        let env_src_text = match &mut self.server_env_source_buffer {
                            Some((k, buf)) if k == &env_key => buf,
                            _ => {
                                self.server_env_source_buffer =
                                    Some((env_key.clone(), env_cfg.env_source.clone().unwrap_or_default()));
                                &mut self.server_env_source_buffer.as_mut().unwrap().1
                            }
                        };

                        if is_2col {
                            ui.horizontal(|ui| {
                                kit::form_field(ui, "Env file (.env source)", None, false, |ui| {
                                    if styled_text_edit(
                                        ui,
                                        ("srv_env_source", &env_key),
                                        env_src_text,
                                        col_w,
                                        Some("Optional — e.g. ./.env.dev or ./.env.production"),
                                    ) {
                                        env_cfg.env_source = if env_src_text.trim().is_empty() {
                                            None
                                        } else {
                                            Some(env_src_text.clone())
                                        };
                                        config_changed = true;
                                    }
                                });
                                ui.add_space(SP_16);
                                let proxy_text = match &mut self.server_http_proxy_buffer {
                                    Some((k, buf)) if k == &env_key => buf,
                                    _ => {
                                        self.server_http_proxy_buffer =
                                            Some((env_key.clone(), env_cfg.http_proxy.clone().unwrap_or_default()));
                                        &mut self.server_http_proxy_buffer.as_mut().unwrap().1
                                    }
                                };
                                kit::form_field(ui, "HTTP proxy", None, false, |ui| {
                                    if styled_text_edit(
                                        ui,
                                        ("srv_http_proxy", &env_key),
                                        proxy_text,
                                        col_w,
                                        Some("Optional — e.g. http://proxy.example.com:8080"),
                                    ) {
                                        env_cfg.http_proxy = if proxy_text.trim().is_empty() {
                                            None
                                        } else {
                                            Some(proxy_text.clone())
                                        };
                                        config_changed = true;
                                    }
                                });
                            });
                        } else {
                            kit::form_field(ui, "Env file (.env source)", None, false, |ui| {
                                if styled_text_edit(
                                    ui,
                                    ("srv_env_source", &env_key),
                                    env_src_text,
                                    full_w,
                                    Some("Optional — e.g. ./.env.dev or ./.env.production"),
                                ) {
                                    env_cfg.env_source = if env_src_text.trim().is_empty() {
                                        None
                                    } else {
                                        Some(env_src_text.clone())
                                    };
                                    config_changed = true;
                                }
                            });
                            ui.add_space(SP_12);
                            let proxy_text = match &mut self.server_http_proxy_buffer {
                                Some((k, buf)) if k == &env_key => buf,
                                _ => {
                                    self.server_http_proxy_buffer =
                                        Some((env_key.clone(), env_cfg.http_proxy.clone().unwrap_or_default()));
                                    &mut self.server_http_proxy_buffer.as_mut().unwrap().1
                                }
                            };
                            kit::form_field(ui, "HTTP proxy", None, false, |ui| {
                                if styled_text_edit(
                                    ui,
                                    ("srv_http_proxy", &env_key),
                                    proxy_text,
                                    full_w,
                                    Some("Optional — e.g. http://proxy.example.com:8080"),
                                ) {
                                    env_cfg.http_proxy = if proxy_text.trim().is_empty() {
                                        None
                                    } else {
                                        Some(proxy_text.clone())
                                    };
                                    config_changed = true;
                                }
                            });
                        }
                    }
                });

                ui.add_space(SP_16);

                // Danger zone card
                egui::Frame::new()
                    .fill(CARD_BG)
                    .stroke(Stroke::new(1.0_f32, DANGER_BORDER))
                    .corner_radius(CornerRadius::same(R_LG as u8))
                    .shadow(Shadow {
                        offset: [0, 1],
                        blur: 2,
                        spread: 0,
                        color: SHADOW_CARD,
                    })
                    .inner_margin(Margin::same(20))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(Typo::H3.rich("Delete environment").color(DANGER));
                                ui.add_space(4.0);
                                ui.label(
                                    Typo::Small
                                        .rich("Removes this environment and its pipeline. Deployment history is kept.")
                                        .color(TEXT_MUTED),
                                );
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if kit::button(
                                    ui,
                                    ButtonVariant::Danger,
                                    ButtonSize::Sm,
                                    "Delete environment",
                                    Some(OutlineIcon::Trash),
                                )
                                .on_hover_text("Delete this environment from this project")
                                .clicked()
                                {
                                    delete_current_env = true;
                                }
                            });
                        });
                    });

                ui.add_space(SP_24);
            });
        });

        // Handlers
        if open_cred_guide_on_overview {
            let desc = format!(
                "SSH Vault target for {} ({})",
                self.storage.projects[proj_idx].name, current_env_name
            );
            self.open_secret_management_for_target(&guide_vault_target, &guide_remote_user, &desc);
        } else if verify_vault_clicked {
            let _ = self.storage.save();
            self.refresh_selected_project_metadata(false);
            if self.cached_vault_found {
                self.status_banner = Some((
                    format!(
                        "Verified! Found credential for target '{guide_vault_target}' in OS Vault."
                    ),
                    true,
                ));
            } else {
                self.status_banner = Some((
                    format!(
                        "No credential found for target '{guide_vault_target}' in OS Vault. Click 'Set Secret' or 'Secret Management' in the sidebar to save it."
                    ),
                    false,
                ));
            }
        } else if delete_current_env {
            self.servers_edit_buffer = None;
            self.pending_delete_confirm = Some(ConfirmDeleteAction::Environment {
                project_id: self.storage.projects[proj_idx].id.clone(),
                project_name: self.storage.projects[proj_idx].name.clone(),
                env_key: env_key.clone(),
                env_name: current_env_name.clone(),
            });
        } else if config_changed {
            self.mark_config_dirty();
        }
    }

    pub(crate) fn render_history_and_logs_view(
        &mut self,
        ui: &mut egui::Ui,
        project_filter: Option<&str>,
    ) {
        let is_project_view = project_filter.is_some();
        let project_envs: Vec<(String, String)> = if let Some(pid) = project_filter {
            self.storage
                .projects
                .iter()
                .find(|p| p.id == pid)
                .map(|p| {
                    p.environments
                        .iter()
                        .map(|e| (e.name.clone(), e.key.clone()))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let filter_options = if !is_project_view {
            let opts = self.storage.existing_environment_tabs(None);
            if self.history_env_filter.is_some()
                && !opts.iter().any(|(_, k)| *k == self.history_env_filter)
            {
                self.history_env_filter = None;
            }
            opts
        } else {
            Vec::new()
        };

        let mut switched_env_key = None;
        let mut switched_history_filter = None;

        let effective_env_filter_preview = if is_project_view {
            if self.selected_env_key.is_empty() {
                None
            } else {
                Some(self.selected_env_key.clone())
            }
        } else {
            self.history_env_filter.clone()
        };
        let deletable_count = self
            .storage
            .filter_deployments(
                project_filter,
                effective_env_filter_preview.as_deref(),
                None,
            )
            .into_iter()
            .filter(|d| d.status != DeploymentStatus::Running)
            .count();

        // Section Header + Filter Pills + Clear History Button (Fixed 34px height)
        let mut trigger_clear_history = false;
        horizontal_row(ui, 36.0, |ui| {
            if is_project_view {
                let mut selected_idx = project_envs
                    .iter()
                    .position(|(_, k)| k == &self.selected_env_key)
                    .unwrap_or(0);
                let env_names: Vec<&str> =
                    project_envs.iter().map(|(n, _)| n.as_str()).collect();
                if !env_names.is_empty() && kit::segmented(ui, &mut selected_idx, &env_names) {
                    if let Some((_, k)) = project_envs.get(selected_idx) {
                        switched_env_key = Some(k.clone());
                    }
                }
            } else {
                let mut selected_idx = if let Some(ref cur) = self.history_env_filter {
                    filter_options
                        .iter()
                        .position(|(_, k)| k.as_ref() == Some(cur))
                        .unwrap_or(0)
                } else {
                    0
                };
                let filter_labels: Vec<&str> =
                    filter_options.iter().map(|(lbl, _)| lbl.as_str()).collect();
                if !filter_labels.is_empty()
                    && kit::segmented(ui, &mut selected_idx, &filter_labels)
                {
                    if let Some((_, opt_val)) = filter_options.get(selected_idx) {
                        switched_history_filter = Some(opt_val.clone());
                    }
                }
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

                // ⋯ Overflow menu button containing "Clear history…"
                let more_id = ui.make_persistent_id("history_overflow_menu");
                let more_resp = outline_icon_button(
                    ui,
                    ("history_more_btn", project_filter.unwrap_or("all")),
                    OutlineIcon::MoreHorizontal,
                    None,
                    TEXT_PRIMARY,
                    CARD_BG,
                    SUBTLE_BG,
                    CARD_BORDER,
                    true,
                )
                .on_hover_text("More actions");

                if more_resp.clicked() {
                    ui.memory_mut(|m| m.toggle_popup(more_id));
                }

                egui::popup_below_widget(
                    ui,
                    more_id,
                    &more_resp,
                    egui::PopupCloseBehavior::CloseOnClick,
                    |ui| {
                        ui.set_min_width(140.0);
                        let can_clear = deletable_count > 0;
                        let clr_btn = egui::Button::new(
                            RichText::new("Clear history…")
                                .typo(Typo::SmallMedium)
                                .color(if can_clear {
                                    FAIL_PILL_TEXT
                                } else {
                                    TEXT_PLACEHOLDER
                                }),
                        )
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE);
                        if ui.add_enabled(can_clear, clr_btn).clicked() {
                            trigger_clear_history = true;
                            ui.memory_mut(|m| m.close_popup());
                        }
                    },
                );

                // Search/filter input
                styled_text_edit(
                    ui,
                    "history_search_input",
                    &mut self.history_search_query,
                    160.0,
                    Some("Filter deployments..."),
                );
            });
        });

        if trigger_clear_history && deletable_count > 0 {
            let proj_name = project_filter.and_then(|pid| {
                self.storage
                    .projects
                    .iter()
                    .find(|p| p.id == pid)
                    .map(|p| p.name.clone())
            });
            let env_name = if is_project_view {
                project_envs
                    .iter()
                    .find(|(_, k)| k == &self.selected_env_key)
                    .map(|(n, _)| n.clone())
                    .or_else(|| {
                        if self.selected_env_key.is_empty() {
                            None
                        } else {
                            Some(self.selected_env_key.clone())
                        }
                    })
            } else {
                self.history_env_filter.clone()
            };
            self.pending_delete_confirm = Some(ConfirmDeleteAction::ClearDeploymentHistory {
                project_id: project_filter.map(str::to_string),
                project_name: proj_name,
                env_key: effective_env_filter_preview,
                env_name,
                count: deletable_count,
            });
        }

        if let Some(env_key) = switched_env_key {
            self.selected_env_key = env_key;
            self.history_visible_limit = 5;
            self.refresh_selected_project_metadata(true);
        } else if let Some(filter_val) = switched_history_filter {
            self.history_env_filter = filter_val;
            self.history_visible_limit = 5;
        }

        ui.add_space(10.0);

        // When viewing a project workspace, deployment history strictly filters by the active environment
        let effective_env_filter = if is_project_view {
            if self.selected_env_key.is_empty() {
                None
            } else {
                Some(self.selected_env_key.as_str())
            }
        } else {
            self.history_env_filter.as_deref()
        };

        let query = self.history_search_query.trim().to_lowercase();
        let filtered: Vec<DeploymentRecord> = self
            .storage
            .filter_deployments(project_filter, effective_env_filter, None)
            .into_iter()
            .filter(|d| {
                if query.is_empty() {
                    true
                } else {
                    d.version.to_lowercase().contains(&query)
                        || d.project_name.to_lowercase().contains(&query)
                        || d.environment.to_lowercase().contains(&query)
                        || d.git_branch.as_deref().unwrap_or("").to_lowercase().contains(&query)
                        || d.git_commit.as_deref().unwrap_or("").to_lowercase().contains(&query)
                }
            })
            .cloned()
            .collect();

        let mut redeploy_target: Option<(DeploymentRecord, bool)> = None;
        let mut cancel_target: Option<String> = None;
        let mut delete_target: Option<DeploymentRecord> = None;

        let is_stacked_view =
            ui.ctx().screen_rect().width() < 720.0 || ui.available_width() < 560.0;
        let total_count = filtered.len();
        let visible_count = total_count.min(self.history_visible_limit.max(5));

        // Structured Deployment History Table Card
        egui::Frame::new()
            .fill(CARD_BG)
            .stroke(Stroke::new(1.0_f32, CARD_BORDER))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(18, 14))
            .show(ui, |ui| {
                // Fixed column widths: Status: 90, Version: 80, Branch: 120, Commit: 70, Deployed: 140, Trigger: 110, Actions: remaining
                let col_status = 90.0;
                let col_version = 80.0;
                let col_branch = 120.0;
                let col_commit = 70.0;
                let col_deployed = 140.0;
                let col_trigger = 110.0;

                if !is_stacked_view {
                    horizontal_row(ui, 24.0, |ui| {
                        ui.add_sized(
                            [col_status, 20.0],
                            egui::Label::new(
                                RichText::new("Status")
                                    .typo(Typo::Caption)
                                    .color(TEXT_MUTED),
                            ),
                        );
                        ui.add_sized(
                            [col_version, 20.0],
                            egui::Label::new(
                                RichText::new("Version")
                                    .typo(Typo::Caption)
                                    .color(TEXT_MUTED),
                            ),
                        );
                        ui.add_sized(
                            [col_branch, 20.0],
                            egui::Label::new(
                                RichText::new("Branch")
                                    .typo(Typo::Caption)
                                    .color(TEXT_MUTED),
                            ),
                        );
                        ui.add_sized(
                            [col_commit, 20.0],
                            egui::Label::new(
                                RichText::new("Commit")
                                    .typo(Typo::Caption)
                                    .color(TEXT_MUTED),
                            ),
                        );
                        ui.add_sized(
                            [col_deployed, 20.0],
                            egui::Label::new(
                                RichText::new("Deployed")
                                    .typo(Typo::Caption)
                                    .color(TEXT_MUTED),
                            ),
                        );
                        ui.add_sized(
                            [col_trigger, 20.0],
                            egui::Label::new(
                                RichText::new("Trigger / User")
                                    .typo(Typo::Caption)
                                    .color(TEXT_MUTED),
                            ),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new("Actions")
                                    .typo(Typo::Caption)
                                    .color(TEXT_MUTED),
                            );
                        });
                    });

                    ui.add_space(6.0);
                    draw_divider(ui, CARD_BORDER);
                }

                if filtered.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        let msg = if is_project_view {
                            format!(
                                "No deployments recorded for '{}' yet.",
                                self.selected_env_key
                            )
                        } else {
                            "No deployments recorded yet.".to_string()
                        };
                        ui.label(RichText::new(msg).typo(Typo::Body).color(TEXT_MUTED));
                        ui.add_space(20.0);
                    });
                }

                if is_stacked_view {
                    // Mobile / narrow stacked card view (< 720px)
                    for (idx, dep) in filtered.iter().take(visible_count).enumerate() {
                        if idx > 0 {
                            draw_divider(ui, SUBTLE_BG);
                        }
                        ui.add_space(6.0);
                        let is_selected = self.selected_deployment_id.as_deref() == Some(&dep.id);

                        // Line 1: Status + Env + Version (left), Deployed relative time (right)
                        horizontal_row(ui, 24.0, |ui| {
                            let (s_rect, _) =
                                ui.allocate_exact_size(Vec2::new(96.0, 22.0), Sense::hover());
                            let (s_bg, s_fg) = status_pill_colors(dep.status);
                            if dep.status == DeploymentStatus::Running {
                                ui.ctx().request_repaint();
                                let time = ui.input(|i| i.time);
                                paint_running_pill_left_aligned(
                                    ui.painter(),
                                    Pos2::new(s_rect.left(), s_rect.center().y),
                                    dep.status.label(),
                                    s_bg,
                                    s_fg,
                                    time,
                                );
                            } else {
                                paint_pill_left_aligned(
                                    ui.painter(),
                                    Pos2::new(s_rect.left(), s_rect.center().y),
                                    dep.status.label(),
                                    s_bg,
                                    s_fg,
                                );
                            }

                            let (_, env_fg) = env_pastel_pill_colors(&dep.environment);
                            ui.label(
                                RichText::new(format_env_badge(&dep.environment))
                                    .typo(Typo::SmallMedium)
                                    .color(env_fg),
                            );

                            ui.label(
                                RichText::new(format!("v{}", dep.version))
                                    .typo(Typo::Mono)
                                    .color(TEXT_PRIMARY),
                            );

                            if dep.is_rollback {
                                ui.label(
                                    RichText::new("Rollback")
                                        .typo(Typo::Caption)
                                        .color(WARN_PILL_TEXT),
                                );
                            }

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let rel = helpers::format_relative_time(dep.started_at);
                                let ts_resp = ui.label(
                                    RichText::new(rel)
                                        .typo(Typo::Small)
                                        .color(TEXT_MUTED),
                                );
                                let mut hover_msg = format!(
                                    "Started (Local): {}",
                                    dep.format_started_at_local("%Y-%m-%d %H:%M:%S %:z")
                                );
                                if let Some(fin) =
                                    dep.format_finished_at_local("%Y-%m-%d %H:%M:%S %:z")
                                {
                                    hover_msg.push_str(&format!("\nFinished (Local): {fin}"));
                                }
                                ts_resp.on_hover_text(hover_msg);
                            });
                        });

                        ui.add_space(4.0);

                        // Line 2: Git branch @ commit
                        horizontal_row(ui, 20.0, |ui| {
                            let (icon_r, _) =
                                ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
                            draw_icon_git_branch(
                                ui.painter(),
                                Pos2::new(icon_r.left(), icon_r.top() + 3.0),
                                TEXT_MUTED,
                            );
                            let git_line = match (&dep.git_branch, &dep.git_commit) {
                                (Some(b), Some(c)) => {
                                    let short = if c.len() > 7 { &c[..7] } else { c };
                                    format!("{b} @ {short}")
                                }
                                _ => dep.project_name.clone(),
                            };
                            ui.label(
                                RichText::new(git_line)
                                    .typo(Typo::MonoSm)
                                    .color(TEXT_SECONDARY),
                            );
                        });

                        ui.add_space(6.0);

                        // Line 3: Duration (left), Action buttons (right)
                        horizontal_row(ui, 28.0, |ui| {
                            let dur_str = dep
                                .duration_ms
                                .map(|ms| format!("took {:.1}s", ms as f64 / 1000.0))
                                .unwrap_or_else(|| "running...".to_string());
                            ui.label(RichText::new(dur_str).typo(Typo::Small).color(TEXT_MUTED));

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
                                let is_busy =
                                    self.is_env_deploying(&dep.project_id, &dep.environment);
                                let can_delete = dep.status != DeploymentStatus::Running;

                                if outline_icon_button(
                                    ui,
                                    ("stacked_del_dep_btn", &dep.id),
                                    OutlineIcon::Trash,
                                    None,
                                    FAIL_PILL_TEXT,
                                    SUBTLE_BG,
                                    FAIL_PILL_BG,
                                    CARD_BORDER,
                                    can_delete,
                                )
                                .on_hover_text("Delete this deployment record")
                                .clicked()
                                    && can_delete
                                {
                                    delete_target = Some(dep.clone());
                                }

                                if outline_icon_button(
                                    ui,
                                    ("stacked_rerun_btn", &dep.id),
                                    OutlineIcon::Refresh,
                                    None,
                                    TEXT_SECONDARY,
                                    SUBTLE_BG,
                                    PILL_CONTAINER_BG,
                                    CARD_BORDER,
                                    !is_busy,
                                )
                                .on_hover_text("Re-run this deployment")
                                .clicked()
                                    && !is_busy
                                {
                                    redeploy_target = Some((dep.clone(), false));
                                }

                                if dep.status == DeploymentStatus::Running
                                    && outline_icon_button(
                                        ui,
                                        ("stacked_cancel_btn", &dep.id),
                                        OutlineIcon::X,
                                        Some("Cancel"),
                                        FAIL_PILL_TEXT,
                                        FAIL_PILL_BG,
                                        DANGER_SOFT,
                                        DANGER_SOFT,
                                        true,
                                    )
                                    .on_hover_text("Cancel this running deployment")
                                    .clicked()
                                {
                                    cancel_target = Some(dep.id.clone());
                                }

                                if dep.status == DeploymentStatus::Succeeded
                                    && outline_icon_button(
                                        ui,
                                        ("stacked_rollback_btn", &dep.id),
                                        OutlineIcon::RotateCcw,
                                        Some("Rollback"),
                                        WARN_PILL_TEXT,
                                        WARN_PILL_BG,
                                        WARNING_SOFT,
                                        WARNING,
                                        !is_busy,
                                    )
                                    .on_hover_text(format!(
                                        "Rollback {} to v{}",
                                        dep.environment, dep.version
                                    ))
                                    .clicked()
                                    && !is_busy
                                {
                                    redeploy_target = Some((dep.clone(), true));
                                }

                                let logs_btn = egui::Button::new(
                                    RichText::new(if is_selected {
                                        "Viewing log"
                                    } else {
                                        "View log"
                                    })
                                    .typo(Typo::SmallMedium)
                                    .color(if is_selected { PRIMARY_BLUE } else { TEXT_PRIMARY }),
                                )
                                .fill(if is_selected {
                                    BLUE_PILL_BG
                                } else {
                                    SUBTLE_BG
                                })
                                .stroke(Stroke::new(1.0_f32, if is_selected { PRIMARY_BLUE } else { CARD_BORDER }))
                                .corner_radius(CornerRadius::same(6));

                                if ui.add(logs_btn).clicked() {
                                    self.selected_deployment_id = Some(dep.id.clone());
                                    self.console_collapsed = false;
                                }
                            });
                        });
                        ui.add_space(6.0);
                    }
                } else {
                    // Desktop table: Status: 90, Version: 80, Branch: 120, Commit: 70, Deployed: 140, Trigger: 110, Actions: remaining
                    for (idx, dep) in filtered.iter().take(visible_count).enumerate() {
                        if idx > 0 {
                            draw_divider(ui, SUBTLE_BG);
                        }
                        ui.add_space(4.0);
                        let is_selected = self.selected_deployment_id.as_deref() == Some(&dep.id);
                        let row_h = 42.0;

                        horizontal_row(ui, row_h, |ui| {
                            // 1. Status (90px)
                            let (s_rect, _) =
                                ui.allocate_exact_size(Vec2::new(col_status, row_h), Sense::hover());
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
                                paint_pill_left_aligned(
                                    ui.painter(),
                                    s_pos,
                                    dep.status.label(),
                                    s_bg,
                                    s_fg,
                                );
                            }

                            // 2. Version (80px, MONO)
                            let (v_rect, _) =
                                ui.allocate_exact_size(Vec2::new(col_version, row_h), Sense::hover());
                            ui.painter().text(
                                Pos2::new(v_rect.left(), v_rect.center().y),
                                Align2::LEFT_CENTER,
                                format!("v{}", dep.version),
                                Typo::Mono.font(),
                                TEXT_PRIMARY,
                            );

                            // 3. Branch (120px, MONO, truncated with ellipsis)
                            let (b_rect, b_resp) =
                                ui.allocate_exact_size(Vec2::new(col_branch, row_h), Sense::hover());
                            draw_icon_git_branch(
                                ui.painter(),
                                Pos2::new(b_rect.left() + 2.0, b_rect.center().y),
                                TEXT_MUTED,
                            );
                            let raw_branch = dep.git_branch.as_deref().unwrap_or("main");
                            let max_b_chars = 11;
                            let b_text = if raw_branch.chars().count() > max_b_chars {
                                let truncated: String =
                                    raw_branch.chars().take(max_b_chars.saturating_sub(1)).collect();
                                format!("{truncated}…")
                            } else {
                                raw_branch.to_string()
                            };
                            ui.painter().with_clip_rect(b_rect).text(
                                Pos2::new(b_rect.left() + 16.0, b_rect.center().y),
                                Align2::LEFT_CENTER,
                                b_text,
                                Typo::MonoSm.font(),
                                TEXT_SECONDARY,
                            );
                            b_resp.on_hover_text(format!("Branch: {raw_branch}"));

                            // 4. Commit (70px, MONO)
                            let (c_rect, c_resp) =
                                ui.allocate_exact_size(Vec2::new(col_commit, row_h), Sense::hover());
                            let full_sha = dep.git_commit.as_deref().unwrap_or("-");
                            let short_sha =
                                if full_sha.len() > 7 { &full_sha[..7] } else { full_sha };
                            ui.painter().text(
                                Pos2::new(c_rect.left(), c_rect.center().y),
                                Align2::LEFT_CENTER,
                                short_sha,
                                Typo::MonoSm.font(),
                                TEXT_MUTED,
                            );
                            if full_sha != "-" {
                                c_resp.on_hover_text(format!("Commit: {full_sha}"));
                            }

                            // 5. Deployed (140px, relative time + tooltip with full timestamp)
                            let (t_rect, t_resp) =
                                ui.allocate_exact_size(Vec2::new(col_deployed, row_h), Sense::hover());
                            let rel_time = helpers::format_relative_time(dep.started_at);
                            ui.painter().text(
                                Pos2::new(t_rect.left(), t_rect.center().y),
                                Align2::LEFT_CENTER,
                                rel_time,
                                Typo::Small.font(),
                                TEXT_SECONDARY,
                            );
                            let mut hover_msg = format!(
                                "Started: {}",
                                dep.format_started_at_local("%Y-%m-%d %H:%M:%S %:z")
                            );
                            if let Some(fin) =
                                dep.format_finished_at_local("%Y-%m-%d %H:%M:%S %:z")
                            {
                                hover_msg.push_str(&format!("\nFinished: {fin}"));
                            }
                            if let Some(ms) = dep.duration_ms {
                                hover_msg.push_str(&format!("\nDuration: {:.1}s", ms as f64 / 1000.0));
                            }
                            t_resp.on_hover_text(hover_msg);

                            // 6. Trigger / User (110px)
                            let (u_rect, _) =
                                ui.allocate_exact_size(Vec2::new(col_trigger, row_h), Sense::hover());
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

                            // 7. Actions (remaining width, right-aligned)
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
                                let is_busy =
                                    self.is_env_deploying(&dep.project_id, &dep.environment);
                                let can_delete = dep.status != DeploymentStatus::Running;

                                // Row overflow menu "⋯"
                                let row_more_id = ui.make_persistent_id(("row_more_menu", &dep.id));
                                let row_more_resp = outline_icon_button(
                                    ui,
                                    ("row_more_btn", &dep.id),
                                    OutlineIcon::MoreHorizontal,
                                    None,
                                    TEXT_PRIMARY,
                                    CARD_BG,
                                    SUBTLE_BG,
                                    CARD_BORDER,
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
                                            delete_target = Some(dep.clone());
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
                                        WARN_PILL_TEXT,
                                        WARN_PILL_BG,
                                        WARNING_SOFT,
                                        WARNING,
                                        !is_busy,
                                    )
                                    .on_hover_text(format!(
                                        "Rollback {} to v{}",
                                        dep.environment, dep.version
                                    ))
                                    .clicked()
                                        && !is_busy
                                    {
                                        redeploy_target = Some((dep.clone(), true));
                                    }
                                } else {
                                    if outline_icon_button(
                                        ui,
                                        ("hist_rerun_btn", &dep.id),
                                        OutlineIcon::Refresh,
                                        None,
                                        TEXT_SECONDARY,
                                        SUBTLE_BG,
                                        PILL_CONTAINER_BG,
                                        CARD_BORDER,
                                        !is_busy,
                                    )
                                    .on_hover_text("Re-run this deployment")
                                    .clicked()
                                        && !is_busy
                                    {
                                        redeploy_target = Some((dep.clone(), false));
                                    }
                                }

                                if dep.status == DeploymentStatus::Running
                                    && outline_icon_button(
                                        ui,
                                        ("hist_cancel_btn", &dep.id),
                                        OutlineIcon::X,
                                        Some("Cancel"),
                                        FAIL_PILL_TEXT,
                                        FAIL_PILL_BG,
                                        DANGER_SOFT,
                                        DANGER_SOFT,
                                        true,
                                    )
                                    .on_hover_text("Cancel this running deployment")
                                    .clicked()
                                {
                                    cancel_target = Some(dep.id.clone());
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
                                .fill(if is_selected {
                                    BLUE_PILL_BG
                                } else {
                                    SUBTLE_BG
                                })
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

                if total_count > visible_count {
                    if visible_count > 0 {
                        draw_divider(ui, SUBTLE_BG);
                    }
                    ui.add_space(10.0);
                    ui.vertical_centered(|ui| {
                        let remaining = total_count - visible_count;
                        let next_chunk = remaining.min(10);
                        let btn_text =
                            format!("Show More (+{next_chunk} of {remaining} remaining)...");
                        let show_more_btn = egui::Button::new(
                            RichText::new(btn_text)
                                .typo(Typo::SmallMedium)
                                .color(PRIMARY_BLUE),
                        )
                        .fill(SUBTLE_BG)
                        .stroke(Stroke::new(1.0_f32, PRIMARY_BLUE.gamma_multiply(0.4)))
                        .corner_radius(6.0)
                        .min_size(Vec2::new(240.0, 32.0));

                        if ui
                            .add(show_more_btn)
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            self.history_visible_limit =
                                self.history_visible_limit.saturating_add(10);
                        }
                    });
                    ui.add_space(4.0);
                } else if total_count > 5 {
                    draw_divider(ui, SUBTLE_BG);
                    ui.add_space(8.0);
                    ui.vertical_centered(|ui| {
                        horizontal_row(ui, 24.0, |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Showing all {total_count} deployment records."
                                ))
                                .typo(Typo::Caption)
                                .color(TEXT_MUTED),
                            );
                            let collapse_btn = egui::Button::new(
                                RichText::new("Collapse to Last 5")
                                    .typo(Typo::Caption)
                                    .color(TEXT_SECONDARY),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE);
                            if ui
                                .add(collapse_btn)
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.history_visible_limit = 5;
                            }
                        });
                    });
                    ui.add_space(4.0);
                }
            });

        if let Some(cancel_id) = cancel_target {
            self.cancel_deployment(&cancel_id);
        } else if let Some(dep) = delete_target {
            self.pending_delete_confirm = Some(ConfirmDeleteAction::DeploymentRecord {
                deployment_id: dep.id,
                project_name: dep.project_name,
                env_name: dep.environment,
                version: dep.version,
            });
        } else if let Some((target, is_rb)) = redeploy_target {
            self.trigger_redeploy_from_record(&target, is_rb);
        }

        ui.add_space(18.0);

        let selected_dep = self
            .selected_deployment_id
            .as_ref()
            .and_then(|id| filtered.iter().find(|d| &d.id == id))
            .or_else(|| filtered.first())
            .cloned();

        if let Some(dep) = &selected_dep {
            if self.selected_deployment_id.as_ref() != Some(&dep.id) {
                self.selected_deployment_id = Some(dep.id.clone());
            }
        }

        // Code changes to display:
        // Strictly show code changes of the active project.
        // If a deployment of the active project is selected and has recorded git_changes, show them.
        // Otherwise, if viewing a project workspace, fall back to the active project's current git branch commits (cached_git_changes).
        let active_project_name = project_filter.and_then(|pid| {
            self.storage
                .projects
                .iter()
                .find(|p| p.id == pid)
                .map(|p| p.name.clone())
        });

        let code_changes_data: Option<Vec<String>> = if let Some(dep) = &selected_dep {
            if !dep.git_changes.is_empty() {
                Some(dep.git_changes.clone())
            } else if active_project_name.is_some() {
                if !self.cached_git_changes.is_empty() {
                    Some(self.cached_git_changes.clone())
                } else {
                    None
                }
            } else {
                None
            }
        } else if active_project_name.is_some() {
            if !self.cached_git_changes.is_empty() {
                Some(self.cached_git_changes.clone())
            } else {
                None
            }
        } else {
            None
        };

        if let Some(changes_lines) = code_changes_data {
            egui::Frame::new()
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0_f32, CARD_BORDER))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(18, 12))
                .show(ui, |ui| {
                    horizontal_row(ui, 28.0, |ui| {
                        let (chev_r, chev_resp) =
                            ui.allocate_exact_size(Vec2::new(18.0, 18.0), Sense::click());
                        let chev_resp = chev_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        draw_icon_chevron(
                            ui.painter(),
                            chev_r.center(),
                            !self.changes_collapsed,
                            TEXT_PRIMARY,
                        );
                        ui.add_space(8.0);
                        let title_resp = ui.add(
                            egui::Button::new(
                                RichText::new("Code changes since last deploy")
                                    .typo(Typo::Label)
                                    .color(TEXT_PRIMARY),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE),
                        );
                        ui.add_space(6.0);
                        let count_str = format!("{} file(s) modified", changes_lines.len());
                        render_pill(ui, &count_str, WARN_PILL_BG, WARN_PILL_TEXT);

                        if chev_resp.clicked() || title_resp.clicked() {
                            self.changes_collapsed = !self.changes_collapsed;
                        }
                    });

                    if !self.changes_collapsed {
                        ui.add_space(8.0);
                        draw_divider(ui, SUBTLE_BG);
                        ui.add_space(8.0);
                        for line in &changes_lines {
                            let trimmed = line.trim();
                            let (status_char, path, char_color) =
                                if let Some(rest) = trimmed.strip_prefix("M ") {
                                    ("M", rest, WARNING)
                                } else if let Some(rest) = trimmed.strip_prefix("A ") {
                                    ("A", rest, SUCCESS)
                                } else if let Some(rest) = trimmed.strip_prefix("D ") {
                                    ("D", rest, DANGER)
                                } else if let Some(rest) = trimmed.strip_prefix("?? ") {
                                    ("?", rest, TEXT_MUTED)
                                } else if let Some(rest) = trimmed.strip_prefix("? ") {
                                    ("?", rest, TEXT_MUTED)
                                } else {
                                    ("•", trimmed, PRIMARY_BLUE)
                                };

                            ui.horizontal(|ui| {
                                ui.add_space(8.0);
                                let (badge_r, _) =
                                    ui.allocate_exact_size(Vec2::new(16.0, 18.0), Sense::hover());
                                ui.painter().text(
                                    badge_r.center(),
                                    Align2::CENTER_CENTER,
                                    status_char,
                                    Typo::MonoSm.font(),
                                    char_color,
                                );
                                ui.label(
                                    RichText::new(path)
                                        .typo(Typo::MonoSm)
                                        .color(TEXT_SECONDARY),
                                );
                            });
                        }
                    }
                });
            ui.add_space(14.0);
        }

        if let Some(dep) = selected_dep {
            self.render_terminal_window(ui, &dep);
        }
    }

    pub(crate) fn render_terminal_window(&mut self, ui: &mut egui::Ui, dep: &DeploymentRecord) {
        let is_running = dep.status == DeploymentStatus::Running || self.is_deployment_active(&dep.id);
        let term_h = if self.console_expanded_height { 480.0 } else { 260.0 };

        egui::Frame::new()
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

                // Header (36px): "Deployment log" + running spinner or status pill + Copy log + Clear + expand/collapse toggle
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

                // Decorative dots
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

                // Right controls in header
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("term_header_controls")
                        .max_rect(top_rect.shrink2(Vec2::new(12.0, 0.0)))
                        .layout(Layout::right_to_left(Align::Center)),
                );

                // Expand / Collapse height toggle
                let height_toggle_lbl = if self.console_expanded_height {
                    "Compact 260px"
                } else {
                    "Expand 480px"
                };
                let h_btn = egui::Button::new(
                    RichText::new(height_toggle_lbl)
                        .typo(Typo::Small)
                        .color(SB_TEXT_MUTED),
                )
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE);
                if child.add(h_btn).clicked() {
                    self.console_expanded_height = !self.console_expanded_height;
                }

                child.add_space(8.0);

                // Clear button
                let clr_btn = egui::Button::new(
                    RichText::new("Clear")
                        .typo(Typo::Small)
                        .color(SB_TEXT_MUTED),
                )
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE);
                if child.add(clr_btn).on_hover_text("Close log console").clicked() {
                    self.selected_deployment_id = None;
                    self.console_collapsed = true;
                }

                child.add_space(8.0);

                // Copy log button
                let copy_btn = egui::Button::new(
                    RichText::new("Copy log")
                        .typo(Typo::Small)
                        .color(Color32::WHITE),
                )
                .fill(SB_SURFACE_HOVER)
                .stroke(Stroke::new(1.0_f32, SB_BORDER_LINE))
                .corner_radius(CornerRadius::same(4));

                if child.add(copy_btn).on_hover_text("Copy full log to clipboard").clicked() {
                    let mut full_log = String::new();
                    for step in &dep.step_logs {
                        full_log.push_str(&format!("[{}] {}\n", step.status.label(), step.step_name));
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
                    let (spin_r, _) = child.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
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

                // Terminal Body: Monospace, auto-scroll when running, height 260px / 480px
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

                        egui::ScrollArea::vertical()
                            .max_height(term_h)
                            .stick_to_bottom(is_running)
                            .show(ui, |ui| {
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
                                        let (ic_r, _) =
                                            ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
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

                                        let (ch_r, _) =
                                            ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
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
                                                RichText::new(format!("$ {}", step_log.command_summary))
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
                                        let (ic_r, _) =
                                            ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
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
                    });
            });
    }

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

                let new_btn_txt = if is_compact_hdr { "+ New" } else { "+ New secret" };
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
            .filter(|s| self.secret_vault_cache.get(&s.key).is_some_and(|c| c.in_vault))
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
                if missing_count > 0 { WARNING } else { TEXT_MUTED },
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
                                ui.label(
                                    Typo::SmallMedium.rich("Actions").color(TEXT_SECONDARY),
                                );
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
                            let copy_tip = format!("Copy ${{{{ secrets.{} }}}} to clipboard", sec.key);
                            if kit::icon_button(
                                ui,
                                OutlineIcon::Copy,
                                ButtonSize::Sm,
                                &copy_tip,
                            )
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
                    let (row_rect, row_resp) = ui.allocate_exact_size(
                        Vec2::new(table_avail_w, row_h),
                        Sense::click(),
                    );
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
                        let (k_rect, _) = ui.allocate_exact_size(
                            Vec2::new(key_col_w - 8.0, row_h),
                            Sense::hover(),
                        );
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
                            egui::UiBuilder::new().max_rect(s_rect).layout(
                                Layout::left_to_right(Align::Center),
                            ),
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
                            egui::UiBuilder::new().max_rect(ub_rect).layout(
                                Layout::left_to_right(Align::Center),
                            ),
                        );
                        if referencing_envs.is_empty() {
                            ub_child.label(Typo::Small.rich("Not used").color(TEXT_MUTED));
                        } else {
                            for (badge, _) in referencing_envs.iter().take(2) {
                                kit::tag(
                                    &mut ub_child,
                                    badge,
                                    env_pastel_pill_colors(badge),
                                );
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
                            let copy_tip = format!("Copy ${{{{ secrets.{} }}}} to clipboard", sec.key);
                            if kit::icon_button(
                                ui,
                                OutlineIcon::Copy,
                                ButtonSize::Sm,
                                &copy_tip,
                            )
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
                    if kit::button(
                        ui,
                        ButtonVariant::Ghost,
                        ButtonSize::Sm,
                        toggle_lbl,
                        None,
                    )
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
