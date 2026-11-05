//! Secret creation and editing modal for OS Credential Vault.

use super::*;

impl ShiplyApp {
    pub(crate) fn render_secret_modal(&mut self, ctx: &egui::Context) {
        let Some(anim_t) = prepare_animated_modal(ctx, "secret_modal", &mut self.show_secret_modal)
        else {
            return;
        };

        let title = if self.secret_form_editing_existing {
            "Edit secret"
        } else {
            "New secret"
        };
        let (window, inner_w) = animated_modal_window(ctx, title, anim_t, None);
        window.show(ctx, |ui| {
            ui.set_width(inner_w);

            let os_name = if cfg!(windows) {
                "Windows Credential Manager"
            } else if cfg!(target_os = "macos") {
                "macOS Keychain"
            } else {
                "Linux Secret Service"
            };
            let subtitle = format!("Saved to {os_name}. The value never touches Shiply's database.");

            if render_modal_header(ui, "close_secret_modal", title, &subtitle) {
                self.show_secret_modal = false;
            }

            ui.add_space(SP_16);

            // Field 1: Name / Target Key
            ui.label(Typo::Label.rich("Secret name / target key *").color(MD_TEXT));
            ui.add_space(SP_8);
            if self.secret_form_editing_existing {
                horizontal_row(ui, INPUT_HEIGHT, |ui| {
                    ui.add_enabled(
                        false,
                        egui::TextEdit::singleline(&mut self.secret_form_key)
                            .font(Typo::Mono.font())
                            .desired_width(inner_w - 36.0),
                    );
                    ui.add_space(SP_8);
                    draw_outline_icon(
                        ui.painter(),
                        Pos2::new(ui.cursor().min.x + 8.0, ui.cursor().min.y + 10.0),
                        OutlineIcon::Lock,
                        TEXT_MUTED,
                    );
                });
                ui.add_space(4.0);
                ui.label(
                    Typo::Caption
                        .rich("Locked: Secret key cannot be renamed. Create a new secret if you need a different key.")
                        .color(TEXT_MUTED),
                );
            } else {
                modal_text_input(
                    ui,
                    "secret_modal_key_input",
                    &mut self.secret_form_key,
                    inner_w,
                    Some("e.g. indyserver, prod-ssh-server, DOCKER_TOKEN"),
                );
                ui.add_space(4.0);
                ui.label(
                    Typo::Caption
                        .rich("Used as the vault target and as ${{ secrets.NAME }} in pipelines.")
                        .color(TEXT_MUTED),
                );
            }

            ui.add_space(SP_16);

            // Field 2: Username
            ui.label(Typo::Label.rich("Username / account (optional)").color(MD_TEXT));
            ui.add_space(SP_8);
            modal_text_input(
                ui,
                "secret_modal_username_input",
                &mut self.secret_form_username,
                inner_w,
                Some("e.g. appndq, deploy, ubuntu"),
            );

            ui.add_space(SP_16);

            // Field 3: Secret Value
            let val_label = if self.secret_form_editing_existing {
                "New value *"
            } else {
                "Secret value / password *"
            };
            ui.label(Typo::Label.rich(val_label).color(MD_TEXT));
            ui.add_space(SP_8);
            horizontal_row(ui, INPUT_HEIGHT, |ui| {
                styled_text_edit_ext(
                    ui,
                    "secret_modal_value_input",
                    &mut self.secret_form_value,
                    inner_w - 44.0,
                    Some(if self.secret_form_editing_existing {
                        "Enter new password or secret token"
                    } else {
                        "Enter password or secret token (saved to OS Vault)"
                    }),
                    !self.secret_form_show_password,
                );
                ui.add_space(SP_8);
                let eye_icon = if self.secret_form_show_password {
                    OutlineIcon::EyeOff
                } else {
                    OutlineIcon::Eye
                };
                let eye_tooltip = if self.secret_form_show_password {
                    "Hide secret value"
                } else {
                    "Show secret value"
                };
                if kit::icon_button(ui, eye_icon, ButtonSize::Md, eye_tooltip).clicked() {
                    self.secret_form_show_password = !self.secret_form_show_password;
                }
            });
            ui.add_space(4.0);
            ui.label(
                Typo::Caption
                    .rich(if self.secret_form_editing_existing {
                        "Enter a new value to rotate this secret."
                    } else {
                        "Password, token, or private key stored in OS vault."
                    })
                    .color(TEXT_MUTED),
            );

            ui.add_space(SP_16);

            // Field 4: Description
            ui.label(Typo::Label.rich("Description / note (optional)").color(MD_TEXT));
            ui.add_space(SP_8);
            modal_text_input(
                ui,
                "secret_modal_desc_input",
                &mut self.secret_form_description,
                inner_w,
                Some("e.g. Production SSH Server or Registry Token"),
            );

            ui.add_space(SP_16);

            // Reference preview
            let preview_key = if self.secret_form_key.trim().is_empty() {
                "SECRET_KEY"
            } else {
                self.secret_form_key.trim()
            };
            let pipeline_expr = format!("${{{{ secrets.{preview_key} }}}}");

            egui::Frame::new()
                .fill(SUBTLE_BG)
                .stroke(Stroke::new(1.0_f32, CARD_BORDER))
                .corner_radius(CornerRadius::same(R_MD as u8))
                .inner_margin(Margin::same(12))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(Typo::Caption.rich("Pipeline syntax:").color(TEXT_MUTED));
                        ui.label(Typo::MonoSm.rich(&pipeline_expr).color(PRIMARY_BLUE));
                        ui.add_space(SP_8);
                        ui.label(Typo::Caption.rich("SSH target:").color(TEXT_MUTED));
                        ui.label(Typo::MonoSm.rich(preview_key).color(TEXT_SECONDARY));

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if kit::icon_button(
                                ui,
                                OutlineIcon::Copy,
                                ButtonSize::Sm,
                                "Copy pipeline syntax to clipboard",
                            )
                            .clicked()
                            {
                                ui.ctx().copy_text(pipeline_expr.clone());
                                self.status_banner = Some((format!("Copied '{pipeline_expr}' to clipboard!"), true));
                                self.status_banner_time = Some(Instant::now());
                            }
                        });
                    });
                });

            // Footer
            ui.add_space(SP_24);
            draw_divider(ui, MD_BORDER);
            ui.add_space(SP_16);

            let can_save = !self.secret_form_key.trim().is_empty() && !self.secret_form_value.is_empty();
            let save_lbl = if self.secret_form_editing_existing {
                "Update secret"
            } else {
                "Save to vault"
            };

            horizontal_row(ui, 40.0, |ui| {
                ui.label(
                    Typo::Caption
                        .rich("Credentials are encrypted in your OS vault.")
                        .color(MD_TEXT_MUTED),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let save_btn = modal_primary_button(save_lbl, can_save);
                    let enter_submitted = can_save && ctx.input(|i| i.key_pressed(egui::Key::Enter));
                    if (ui.add(save_btn).clicked() || enter_submitted) && can_save {
                        match self.save_secret_to_os_vault() {
                            Ok(_) => {
                                self.show_secret_modal = false;
                            }
                            Err(err) => {
                                self.status_banner = Some((err, false));
                                self.status_banner_time = Some(Instant::now());
                            }
                        }
                    }

                    ui.add_space(SP_8);
                    let cancel_btn = modal_secondary_button("Cancel");
                    if ui.add(cancel_btn).clicked() {
                        self.show_secret_modal = false;
                    }
                });
            });
        });
    }
}
