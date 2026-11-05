//! Code changes since last deployment panel.

use super::*;

impl AppLoadApp {
    pub(crate) fn render_history_code_changes(
        &mut self,
        ui: &mut egui::Ui,
        changes_lines: &[String],
    ) {
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
                    for line in changes_lines {
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
                            ui.label(RichText::new(path).typo(Typo::MonoSm).color(TEXT_SECONDARY));
                        });
                    }
                }
            });
        ui.add_space(14.0);
    }
}
