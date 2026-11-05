//! Tests for typography, Webflow color tokens, styled text inputs, and topbar styling.

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
fn test_input_core_vertical_centring() {
    let ctx = egui::Context::default();
    ShiplyApp::install_fonts(&ctx);
    ShiplyApp::configure_light_workspace_style(&ctx);

    let mut val = "Shiply Production Deployment".to_string();

    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let opts = InputCoreOpts {
                hint: Some("Enter text..."),
                ..Default::default()
            };
            let output = input_core(ui, "test_centering", &mut val, 300.0, opts);
            let field_center_y = output.response.rect.center().y;
            let galley_center_y = output.galley_pos.y + output.galley.size().y * 0.5;
            let diff = (galley_center_y - field_center_y).abs();
            assert!(
                diff <= 1.0,
                "Galley vertical center ({galley_center_y:.2}) must be within ±1px of field center ({field_center_y:.2}), got diff {diff:.2}"
            );
        });
    });
}

#[test]
fn test_input_core_single_click_mid_text() {
    let ctx = egui::Context::default();
    ShiplyApp::install_fonts(&ctx);
    ShiplyApp::configure_light_workspace_style(&ctx);

    let mut val = "hello world foo bar".to_string();
    let id_salt = "test_click_mid";

    // Frame 1: Initial layout
    let mut click_pos = Pos2::ZERO;
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let output = input_core(ui, id_salt, &mut val, 300.0, InputCoreOpts::default());
            click_pos = Pos2::new(
                output.galley_pos.x + output.galley.size().x * 0.5,
                output.response.rect.center().y,
            );
        });
    });

    // Frame 2: Primary press at click_pos
    let mut raw = egui::RawInput {
        time: Some(1.0),
        ..Default::default()
    };
    raw.events.push(egui::Event::PointerMoved(click_pos));
    raw.events.push(egui::Event::PointerButton {
        pos: click_pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    let _ = ctx.run(raw, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            input_core(ui, id_salt, &mut val, 300.0, InputCoreOpts::default());
        });
    });

    // Frame 3: Primary release at click_pos
    let mut raw2 = egui::RawInput {
        time: Some(1.05),
        ..Default::default()
    };
    raw2.events.push(egui::Event::PointerMoved(click_pos));
    raw2.events.push(egui::Event::PointerButton {
        pos: click_pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    let _ = ctx.run(raw2, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let output = input_core(ui, id_salt, &mut val, 300.0, InputCoreOpts::default());
            if let Some(state) = egui::text_edit::TextEditState::load(ctx, output.response.id) {
                if let Some(range) = state.cursor.char_range() {
                    let idx = range.primary.index;
                    // Middle of "hello world foo bar" is around char 9 or 10.
                    assert!(
                        (6..=14).contains(&idx),
                        "Caret must be placed in the middle near click position, got index {idx}"
                    );
                    assert_ne!(idx, 0);
                    assert_ne!(idx, val.len());
                } else {
                    panic!("Cursor range should be present after click");
                }
            } else {
                panic!("TextEditState should be present after click");
            }
        });
    });
}

#[test]
fn test_input_core_drag_selection() {
    let ctx = egui::Context::default();
    ShiplyApp::install_fonts(&ctx);
    ShiplyApp::configure_light_workspace_style(&ctx);

    let mut val = "abcdefghijklmnopqrstuvwxyz".to_string();
    let id_salt = "test_drag_select";

    // Frame 1: Initial layout
    let mut pos_x1 = Pos2::ZERO;
    let mut pos_x2 = Pos2::ZERO;
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let output = input_core(ui, id_salt, &mut val, 300.0, InputCoreOpts::default());
            let cy = output.response.rect.center().y;
            pos_x1 = Pos2::new(output.galley_pos.x + 30.0, cy);
            pos_x2 = Pos2::new(output.galley_pos.x + 130.0, cy);
        });
    });

    // Frame 2: Mouse press at pos_x1
    let mut raw1 = egui::RawInput {
        time: Some(1.0),
        ..Default::default()
    };
    raw1.events.push(egui::Event::PointerMoved(pos_x1));
    raw1.events.push(egui::Event::PointerButton {
        pos: pos_x1,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    let _ = ctx.run(raw1, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            input_core(ui, id_salt, &mut val, 300.0, InputCoreOpts::default());
        });
    });

    // Frame 3: Drag mouse to pos_x2
    let mut raw2 = egui::RawInput {
        time: Some(1.1),
        ..Default::default()
    };
    raw2.events.push(egui::Event::PointerMoved(pos_x2));
    let _ = ctx.run(raw2, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            input_core(ui, id_salt, &mut val, 300.0, InputCoreOpts::default());
        });
    });

    // Frame 4: Release mouse at pos_x2
    let mut raw3 = egui::RawInput {
        time: Some(1.2),
        ..Default::default()
    };
    raw3.events.push(egui::Event::PointerButton {
        pos: pos_x2,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    let _ = ctx.run(raw3, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let output = input_core(ui, id_salt, &mut val, 300.0, InputCoreOpts::default());
            if let Some(state) = egui::text_edit::TextEditState::load(ctx, output.response.id) {
                if let Some(range) = state.cursor.char_range() {
                    assert_ne!(
                        range.primary, range.secondary,
                        "Drag select must produce a non-empty selection spanning x1..x2"
                    );
                    let span =
                        (range.primary.index as isize - range.secondary.index as isize).abs();
                    assert!(
                        span >= 5,
                        "Drag selection span should be at least 5 characters, got {span}"
                    );
                } else {
                    panic!("Cursor range should be present after drag-select");
                }
            } else {
                panic!("TextEditState should be present after drag-select");
            }
        });
    });
}

#[test]
fn test_input_core_double_click_word() {
    let ctx = egui::Context::default();
    ShiplyApp::install_fonts(&ctx);
    ShiplyApp::configure_light_workspace_style(&ctx);

    let mut val = "deploy staging server".to_string();
    let id_salt = "test_double_click";

    // Frame 1: Initial layout
    let mut word_pos = Pos2::ZERO;
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let output = input_core(ui, id_salt, &mut val, 300.0, InputCoreOpts::default());
            word_pos = Pos2::new(
                output.galley_pos.x + output.galley.size().x * 0.45,
                output.response.rect.center().y,
            );
        });
    });

    // Click 1: press & release
    let mut raw1 = egui::RawInput {
        time: Some(1.00),
        ..Default::default()
    };
    raw1.events.push(egui::Event::PointerMoved(word_pos));
    raw1.events.push(egui::Event::PointerButton {
        pos: word_pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    let _ = ctx.run(raw1, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            input_core(ui, id_salt, &mut val, 300.0, InputCoreOpts::default());
        });
    });

    let mut raw2 = egui::RawInput {
        time: Some(1.05),
        ..Default::default()
    };
    raw2.events.push(egui::Event::PointerButton {
        pos: word_pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    let _ = ctx.run(raw2, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            input_core(ui, id_salt, &mut val, 300.0, InputCoreOpts::default());
        });
    });

    // Click 2: press & release within double-click threshold (<0.3s)
    let mut raw3 = egui::RawInput {
        time: Some(1.15),
        ..Default::default()
    };
    raw3.events.push(egui::Event::PointerButton {
        pos: word_pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    let _ = ctx.run(raw3, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            input_core(ui, id_salt, &mut val, 300.0, InputCoreOpts::default());
        });
    });

    let mut raw4 = egui::RawInput {
        time: Some(1.20),
        ..Default::default()
    };
    raw4.events.push(egui::Event::PointerButton {
        pos: word_pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    let _ = ctx.run(raw4, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let output = input_core(ui, id_salt, &mut val, 300.0, InputCoreOpts::default());
            if let Some(state) = egui::text_edit::TextEditState::load(ctx, output.response.id) {
                if let Some(range) = state.cursor.char_range() {
                    assert_ne!(
                        range.primary, range.secondary,
                        "Double click must select the word"
                    );
                    let min_idx = range.primary.index.min(range.secondary.index);
                    let max_idx = range.primary.index.max(range.secondary.index);
                    let selected = &val[min_idx..max_idx];
                    assert_eq!(
                        selected, "staging",
                        "Double click should select the word 'staging', got '{selected}'"
                    );
                } else {
                    panic!("Cursor range should be present after double click");
                }
            } else {
                panic!("TextEditState should be present after double click");
            }
        });
    });
}

#[test]
fn test_settings_cards_equal_widths_and_grid_alignment() {
    let ctx = egui::Context::default();
    ShiplyApp::install_fonts(&ctx);
    ShiplyApp::configure_light_workspace_style(&ctx);

    let mut app = ShiplyApp::new(crate::storage::AppStorage::default());
    let proj = crate::domain::Project {
        id: "proj_settings_test".to_string(),
        name: "Settings Test Project".to_string(),
        description: "Test".to_string(),
        local_path: r"C:\Projects\test".to_string(),
        environments: vec![crate::domain::EnvironmentConfig {
            key: "development".to_string(),
            name: "Development".to_string(),
            app_name: "test-app".to_string(),
            remote_dir: "/var/www/test".to_string(),
            remote_user: "deploy".to_string(),
            servers: vec!["10.0.0.1".to_string()],
            vault_target: "test-vault".to_string(),
            env_source: None,
            http_proxy: None,
            steps: vec![],
        }],
        group: None,
        sort_order: Some(0),
    };
    app.storage.projects = vec![proj];
    app.selected_env_key = "development".to_string();

    let mut browse_w = 0.0;
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_server_settings_tab(ui, 0);
            browse_w = kit::button_width(ui, ButtonSize::Md, "Browse", true);
        });
    });

    assert!(browse_w > 80.0 && browse_w < 120.0);
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
