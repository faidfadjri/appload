//! Tests for modal window lifecycle, responsive breakpoints, and form reset behavior.

use super::*;

#[test]
fn test_responsive_modal_width_and_sheet_breakpoints() {
    let ctx = egui::Context::default();

    // 1. Desktop screen width 1280px
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0))),
            ..Default::default()
        },
        |ctx| {
            let (modal_w, inner_w) = modal_width_and_inner(ctx);
            assert_eq!(modal_w, 560.0);
            assert_eq!(inner_w, 560.0 - 48.0);
        },
    );

    // 2. Narrow screen width 400px (< 480px) becomes full-screen sheet
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 700.0))),
            ..Default::default()
        },
        |ctx| {
            let (modal_w, inner_w) = modal_width_and_inner(ctx);
            assert_eq!(modal_w, 400.0);
            assert_eq!(inner_w, 400.0 - 24.0);
        },
    );
}

#[test]
fn test_modal_primary_button_grey_when_disabled() {
    let _btn_disabled = modal_primary_button("Create Project", false);
    let _btn_enabled = modal_primary_button("Create Project", true);
}

#[test]
fn test_open_new_project_modal_resets_form_fields() {
    let mut app = ShiplyApp::new(crate::storage::AppStorage::default());
    app.new_project_name = "dirty_name".to_string();
    app.new_project_path = "dirty_path".to_string();
    app.new_project_ps1_path = "dirty.ps1".to_string();

    app.open_new_project_modal();

    assert!(app.show_new_project_modal);
    assert!(app.new_project_name.is_empty());
    assert!(app.new_project_path.is_empty());
    assert!(app.new_project_ps1_path.is_empty());
}

#[test]
fn test_prepare_animated_modal_and_window_lifecycle() {
    let ctx = egui::Context::default();
    let mut is_open = false;

    let anim_none = prepare_animated_modal(&ctx, "test_modal", &mut is_open);
    assert!(anim_none.is_none());

    is_open = true;
    let _ = ctx.run(Default::default(), |ctx| {
        let anim_some = prepare_animated_modal(ctx, "test_modal", &mut is_open);
        assert!(anim_some.is_some());
        let (window, inner_w) = animated_modal_window(ctx, "Test Window", anim_some.unwrap(), None);
        assert!(inner_w > 0.0);
        window.show(ctx, |ui| {
            ui.label("Inside modal");
        });
    });
}

#[test]
fn test_add_and_edit_project_modal_full_height_and_centered_position() {
    let ctx = egui::Context::default();
    ShiplyApp::configure_light_workspace_style(&ctx);

    let mut storage = crate::storage::AppStorage::default();
    storage.projects.push(Project {
        id: "proj-edit".to_string(),
        name: "Existing Project".to_string(),
        description: "Desc".to_string(),
        local_path: ".".to_string(),
        environments: Vec::new(),
        group: None,
        sort_order: Some(0),
    });

    let mut app = ShiplyApp::new(storage);
    app.open_new_project_modal();

    let screen_size = egui::vec2(1280.0, 800.0);
    // Run two frames so egui settles the auto-sized window rect
    for _ in 0..2 {
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, screen_size)),
            ..Default::default()
        };
        let _ = ctx.run(raw_input, |ctx| {
            app.render_new_project_modal(ctx);
        });
    }

    let add_area = ctx.memory(|mem| mem.area_rect(egui::Id::new("Add Project")));
    assert!(add_area.is_some(), "Add Project modal area should exist");
    let add_rect = add_area.unwrap();
    assert!(
        add_rect.height() > 500.0,
        "Add Project modal must not shrink vertically (got height {})",
        add_rect.height()
    );
    assert!(
        add_rect.top() < 200.0,
        "Add Project modal must be vertically centered, not sunk too low (got top {})",
        add_rect.top()
    );

    // Now test Edit Project modal
    app.show_new_project_modal = false;
    app.open_edit_project_modal(0);
    for _ in 0..2 {
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, screen_size)),
            ..Default::default()
        };
        let _ = ctx.run(raw_input, |ctx| {
            app.render_edit_project_modal(ctx);
        });
    }

    let edit_area = ctx.memory(|mem| mem.area_rect(egui::Id::new("Edit Project")));
    assert!(edit_area.is_some(), "Edit Project modal area should exist");
    let edit_rect = edit_area.unwrap();
    assert!(
        edit_rect.height() > 500.0,
        "Edit Project modal must not shrink vertically (got height {})",
        edit_rect.height()
    );
    assert!(
        edit_rect.top() < 200.0,
        "Edit Project modal must be vertically centered, not sunk too low (got top {})",
        edit_rect.top()
    );
}

#[test]
fn test_delete_single_deployment_and_clear_history_modal() {
    let ctx = egui::Context::default();
    ShiplyApp::configure_light_workspace_style(&ctx);

    let mut storage = crate::storage::AppStorage::default();
    let now = chrono::Utc::now();
    for (id, status) in [
        ("dep-a", DeploymentStatus::Succeeded),
        ("dep-b", DeploymentStatus::Failed),
        ("dep-c", DeploymentStatus::Cancelled),
    ] {
        storage.deployments.push(DeploymentRecord {
            id: id.to_string(),
            project_id: "backend-cdo".to_string(),
            project_name: "Backend Customer Data Ops".to_string(),
            environment: "production".to_string(),
            app_name: "backend-cdo".to_string(),
            version: "1.1.22".to_string(),
            git_branch: Some("main".to_string()),
            git_commit: Some("70c0d68".to_string()),
            git_changes: vec![],
            is_rollback: false,
            servers: vec!["10.0.0.1".to_string()],
            status,
            started_at: now,
            finished_at: Some(now),
            duration_ms: Some(1500),
            step_logs: vec![],
            executed_steps: vec![],
        });
    }

    let mut app = ShiplyApp::new(storage);
    app.selected_deployment_id = Some("dep-a".to_string());
    app.pending_delete_confirm = Some(ConfirmDeleteAction::DeploymentRecord {
        deployment_id: "dep-a".to_string(),
        project_name: "Backend Customer Data Ops".to_string(),
        env_name: "production".to_string(),
        version: "1.1.22".to_string(),
    });

    // Render history and delete confirmation modal to verify no panics or layout issues
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_history_and_logs_view(ui, None);
        });
        app.render_delete_confirmation_modal(ctx);
    });

    // Verify ClearDeploymentHistory modal variant renders cleanly as well
    app.pending_delete_confirm = Some(ConfirmDeleteAction::ClearDeploymentHistory {
        project_id: Some("backend-cdo".to_string()),
        project_name: Some("Backend Customer Data Ops".to_string()),
        env_key: Some("production".to_string()),
        env_name: Some("Production".to_string()),
        count: 3,
    });
    let _ = ctx.run(Default::default(), |ctx| {
        app.render_delete_confirmation_modal(ctx);
    });
}
