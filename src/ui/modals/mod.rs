//! Standardized modal dialogs (Project, Environment, Confirm Delete, Bulk Deploy, GitHub Update, Secret).

use super::*;

pub(crate) mod bulk_deploy;
pub(crate) mod confirm_delete;
pub(crate) mod environment;
pub(crate) mod project;
pub(crate) mod secret;
pub(crate) mod update;

/// Prepares a modal with smooth opacity fade, slide translateY entrance, backdrop click-to-dismiss, and Escape key handling.
/// Returns `Some(anim_t)` if the modal is currently open or animating, or `None` if completely closed.
pub(crate) fn prepare_animated_modal(
    ctx: &egui::Context,
    modal_id: &str,
    is_open: &mut bool,
) -> Option<f32> {
    let anim_t = ctx.animate_bool(egui::Id::new(("anim_modal", modal_id)), *is_open);
    if anim_t <= 0.005 {
        return None;
    }

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        *is_open = false;
        return None;
    }

    let overlay_color = OVERLAY_SCRIM.gamma_multiply(anim_t);
    egui::Area::new(egui::Id::new(("modal_backdrop", modal_id)))
        .order(egui::Order::Middle)
        .fixed_pos(Pos2::ZERO)
        .show(ctx, |ui| {
            let screen_rect = ctx.screen_rect();
            let resp = ui.allocate_rect(screen_rect, Sense::click());
            ui.painter()
                .rect_filled(screen_rect, CornerRadius::ZERO, overlay_color);
            if resp.clicked() {
                *is_open = false;
            }
        });

    if !*is_open {
        return None;
    }

    Some(anim_t)
}

/// Constructs a standardized modal window with responsive width, frameless dialog styling, and vertical slide offset.
pub(crate) fn animated_modal_window(
    ctx: &egui::Context,
    title: &str,
    anim_t: f32,
    frame_override: Option<egui::Frame>,
) -> (egui::Window<'static>, f32) {
    animated_modal_window_with_width(ctx, title, anim_t, 560.0, frame_override)
}

/// Constructs a standardized modal window with custom target width (e.g. 720px for bulk deploy).
pub(crate) fn animated_modal_window_with_width(
    ctx: &egui::Context,
    title: &str,
    anim_t: f32,
    target_width: f32,
    frame_override: Option<egui::Frame>,
) -> (egui::Window<'static>, f32) {
    let screen_w = ctx.screen_rect().width();
    let (modal_w, inner_w) = if screen_w < 480.0 {
        let modal_w = screen_w;
        let inner_w = (screen_w - 24.0).max(200.0);
        (modal_w, inner_w)
    } else {
        let modal_w = target_width.min(screen_w - 32.0).max(300.0);
        let inner_w = modal_w - 48.0;
        (modal_w, inner_w)
    };
    let slide_y = (1.0 - anim_t) * -18.0;
    let frame = frame_override.unwrap_or_else(|| modal_dialog_frame_for(ctx));
    let window = egui::Window::new(title.to_string())
        .order(egui::Order::Foreground)
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .auto_sized()
        .default_width(modal_w)
        .min_width(modal_w)
        .max_width(modal_w)
        .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, slide_y))
        .frame(frame);
    (window, inner_w)
}

impl AppLoadApp {
    pub fn open_new_project_modal(&mut self) {
        self.new_project_ps1_path.clear();
        self.new_project_name.clear();
        self.new_project_path.clear();
        self.new_project_desc.clear();
        self.new_project_group.clear();
        self.new_project_imported_envs.clear();
        self.new_project_import_notice = None;
        self.show_new_project_modal = true;
    }
}
