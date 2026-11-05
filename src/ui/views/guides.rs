//! Onboarding and OS credentials guideline cards.

use super::*;

impl ShiplyApp {
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
                    if kit::button(ui, ButtonVariant::Ghost, ButtonSize::Sm, toggle_lbl, None)
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
                ((onboard_avail - spacing * (onboard_cols - 1) as f32) / onboard_cols as f32)
                    .floor()
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
                                let (num_rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(24.0), Sense::hover());
                                ui.painter()
                                    .circle_filled(num_rect.center(), 12.0, INFO_SOFT);
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
}
