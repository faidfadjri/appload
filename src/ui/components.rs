//! Reusable UI widgets, form elements, status pills, and animated spinners.

use super::*;

// =============================================================================
// Bounded Layout Helpers & Fixed-Height Pill/Icon Painters (Never Stretches!)
// =============================================================================

pub(crate) fn horizontal_row<R>(
    ui: &mut egui::Ui,
    height: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let width = ui.available_width();
    ui.allocate_ui_with_layout(
        Vec2::new(width, height),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_max_height(height);
            add_contents(ui)
        },
    )
}

/// Consistent text input field with white fill (`INPUT_BG`), 1px hairline/strong border (`INPUT_BORDER`),
/// focus ring (`PRIMARY_BLUE`), structured proportional font (`Typo::Body`), and grey placeholder (`TEXT_PLACEHOLDER`).
pub(crate) fn styled_text_edit(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    value: &mut String,
    width: f32,
    hint: Option<&str>,
) -> bool {
    styled_text_edit_ext(ui, id_salt, value, width, hint, false)
}

pub(crate) fn styled_password_edit(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    value: &mut String,
    width: f32,
    hint: Option<&str>,
) -> bool {
    styled_text_edit_ext(ui, id_salt, value, width, hint, true)
}

pub(crate) type TextEditLayouter<'a> =
    &'a mut dyn FnMut(&egui::Ui, &str, f32) -> std::sync::Arc<egui::Galley>;

/// Options for the unified singleline input field core.
pub(crate) struct InputCoreOpts<'a> {
    pub hint: Option<&'a str>,
    pub hint_color: Option<Color32>,
    pub password: bool,
    pub left_pad: f32,
    pub right_pad: f32,
    pub height: f32,
    pub font: egui::FontId,
    pub text_color: Color32,
    pub bg_color: Color32,
    pub border_color: Color32,
    pub border_hover_color: Color32,
    pub border_focus_color: Color32,
    pub corner_radius: CornerRadius,
    pub layouter: Option<TextEditLayouter<'a>>,
    pub draw_bg_and_border: bool,
}

impl<'a> Default for InputCoreOpts<'a> {
    fn default() -> Self {
        Self {
            hint: None,
            hint_color: None,
            password: false,
            left_pad: 10.0,
            right_pad: 10.0,
            height: INPUT_HEIGHT, // 36.0
            font: Typo::Body.font(),
            text_color: TEXT_PRIMARY,
            bg_color: INPUT_BG,
            border_color: INPUT_BORDER,
            border_hover_color: INPUT_BORDER_HOVER,
            border_focus_color: PRIMARY_BLUE,
            corner_radius: CornerRadius::same(6),
            layouter: None,
            draw_bg_and_border: true,
        }
    }
}

/// Shared single-line text input core for all AppLoad text fields.
///
/// Allocates the entire input height (e.g. 36px) as the native `TextEdit`'s interactive area,
/// vertically centers the text galley (`Align::Center`), and draws the background and border
/// behind and around the field using the native response.
pub(crate) fn input_core(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    value: &mut String,
    width: f32,
    mut opts: InputCoreOpts<'_>,
) -> egui::text_edit::TextEditOutput {
    let height = opts.height;
    // Always reserve the exact width × height in the parent UI layout
    let (outer_rect, outer_resp) =
        ui.allocate_exact_size(Vec2::new(width, height), Sense::click_and_drag());

    let left_pad = opts.left_pad;
    let right_pad = opts.right_pad;
    let desired_text_w = (width - left_pad - right_pad).max(10.0);

    let row_height = ui.fonts(|f| f.row_height(&opts.font));
    let pad_y = ((height - row_height) * 0.5).floor().max(0.0) as i8;

    let mut edit = egui::TextEdit::singleline(value)
        .id_salt(&id_salt)
        .font(opts.font.clone())
        .text_color(opts.text_color)
        .password(opts.password)
        .frame(false)
        .min_size(Vec2::new(width, height))
        .desired_width(desired_text_w)
        .vertical_align(Align::Center)
        .margin(Margin {
            left: left_pad.round() as i8,
            right: right_pad.round() as i8,
            top: pad_y,
            bottom: pad_y,
        });

    if let Some(placeholder) = opts.hint {
        let color = opts.hint_color.unwrap_or(TEXT_PLACEHOLDER);
        edit = edit.hint_text(
            RichText::new(placeholder)
                .font(opts.font.clone())
                .color(color),
        );
    }

    let mut child_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(outer_rect)
            .layout(Layout::left_to_right(Align::Center)),
    );

    let mut output = if let Some(layouter) = opts.layouter.as_mut() {
        edit.layouter(*layouter).show(&mut child_ui)
    } else {
        edit.show(&mut child_ui)
    };

    // Guarantee the response rect is exactly the allocated field rect
    output.response.rect = outer_rect;

    if opts.draw_bg_and_border {
        ui.painter()
            .rect_filled(outer_rect, opts.corner_radius, opts.bg_color);

        let is_focused =
            output.response.has_focus() || ui.memory(|mem| mem.has_focus(output.response.id));
        let is_hovered = output.response.hovered()
            || outer_resp.hovered()
            || ui.rect_contains_pointer(outer_rect);

        let stroke = if is_focused {
            Stroke::new(1.5_f32, opts.border_focus_color)
        } else if is_hovered {
            Stroke::new(1.0_f32, opts.border_hover_color)
        } else {
            Stroke::new(1.0_f32, opts.border_color)
        };
        ui.painter()
            .rect_stroke(outer_rect, opts.corner_radius, stroke, StrokeKind::Inside);
    }

    output
}

pub(crate) fn styled_search_input(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    value: &mut String,
    width: f32,
    hint: Option<&str>,
) -> bool {
    let placeholder_str = hint.unwrap_or("Search...");
    let opts = InputCoreOpts {
        hint: Some(placeholder_str),
        left_pad: 32.0,
        right_pad: 10.0,
        ..Default::default()
    };
    let output = input_core(ui, id_salt, value, width, opts);

    // Left magnifying glass search icon
    let icon_center = Pos2::new(
        output.response.rect.left() + 14.0,
        output.response.rect.center().y,
    );
    draw_outline_icon(
        ui.painter(),
        icon_center,
        OutlineIcon::Search,
        TEXT_PLACEHOLDER,
    );

    output.response.changed()
}

pub(crate) fn styled_text_edit_ext(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    value: &mut String,
    width: f32,
    hint: Option<&str>,
    is_password: bool,
) -> bool {
    let placeholder_str = hint.unwrap_or("Enter value...");
    let opts = InputCoreOpts {
        hint: Some(placeholder_str),
        password: is_password,
        left_pad: 10.0,
        right_pad: 10.0,
        ..Default::default()
    };
    let output = input_core(ui, id_salt, value, width, opts);
    output.response.changed()
}

pub(crate) fn sidebar_search_text_edit(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    value: &mut String,
    width: f32,
    hint: Option<&str>,
) -> bool {
    let placeholder_str = hint.unwrap_or("Search projects...");
    let height = 32.0_f32;
    let has_clear = !value.is_empty();
    let clear_w = if has_clear { 28.0_f32 } else { 0.0_f32 };
    let edit_w = (width - clear_w).max(10.0);

    // 1. Allocate exact container rect within sidebar bounds
    let (container_rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());

    // 2. Background and border for the search box
    let is_focused = ui.memory(|mem| mem.has_focus(ui.make_persistent_id(&id_salt)));
    let is_hovered = ui.rect_contains_pointer(container_rect);
    let stroke = if is_focused {
        Stroke::new(1.5_f32, SB_ACCENT)
    } else if is_hovered {
        Stroke::new(1.0_f32, SB_ACCENT)
    } else {
        Stroke::new(1.0_f32, SB_BORDER_LINE)
    };
    ui.painter()
        .rect_filled(container_rect, CornerRadius::same(R_SM as u8), SB_SURFACE);
    ui.painter().rect_stroke(
        container_rect,
        CornerRadius::same(R_SM as u8),
        stroke,
        StrokeKind::Inside,
    );

    // 3. Left magnifying glass search icon (centered at x = left + 14.0)
    let icon_center = Pos2::new(container_rect.left() + 14.0, container_rect.center().y);
    draw_outline_icon(
        ui.painter(),
        icon_center,
        OutlineIcon::Search,
        SB_TEXT_FAINT,
    );

    // 4. Text input occupying the area before clear button
    let input_rect = Rect::from_min_max(
        container_rect.min,
        Pos2::new(container_rect.right() - clear_w, container_rect.bottom()),
    );
    let mut child_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(input_rect)
            .layout(Layout::left_to_right(Align::Center)),
    );

    let opts = InputCoreOpts {
        hint: Some(placeholder_str),
        hint_color: Some(SB_TEXT_FAINT),
        left_pad: 32.0, // starts well after the 14px icon
        right_pad: 6.0,
        height,
        font: Typo::Body.font(),
        text_color: SB_TEXT,
        draw_bg_and_border: false,
        ..Default::default()
    };
    let output = input_core(&mut child_ui, id_salt, value, edit_w, opts);

    // 5. Clear (x) button on the right
    let mut value_cleared = false;
    if has_clear {
        let clear_rect = Rect::from_min_max(
            Pos2::new(container_rect.right() - clear_w, container_rect.top()),
            container_rect.max,
        );
        let clear_resp = ui
            .interact(
                clear_rect,
                ui.make_persistent_id("sidebar_search_clear_btn"),
                Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let clear_fg = if clear_resp.hovered() {
            SB_TEXT
        } else {
            SB_TEXT_FAINT
        };
        draw_outline_icon(ui.painter(), clear_rect.center(), OutlineIcon::X, clear_fg);
        if clear_resp.clicked() {
            value.clear();
            value_cleared = true;
        }
    }

    output.response.changed() || value_cleared
}

/// Syntax-highlighted single-line text editor for pipeline commands, paths, and SCP sources.
/// Colors `{template}` variables in purple, CLI commands in blue, flags (`--flag`) in amber,
/// paths/strings in teal, and shell operators (`&&`, `|`, `;`) in crimson.
pub(crate) fn syntax_highlighted_text_edit(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    value: &mut String,
    width: f32,
    hint: Option<&str>,
) -> bool {
    let placeholder_str = hint.unwrap_or("Enter command or path...");
    let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
        let mut job = build_pipeline_syntax_layout_job(string);
        job.wrap.max_width = wrap_width;
        ui.fonts(|f| f.layout_job(job))
    };
    let is_multiline = value.contains('\n');
    if is_multiline {
        let rows = value.lines().count().clamp(2, 8);
        let edit = egui::TextEdit::multiline(value)
            .desired_rows(rows)
            .id_salt(id_salt)
            .font(Typo::Code.font())
            .background_color(INPUT_BG)
            .desired_width(width)
            .margin(Margin::symmetric(10, 8))
            .layouter(&mut layouter)
            .hint_text(Typo::Code.rich(placeholder_str).color(TEXT_PLACEHOLDER));
        let output = edit.show(ui);
        output.response.changed()
    } else {
        let opts = InputCoreOpts {
            hint: Some(placeholder_str),
            left_pad: 10.0,
            right_pad: 10.0,
            font: Typo::Code.font(),
            layouter: Some(&mut layouter),
            ..Default::default()
        };
        let output = input_core(ui, id_salt, value, width, opts);
        output.response.changed()
    }
}

pub fn build_pipeline_syntax_layout_job(text: &str) -> egui::text::LayoutJob {
    use egui::text::{LayoutJob, TextFormat};
    let mut job = LayoutJob::default();
    let font_id = Typo::Code.font();

    let color_template = PURPLE_PILL_TEXT; // {app_name}, {remote_dir}, etc.
    let color_keyword = BLUE_PILL_TEXT; // npm, pm2, cargo, git, ssh, etc.
    let color_flag = WARN_PILL_TEXT; // --update-env, -rf, etc.
    let color_path = TEAL_PILL_TEXT; // ./dist, /var/www/..., "..."
    let color_operator = FAIL_PILL_TEXT; // &&, ||, |, ;

    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];

        // 1. Whitespace
        if b.is_ascii_whitespace() {
            let start = i;
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            job.append(
                &text[start..i],
                0.0,
                TextFormat::simple(font_id.clone(), TEXT_PRIMARY),
            );
            continue;
        }

        // 2. Template Variable `{...}`
        if b == b'{' {
            let start = i;
            i += 1;
            while i < len && bytes[i] != b'}' && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < len && bytes[i] == b'}' {
                i += 1;
            }
            job.append(
                &text[start..i],
                0.0,
                TextFormat::simple(font_id.clone(), color_template),
            );
            continue;
        }

        // 3. Quoted String `"..."` or `'...'`
        if b == b'"' || b == b'\'' {
            let quote = b;
            let start = i;
            i += 1;
            while i < len && bytes[i] != quote {
                if bytes[i] == b'{' {
                    break;
                }
                i += 1;
            }
            if i < len && bytes[i] == quote {
                i += 1;
            }
            job.append(
                &text[start..i],
                0.0,
                TextFormat::simple(font_id.clone(), color_path),
            );
            continue;
        }

        // 4. Operators & Separators (`&&`, `||`, `|`, `;`, `,`)
        if b == b'&' || b == b'|' || b == b';' || b == b',' {
            let start = i;
            i += 1;
            while i < len && (bytes[i] == b'&' || bytes[i] == b'|') {
                i += 1;
            }
            job.append(
                &text[start..i],
                0.0,
                TextFormat::simple(font_id.clone(), color_operator),
            );
            continue;
        }

        // 5. General Token (stop at whitespace, `{`, quote, or operator)
        let start = i;
        while i < len
            && !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'{'
            && bytes[i] != b'"'
            && bytes[i] != b'\''
            && bytes[i] != b'&'
            && bytes[i] != b'|'
            && bytes[i] != b';'
            && bytes[i] != b','
        {
            i += 1;
        }
        let token = &text[start..i];
        let token_color = if token.starts_with('-') {
            color_flag
        } else if token.starts_with("./")
            || token.starts_with("../")
            || token.starts_with('/')
            || token.contains('/')
            || token.contains('\\')
            || token.ends_with(".json")
            || token.ends_with(".js")
            || token.ends_with(".ts")
            || token.ends_with(".ps1")
            || token.ends_with(".env")
        {
            color_path
        } else if token.eq_ignore_ascii_case("npm")
            || token.eq_ignore_ascii_case("npx")
            || token.eq_ignore_ascii_case("yarn")
            || token.eq_ignore_ascii_case("pnpm")
            || token.eq_ignore_ascii_case("node")
            || token.eq_ignore_ascii_case("pm2")
            || token.eq_ignore_ascii_case("cargo")
            || token.eq_ignore_ascii_case("git")
            || token.eq_ignore_ascii_case("docker")
            || token.eq_ignore_ascii_case("systemctl")
            || token.eq_ignore_ascii_case("service")
            || token.eq_ignore_ascii_case("ssh")
            || token.eq_ignore_ascii_case("scp")
            || token.eq_ignore_ascii_case("mkdir")
            || token.eq_ignore_ascii_case("rm")
            || token.eq_ignore_ascii_case("cp")
            || token.eq_ignore_ascii_case("mv")
            || token.eq_ignore_ascii_case("chmod")
            || token.eq_ignore_ascii_case("chown")
            || token.eq_ignore_ascii_case("tar")
            || token.eq_ignore_ascii_case("curl")
            || token.eq_ignore_ascii_case("wget")
            || token.eq_ignore_ascii_case("export")
            || token.eq_ignore_ascii_case("cd")
            || token.eq_ignore_ascii_case("echo")
            || token.eq_ignore_ascii_case("python")
            || token.eq_ignore_ascii_case("pytest")
            || token.eq_ignore_ascii_case("go")
            || token.eq_ignore_ascii_case("cmdkey")
            || token.eq_ignore_ascii_case("security")
            || token.eq_ignore_ascii_case("secret-tool")
            || token.eq_ignore_ascii_case("sudo")
            || token.eq_ignore_ascii_case("run")
            || token.eq_ignore_ascii_case("build")
            || token.eq_ignore_ascii_case("test")
            || token.eq_ignore_ascii_case("install")
            || token.eq_ignore_ascii_case("ci")
            || token.eq_ignore_ascii_case("restart")
            || token.eq_ignore_ascii_case("reload")
            || token.eq_ignore_ascii_case("start")
            || token.eq_ignore_ascii_case("stop")
            || token.eq_ignore_ascii_case("delete")
            || token.eq_ignore_ascii_case("status")
            || token.eq_ignore_ascii_case("store")
        {
            color_keyword
        } else {
            TEXT_PRIMARY
        };

        job.append(token, 0.0, TextFormat::simple(font_id.clone(), token_color));
    }

    job
}

/// Renders a modern modal header according to design tokens:
/// Title (--fs-title 18px, weight 600, --md-text) + description (--fs-small 13px, --md-text-muted, margin-top 4px)
/// + anchored 32x32 close button top-right (radius --r-md 8px, hover bg --md-panel-bg).
pub(crate) fn render_modal_header(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    title: &str,
    subtitle: &str,
) -> bool {
    let full_w = ui.available_width();
    let top_pos = ui.cursor().min;

    // Position the 32x32 close button within the top-right corner of the modal
    let close_rect = Rect::from_min_size(
        Pos2::new(top_pos.x + full_w - 32.0, top_pos.y),
        Vec2::new(32.0, 32.0),
    );
    let close_resp = ui
        .interact(close_rect, ui.make_persistent_id(id_salt), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    let (title_rect, _) = ui.allocate_exact_size(Vec2::new(full_w - 40.0, 26.0), Sense::hover());
    ui.painter().text(
        Pos2::new(title_rect.left(), title_rect.center().y),
        Align2::LEFT_CENTER,
        title,
        Typo::H2.font(),
        MD_TEXT,
    );

    let close_bg = if close_resp.hovered() {
        MD_PANEL_BG
    } else {
        Color32::TRANSPARENT
    };
    let close_fg = if close_resp.hovered() {
        MD_TEXT
    } else {
        MD_TEXT_MUTED
    };

    ui.painter()
        .rect_filled(close_rect, CornerRadius::same(R_MD as u8), close_bg);
    let c = close_rect.center();
    let s = Stroke::new(1.6_f32, close_fg);
    ui.painter().line_segment(
        [
            Pos2::new(c.x - 4.5, c.y - 4.5),
            Pos2::new(c.x + 4.5, c.y + 4.5),
        ],
        s,
    );
    ui.painter().line_segment(
        [
            Pos2::new(c.x + 4.5, c.y - 4.5),
            Pos2::new(c.x - 4.5, c.y + 4.5),
        ],
        s,
    );

    ui.add_space(SP_4);
    ui.label(Typo::Small.rich(subtitle).color(MD_TEXT_MUTED));

    close_resp.on_hover_text("Close dialog (Esc)").clicked()
}

/// Calculates responsive modal width: min(560px, 100vw - 32px); below 480px it becomes a full-screen sheet.
#[allow(dead_code)]
pub(crate) fn modal_width_and_inner(ctx: &egui::Context) -> (f32, f32) {
    let screen_w = ctx.screen_rect().width();
    if screen_w < 480.0 {
        let modal_w = screen_w;
        let inner_w = (screen_w - 24.0).max(200.0);
        (modal_w, inner_w)
    } else {
        let modal_w = 560.0_f32.min(screen_w - 32.0).max(300.0);
        let inner_w = modal_w - 48.0;
        (modal_w, inner_w)
    }
}

/// Standardized dialog window frame adhering to design tokens:
/// width 560px, bg --md-bg, border 1px --md-border, radius --r-xl (14px), padding 24px, shadow 0 24px 60px rgba(2,6,23,0.35)
pub(crate) fn modal_dialog_frame_for(ctx: &egui::Context) -> egui::Frame {
    let screen_w = ctx.screen_rect().width();
    let is_sheet = screen_w < 480.0;
    egui::Frame::new()
        .fill(MD_BG)
        .stroke(Stroke::new(1.0_f32, MD_BORDER))
        .corner_radius(if is_sheet {
            CornerRadius::ZERO
        } else {
            CornerRadius::same(14)
        })
        .shadow(if is_sheet {
            Shadow::NONE
        } else {
            Shadow {
                offset: [0, 24],
                blur: 60,
                spread: 0,
                color: SHADOW_MODAL,
            }
        })
        .inner_margin(if is_sheet {
            Margin::same(12)
        } else {
            Margin::same(24)
        })
}

/// Unified modal text input (height 36px, font Typo::Body 13px, bg MD_INPUT_BG, border 1px MD_BORDER, radius R_MD 6px).
pub(crate) fn modal_text_input(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    value: &mut String,
    width: f32,
    placeholder: Option<&str>,
) -> egui::Response {
    let hint_str = placeholder.unwrap_or("Enter value...");
    let opts = InputCoreOpts {
        hint: Some(hint_str),
        hint_color: Some(TEXT_MUTED),
        left_pad: 12.0,
        right_pad: 12.0,
        bg_color: MD_INPUT_BG,
        border_color: MD_BORDER,
        border_hover_color: ACCENT,
        border_focus_color: PRIMARY_BLUE,
        text_color: MD_TEXT,
        corner_radius: CornerRadius::same(R_MD as u8),
        ..Default::default()
    };
    let output = input_core(ui, id_salt, value, width, opts);
    output.response
}

/// Unified modal primary action button (height 40px, padding 0 16px, font --fs-base 14px, radius --r-md 8px, bg --accent).
pub(crate) fn modal_primary_button(text: &str, enabled: bool) -> egui::Button<'static> {
    egui::Button::new(Typo::Label.rich(text.to_string()).color(if enabled {
        Color32::WHITE
    } else {
        TEXT_PLACEHOLDER
    }))
    .fill(if enabled { PRIMARY_BLUE } else { SUBTLE_BG })
    .stroke(if enabled {
        Stroke::NONE
    } else {
        Stroke::new(1.0_f32, INPUT_BORDER)
    })
    .corner_radius(R_MD)
    .min_size(Vec2::new(0.0, 40.0))
}

/// Unified modal secondary button (height 40px, padding 0 16px, font --fs-base 14px, radius --r-md 8px, white bg, border 1px --md-border, hover --md-panel-bg).
pub(crate) fn modal_secondary_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(Typo::Label.rich(text.to_string()).color(MD_TEXT))
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0_f32, MD_BORDER))
        .corner_radius(R_MD)
        .min_size(Vec2::new(0.0, 40.0))
}

/// Renders a tab button with uniform horizontal padding (`16px` left & right) and consistent
/// bordered styling so adjacent tabs have an exact, uniform visual gap.
#[allow(dead_code)]
pub(crate) fn render_uniform_tab_button(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    label: &str,
    is_active: bool,
    height: f32,
) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        Typo::Label.font(),
        if is_active {
            Color32::WHITE
        } else {
            TEXT_SECONDARY
        },
    );
    let width = galley.size().x + 32.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let resp = ui
        .interact(rect, ui.make_persistent_id(id_salt), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    let (fill, stroke, text_color) = if is_active {
        (
            PRIMARY_BLUE,
            Stroke::new(1.5_f32, PRIMARY_BLUE),
            Color32::WHITE,
        )
    } else if resp.hovered() {
        (
            BLUE_PILL_BG,
            Stroke::new(1.2_f32, PRIMARY_BLUE),
            PRIMARY_BLUE,
        )
    } else {
        (CARD_BG, Stroke::new(1.0_f32, CARD_BORDER), TEXT_SECONDARY)
    };

    ui.painter().rect_filled(rect, CornerRadius::same(6), fill);
    ui.painter()
        .rect_stroke(rect, CornerRadius::same(6), stroke, StrokeKind::Inside);

    let colored_galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), Typo::Label.font(), text_color);
    ui.painter().galley(
        Pos2::new(
            rect.center().x - colored_galley.size().x / 2.0,
            rect.center().y - colored_galley.size().y / 2.0,
        ),
        colored_galley,
        text_color,
    );

    resp
}
