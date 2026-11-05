//! First time empty state view.

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
}
