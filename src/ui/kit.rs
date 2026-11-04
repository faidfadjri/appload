use super::icons::*;
use super::theme::*;
use super::typography::*;
use eframe::egui::{
    self, Align, Color32, CornerRadius, FontId, Layout, Margin, Pos2, Rect, Sense, Shadow, Stroke,
    StrokeKind, Vec2, WidgetText,
};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Danger,
    DangerSolid,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    /// Height 36px, horizontal padding 14px, LABEL font (13px medium)
    Md,
    /// Height 28px, horizontal padding 10px, SMALL_MEDIUM font (12px medium)
    Sm,
}

impl ButtonSize {
    #[inline]
    pub fn height(self) -> f32 {
        match self {
            ButtonSize::Md => 36.0,
            ButtonSize::Sm => 28.0,
        }
    }

    #[inline]
    pub fn pad_x(self) -> f32 {
        match self {
            ButtonSize::Md => 14.0,
            ButtonSize::Sm => 10.0,
        }
    }

    #[inline]
    pub fn font(self) -> FontId {
        match self {
            ButtonSize::Md => Typo::Label.font(),
            ButtonSize::Sm => Typo::SmallMedium.font(),
        }
    }
}

/// Standardized SaaS button adhering to design tokens.
#[allow(dead_code)]
pub(crate) fn button(
    ui: &mut egui::Ui,
    variant: ButtonVariant,
    size: ButtonSize,
    label: &str,
    icon: Option<OutlineIcon>,
) -> egui::Response {
    let font_id = size.font();
    let pad_x = size.pad_x();
    let height = size.height();

    // Determine colors based on variant
    let (base_fill, base_border, text_color, hover_fill, hover_border, hover_text) = match variant {
        ButtonVariant::Primary => (
            PRIMARY_BLUE,
            Color32::TRANSPARENT,
            Color32::WHITE,
            PRIMARY_BLUE_HOVER,
            Color32::TRANSPARENT,
            Color32::WHITE,
        ),
        ButtonVariant::Secondary => (
            CARD_BG,
            CARD_BORDER,
            TEXT_PRIMARY,
            SUBTLE_BG,
            INPUT_BORDER,
            TEXT_PRIMARY,
        ),
        ButtonVariant::Ghost => (
            Color32::TRANSPARENT,
            Color32::TRANSPARENT,
            TEXT_SECONDARY,
            SUBTLE_BG,
            Color32::TRANSPARENT,
            TEXT_PRIMARY,
        ),
        ButtonVariant::Danger => (CARD_BG, CARD_BORDER, DANGER, DANGER_SOFT, DANGER, DANGER),
        ButtonVariant::DangerSolid => (
            DANGER,
            Color32::TRANSPARENT,
            Color32::WHITE,
            DANGER_HOVER, // Darker red on hover
            Color32::TRANSPARENT,
            Color32::WHITE,
        ),
    };

    let text_galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font_id, text_color);
    let icon_w = if icon.is_some() { 16.0 + 6.0 } else { 0.0 };
    let width = pad_x * 2.0 + icon_w + text_galley.size().x;

    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let hovered = resp.hovered();
    let enabled = ui.is_enabled();

    let (fill, border, current_text_color) = if !enabled {
        (
            base_fill.gamma_multiply(0.5),
            base_border.gamma_multiply(0.5),
            text_color.gamma_multiply(0.5),
        )
    } else if hovered {
        (hover_fill, hover_border, hover_text)
    } else {
        (base_fill, base_border, text_color)
    };

    // Draw background and border
    ui.painter()
        .rect_filled(rect, CornerRadius::same(R_MD as u8), fill);
    if border != Color32::TRANSPARENT {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(R_MD as u8),
            Stroke::new(1.0_f32, border),
            StrokeKind::Inside,
        );
    }

    // Draw icon + text centered horizontally
    let content_w = icon_w + text_galley.size().x;
    let content_start_x = rect.center().x - content_w / 2.0;

    let text_x = if let Some(ic) = icon {
        let ic_center = Pos2::new(content_start_x + 8.0, rect.center().y);
        draw_outline_icon(ui.painter(), ic_center, ic, current_text_color);
        content_start_x + icon_w
    } else {
        content_start_x
    };

    let text_y = rect.center().y - text_galley.size().y / 2.0;
    ui.painter()
        .galley(Pos2::new(text_x, text_y), text_galley, current_text_color);

    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

/// Square ghost icon button with required hover tooltip.
#[allow(dead_code)]
pub(crate) fn icon_button(
    ui: &mut egui::Ui,
    icon: OutlineIcon,
    size: ButtonSize,
    tooltip: &str,
) -> egui::Response {
    let side = match size {
        ButtonSize::Md => 32.0,
        ButtonSize::Sm => 28.0,
    };
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(side), Sense::click());
    let hovered = resp.hovered();
    let enabled = ui.is_enabled();

    if enabled && hovered {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(R_MD as u8), SUBTLE_BG);
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(R_MD as u8),
            Stroke::new(1.0_f32, CARD_BORDER),
            StrokeKind::Inside,
        );
    }

    let color = if !enabled {
        TEXT_MUTED.gamma_multiply(0.5)
    } else if hovered {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };

    draw_outline_icon(ui.painter(), rect.center(), icon, color);
    let resp = resp.on_hover_text(tooltip);
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Success,
    Warning,
    Danger,
    Info,
    Neutral,
}

impl StatusKind {
    #[inline]
    pub fn colors(self) -> (Color32, Color32) {
        match self {
            StatusKind::Success => (SUCCESS_SOFT, SUCCESS),
            StatusKind::Warning => (WARNING_SOFT, WARNING),
            StatusKind::Danger => (DANGER_SOFT, DANGER),
            StatusKind::Info => (INFO_SOFT, PRIMARY_BLUE),
            StatusKind::Neutral => (SUBTLE_BG, TEXT_SECONDARY),
        }
    }
}

/// Status badge: 20px high, fully rounded, horizontal padding 8px, 6px dot in status colour, CAPTION text.
pub fn status_badge(ui: &mut egui::Ui, kind: StatusKind, text: &str) {
    let (bg, fg) = kind.colors();
    let font_id = Typo::Caption.font();
    let text_galley = ui.painter().layout_no_wrap(text.to_string(), font_id, fg);

    let h = 20.0;
    let dot_r = 3.0;
    let dot_w = 6.0 + 6.0; // 6px dot + 6px gap
    let w = 8.0 + dot_w + text_galley.size().x + 8.0;

    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
    let radius = CornerRadius::same((h / 2.0) as u8);

    ui.painter().rect_filled(rect, radius, bg);

    let dot_c = Pos2::new(rect.left() + 8.0 + dot_r, rect.center().y);
    ui.painter().circle_filled(dot_c, dot_r, fg);

    let text_pos = Pos2::new(
        rect.left() + 8.0 + dot_w,
        rect.center().y - text_galley.size().y / 2.0,
    );
    ui.painter().galley(text_pos, text_galley, fg);
}

/// Generic tag: 20px high, 4px radius, CAPTION font, no dot.
pub fn tag(ui: &mut egui::Ui, text: &str, colors: (Color32, Color32)) {
    let (bg, fg) = colors;
    let font_id = Typo::Caption.font();
    let text_galley = ui.painter().layout_no_wrap(text.to_string(), font_id, fg);

    let h = 20.0;
    let w = 8.0 + text_galley.size().x + 8.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());

    ui.painter()
        .rect_filled(rect, CornerRadius::same(R_SM as u8), bg);
    let text_pos = Pos2::new(
        rect.center().x - text_galley.size().x / 2.0,
        rect.center().y - text_galley.size().y / 2.0,
    );
    ui.painter().galley(text_pos, text_galley, fg);
}

/// Small rounded grey count badge for tabs and section titles.
pub fn count_badge(ui: &mut egui::Ui, count: usize) {
    let text = count.to_string();
    let font_id = Typo::Caption.font();
    let text_galley = ui.painter().layout_no_wrap(text, font_id, TEXT_MUTED);

    let h = 18.0;
    let w = (text_galley.size().x + 10.0).max(18.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());

    ui.painter()
        .rect_filled(rect, CornerRadius::same((h / 2.0) as u8), SUBTLE_BG);
    let text_pos = Pos2::new(
        rect.center().x - text_galley.size().x / 2.0,
        rect.center().y - text_galley.size().y / 2.0,
    );
    ui.painter().galley(text_pos, text_galley, TEXT_MUTED);
}

/// Standardized underline tab: height 40px, Label font, active 2px primary underline and TEXT_PRIMARY.
pub fn underline_tab(
    ui: &mut egui::Ui,
    is_active: bool,
    label: &str,
    count: Option<usize>,
) -> egui::Response {
    let font_id = Typo::Label.font();
    let text_color = if is_active {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };
    let text_galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font_id, text_color);

    let count_w = if let Some(c) = count {
        let cg = ui
            .painter()
            .layout_no_wrap(c.to_string(), Typo::Caption.font(), TEXT_MUTED);
        (cg.size().x + 10.0).max(18.0) + 6.0
    } else {
        0.0
    };

    let total_w = text_galley.size().x + count_w + 16.0;
    let h = TAB_HEIGHT; // 40.0
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(total_w, h), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

    let text_pos = Pos2::new(
        rect.left() + 8.0,
        rect.center().y - text_galley.size().y / 2.0,
    );
    let final_color = if is_active || resp.hovered() {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };
    ui.painter()
        .galley(text_pos, text_galley.clone(), final_color);

    if let Some(c) = count {
        let badge_x = text_pos.x + text_galley.size().x + 6.0;
        let badge_h = 18.0;
        let cg = ui
            .painter()
            .layout_no_wrap(c.to_string(), Typo::Caption.font(), TEXT_MUTED);
        let badge_w = (cg.size().x + 10.0).max(18.0);
        let badge_rect = Rect::from_min_size(
            Pos2::new(badge_x, rect.center().y - badge_h / 2.0),
            Vec2::new(badge_w, badge_h),
        );
        ui.painter()
            .rect_filled(badge_rect, CornerRadius::same(9), SUBTLE_BG);
        let c_pos = Pos2::new(
            badge_rect.center().x - cg.size().x / 2.0,
            badge_rect.center().y - cg.size().y / 2.0,
        );
        ui.painter().galley(c_pos, cg, TEXT_MUTED);
    }

    if is_active {
        let underline_rect = Rect::from_min_max(
            Pos2::new(rect.left(), rect.bottom() - 2.0),
            Pos2::new(rect.right(), rect.bottom()),
        );
        ui.painter()
            .rect_filled(underline_rect, CornerRadius::ZERO, PRIMARY_BLUE);
    }

    resp
}

/// Standardized card container with 1px CARD_BORDER, R_LG radius, CARD_BG, and subtle shadow.
pub fn card<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    egui::Frame::new()
        .fill(CARD_BG)
        .stroke(Stroke::new(1.0_f32, CARD_BORDER))
        .corner_radius(CornerRadius::same(R_LG as u8))
        .shadow(Shadow {
            offset: [0, 1],
            blur: 2,
            spread: 0,
            color: SHADOW_CARD,
        })
        .inner_margin(Margin::same(20))
        .show(ui, add_contents)
}

/// Standardized kit checkbox: 16x16, radius 4, primary fill when checked, LABEL text.
pub fn checkbox(
    ui: &mut egui::Ui,
    checked: &mut bool,
    label: impl Into<WidgetText>,
) -> egui::Response {
    let size = 16.0;
    let radius = CornerRadius::same(4);
    let label = label.into();
    let text_style = Typo::Label;

    let galley = label.into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        text_style.font(),
    );
    let total_w = size + 8.0 + galley.size().x;
    let total_h = size.max(galley.size().y).max(20.0);
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(total_w, total_h), Sense::click());
    if resp.clicked() {
        *checked = !*checked;
        resp.mark_changed();
    }

    let box_rect = Rect::from_min_size(
        Pos2::new(rect.left(), rect.center().y - size / 2.0),
        Vec2::splat(size),
    );

    if *checked {
        ui.painter().rect_filled(box_rect, radius, PRIMARY_BLUE);
        let p1 = Pos2::new(box_rect.left() + 3.5, box_rect.top() + 8.0);
        let p2 = Pos2::new(box_rect.left() + 6.5, box_rect.top() + 11.5);
        let p3 = Pos2::new(box_rect.left() + 12.5, box_rect.top() + 4.5);
        ui.painter()
            .line_segment([p1, p2], Stroke::new(1.8_f32, Color32::WHITE));
        ui.painter()
            .line_segment([p2, p3], Stroke::new(1.8_f32, Color32::WHITE));
    } else {
        let border_color = if resp.hovered() {
            INPUT_BORDER_HOVER
        } else {
            INPUT_BORDER
        };
        ui.painter().rect_filled(box_rect, radius, CARD_BG);
        ui.painter().rect_stroke(
            box_rect,
            radius,
            Stroke::new(1.0_f32, border_color),
            StrokeKind::Inside,
        );
    }

    let text_pos = Pos2::new(
        rect.left() + size + 8.0,
        rect.center().y - galley.size().y / 2.0,
    );
    let text_color = if *checked {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };
    ui.painter().galley(text_pos, galley, text_color);

    resp
}

/// Standardized card with title header, right actions, and 1px separator line under the header.
pub fn card_with_header<R>(
    ui: &mut egui::Ui,
    title: &str,
    actions: impl FnOnce(&mut egui::Ui),
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(Typo::H3.rich(title));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                actions(ui);
            });
        });
        ui.add_space(SP_12);
        let avail_w = ui.available_width();
        let (line_rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, 1.0), Sense::hover());
        ui.painter().line_segment(
            [line_rect.left_center(), line_rect.right_center()],
            Stroke::new(1.0_f32, CARD_BORDER),
        );
        ui.add_space(SP_12);
        add_contents(ui)
    })
}

/// Standardized page header: H1 + inline badges on left, subtitle in SMALL below, actions on right.
pub fn page_header(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: Option<&str>,
    badges: impl FnOnce(&mut egui::Ui),
    actions: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(SP_8, 0.0);
        ui.label(Typo::H1.rich(title));
        badges(ui);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(SP_8, 0.0);
            actions(ui);
        });
    });

    if let Some(sub) = subtitle {
        ui.add_space(4.0);
        ui.label(Typo::Small.rich(sub).color(TEXT_SECONDARY));
    }
    ui.add_space(SP_24);
}

/// Standardized section header: H2 + subtitle + actions on right.
pub fn section_header(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: Option<&str>,
    actions: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(SP_8, 0.0);
        ui.label(Typo::H2.rich(title));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(SP_8, 0.0);
            actions(ui);
        });
    });
    if let Some(sub) = subtitle {
        ui.add_space(2.0);
        ui.label(Typo::Small.rich(sub).color(TEXT_MUTED));
    }
    ui.add_space(SP_12);
}

/// Segmented control: SUBTLE_BG track, radius 6, height 32, active white segment with shadow.
pub fn segmented(ui: &mut egui::Ui, selected_idx: &mut usize, labels: &[&str]) -> bool {
    let mut changed = false;
    let segment_h = 26.0;

    egui::Frame::new()
        .fill(SUBTLE_BG)
        .corner_radius(CornerRadius::same(R_MD as u8))
        .inner_margin(Margin::symmetric(3, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
                for (idx, &label) in labels.iter().enumerate() {
                    let is_active = *selected_idx == idx;
                    let font_id = Typo::Label.font();
                    let text_color = if is_active {
                        TEXT_PRIMARY
                    } else {
                        TEXT_SECONDARY
                    };
                    let galley =
                        ui.painter()
                            .layout_no_wrap(label.to_string(), font_id, text_color);
                    let seg_w = galley.size().x + 20.0;

                    let (rect, resp) =
                        ui.allocate_exact_size(Vec2::new(seg_w, segment_h), Sense::click());
                    if resp.clicked() && !is_active {
                        *selected_idx = idx;
                        changed = true;
                    }

                    if is_active {
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same((R_MD - 1.0) as u8),
                            CARD_BG,
                        );
                        ui.painter().rect_stroke(
                            rect,
                            CornerRadius::same((R_MD - 1.0) as u8),
                            Stroke::new(1.0_f32, CARD_BORDER),
                            StrokeKind::Inside,
                        );
                    } else if resp.hovered() {
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same((R_MD - 1.0) as u8),
                            CARD_BG.gamma_multiply(0.4),
                        );
                    }

                    let text_pos = Pos2::new(
                        rect.center().x - galley.size().x / 2.0,
                        rect.center().y - galley.size().y / 2.0,
                    );
                    ui.painter().galley(text_pos, galley, text_color);
                }
            });
        });

    changed
}

/// Standardized empty state: centred icon in a 40px soft circle, H3 title, SMALL muted text, optional CTA.
#[allow(dead_code)]
pub(crate) fn empty_state(
    ui: &mut egui::Ui,
    icon: OutlineIcon,
    title: &str,
    text: &str,
    cta_label: Option<&str>,
) -> bool {
    let mut cta_clicked = false;
    ui.vertical_centered(|ui| {
        ui.add_space(SP_32);
        let (circle_rect, _) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::hover());
        ui.painter()
            .circle_filled(circle_rect.center(), 20.0, INFO_SOFT);
        draw_outline_icon(ui.painter(), circle_rect.center(), icon, PRIMARY_BLUE);

        ui.add_space(SP_12);
        ui.label(Typo::H3.rich(title));
        ui.add_space(4.0);
        ui.label(Typo::Small.rich(text).color(TEXT_MUTED));

        if let Some(cta) = cta_label {
            ui.add_space(SP_16);
            if button(
                ui,
                ButtonVariant::Primary,
                ButtonSize::Md,
                cta,
                Some(OutlineIcon::Plus),
            )
            .clicked()
            {
                cta_clicked = true;
            }
        }
        ui.add_space(SP_32);
    });
    cta_clicked
}
