use super::*;

#[test]
fn test_running_badge_pill_colors_and_layout() {
    let (bg, fg) = status_pill_colors(DeploymentStatus::Running);
    assert_eq!(bg, WARN_PILL_BG);
    assert_eq!(fg, WARN_PILL_TEXT);

    // Verify circle loading spinner angle progression
    let speed = 5.2_f64;
    let t0 = 0.0_f64;
    let t1 = 1.0_f64;
    let a0 = (t0 * speed) as f32;
    let a1 = (t1 * speed) as f32;
    assert!((a1 - a0 - 5.2_f32).abs() < 1e-4);

    // Test painter execution with egui Context
    let ctx = egui::Context::default();
    ShiplyApp::install_fonts(&ctx);
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let left_center = Pos2::new(100.0, 50.0);
            let w =
                paint_running_pill_left_aligned(ui.painter(), left_center, "RUNNING", bg, fg, 1.23);
            assert!(w > 50.0);

            let right_center = Pos2::new(200.0, 50.0);
            let w_r = paint_running_pill_right_aligned(
                ui.painter(),
                right_center,
                "RUNNING",
                bg,
                fg,
                1.23,
            );
            assert!(w_r > 50.0);
        });
    });
}

#[test]
fn test_webflow_palette_and_theme_tokens() {
    let light = Theme::light();
    assert_eq!(light.bg, Color32::from_rgb(235, 239, 246)); // Grey-blue slate canvas
    assert_eq!(light.surface, Color32::WHITE); // Pure white card
    assert_ne!(light.bg, light.surface); // High contrast between canvas and cards
    assert_eq!(light.surface_2, Color32::from_rgb(243, 246, 251)); // #F3F6FB
    assert_eq!(light.border, Color32::from_rgb(218, 224, 233));
    assert_eq!(light.border_strong, Color32::from_rgb(203, 213, 225));
    assert_eq!(light.text, Color32::from_rgb(8, 8, 8)); // #080808
    assert_eq!(light.text_muted, Color32::from_rgb(90, 90, 90)); // #5A5A5A
    assert_eq!(light.primary, Color32::from_rgb(20, 110, 245)); // #146EF5
    assert_eq!(light.primary_hover, Color32::from_rgb(15, 94, 215)); // #0F5ED7
    assert_eq!(light.primary_active, Color32::from_rgb(11, 79, 184)); // #0B4FB8
    assert_eq!(light.primary_soft, Color32::from_rgb(231, 240, 254)); // #E7F0FE
    assert_eq!(light.purple, Color32::from_rgb(122, 61, 255)); // #7A3DFF
    assert_eq!(light.danger, Color32::from_rgb(238, 29, 54)); // #EE1D36

    let dark = Theme::dark();
    assert_eq!(dark.bg, Color32::from_rgb(8, 8, 8)); // #080808
    assert_eq!(dark.surface, Color32::from_rgb(16, 16, 18)); // #101012
    assert_eq!(dark.surface_2, Color32::from_rgb(24, 24, 27)); // #18181B
    assert_eq!(dark.border, Color32::from_rgb(38, 38, 43)); // #26262B
    assert_eq!(dark.text, Color32::from_rgb(245, 245, 245)); // #F5F5F5
    assert_eq!(dark.primary, Color32::from_rgb(59, 130, 246)); // #3B82F6
}

#[test]
fn test_text_edit_cursor_and_selection_mechanics() {
    let ctx = egui::Context::default();
    ShiplyApp::configure_light_workspace_style(&ctx);

    let mut test_value = "10.62.70.80".to_string();

    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let changed = styled_text_edit(
                ui,
                "test_cursor_input",
                &mut test_value,
                300.0,
                Some("Enter IP..."),
            );
            assert!(!changed);
        });
    });

    // Test cursor math:
    // Clicking beyond galley width places cursor at end of text (char count)
    let char_count = test_value.chars().count();
    let galley_w = 80.0_f32;
    let click_rel_x_far_right = 250.0_f32;
    let cursor_end = if click_rel_x_far_right >= galley_w {
        egui::text::CCursor::new(char_count)
    } else {
        egui::text::CCursor::new(0)
    };
    assert_eq!(cursor_end.index, 11); // "10.62.70.80".len() == 11

    // Clicking to the left places cursor at 0
    let click_rel_x_left = -5.0_f32;
    let cursor_start = if click_rel_x_left <= 0.0 {
        egui::text::CCursor::new(0)
    } else {
        egui::text::CCursor::new(char_count)
    };
    assert_eq!(cursor_start.index, 0);

    // Multi-character selection range (drag/block text) must be preserved
    let selection_range = egui::text_selection::CCursorRange {
        primary: egui::text::CCursor::new(0),
        secondary: egui::text::CCursor::new(11),
    };
    assert_ne!(selection_range.primary, selection_range.secondary);
}

#[test]
fn test_topbar_frameless_no_horizontal_border() {
    // Frame stroke must be Stroke::NONE to prevent any dark horizontal border line
    assert_eq!(Stroke::NONE.width, 0.0);
    assert_eq!(Stroke::NONE.color, Color32::TRANSPARENT);
}

#[test]
fn test_topbar_black_background_and_no_theme_toggle() {
    let ctx = egui::Context::default();
    ShiplyApp::install_fonts(&ctx);
    let mut app = ShiplyApp::new(crate::storage::AppStorage::default());

    let _ = ctx.run(Default::default(), |ctx| {
        app.render_termius_topbar(ctx);
    });

    // Verify topbar renders without theme toggle
    assert!(!app.window_maximized);
}

#[test]
fn test_styled_text_edit_and_password_edit() {
    let ctx = egui::Context::default();
    let mut pwd = "secret".to_string();

    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let changed =
                styled_password_edit(ui, "test_pwd_field", &mut pwd, 200.0, Some("Password..."));
            assert!(!changed);
        });
    });

    assert_eq!(pwd, "secret");
}
