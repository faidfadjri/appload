//! Tests for project context menus, viewport boundary flips, and action dispatch.

use super::*;

#[test]
fn test_project_context_menu_and_duplication_ui() {
    let ctx = egui::Context::default();
    AppLoadApp::configure_light_workspace_style(&ctx);

    let mut storage = crate::storage::AppStorage::default();
    let proj = Project {
        id: "proj-orig".to_string(),
        name: "Backend Service".to_string(),
        description: "Production API".to_string(),
        local_path: ".".to_string(),
        environments: vec![EnvironmentConfig {
            key: "prod".to_string(),
            name: "Production".to_string(),
            app_name: "backend-prod".to_string(),
            servers: vec!["10.0.0.1".to_string()],
            remote_user: "deploy".to_string(),
            vault_target: "v1".to_string(),
            remote_dir: "/app".to_string(),
            env_source: None,
            http_proxy: None,
            steps: vec![PipelineStep::new(
                "Build",
                StepKind::LocalCommand {
                    command: "cargo build".to_string(),
                    allow_failure: false,
                    shell: None,
                },
            )],
        }],
        group: None,
        sort_order: None,
    };
    storage.projects.push(proj);

    let mut app = AppLoadApp::new(storage);
    app.selection = SidebarSelection::Project("proj-orig".to_string());

    // Render sidebar and project workspace to verify options and context menus execute without panic
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_sidebar(ui.ctx(), 1.0);
            app.render_project_workspace(ui, "proj-orig");
        });
    });

    // Test duplication action via storage
    let dup = app
        .storage
        .duplicate_project("proj-orig")
        .expect("duplicate project");
    assert_eq!(dup.name, "Backend Service (Copy)");
    assert_eq!(dup.id, "proj-orig-copy");
    assert_eq!(app.storage.projects.len(), 2);
}

#[test]
fn test_sidebar_custom_context_menu_viewport_flip_and_actions() {
    let ctx = egui::Context::default();
    AppLoadApp::configure_light_workspace_style(&ctx);

    let mut storage = crate::storage::AppStorage::default();
    storage.projects = vec![
        Project {
            id: "p-first".to_string(),
            name: "First Project".to_string(),
            description: "".to_string(),
            local_path: ".".to_string(),
            environments: vec![],
            group: Some("Services".to_string()),
            sort_order: Some(0),
        },
        Project {
            id: "p-second".to_string(),
            name: "Second Project".to_string(),
            description: "".to_string(),
            local_path: ".".to_string(),
            environments: vec![],
            group: None,
            sort_order: Some(1),
        },
    ];

    let mut app = AppLoadApp::new(storage);

    // 1. Trigger context menu for first project near top-left
    app.sidebar_context_menu = Some(SidebarContextMenuState {
        project_id: "p-first".to_string(),
        click_pos: Pos2::new(100.0, 100.0),
        submenu_open: false,
    });

    let _ = ctx.run(Default::default(), |ctx| {
        app.render_sidebar_custom_context_menu(ctx);
    });
    assert!(app.sidebar_context_menu.is_some());

    // 2. Trigger context menu near bottom-right to test viewport flip math
    let screen = ctx.screen_rect();
    let near_bottom_right = Pos2::new(screen.right() - 10.0, screen.bottom() - 10.0);
    app.sidebar_context_menu = Some(SidebarContextMenuState {
        project_id: "p-second".to_string(),
        click_pos: near_bottom_right,
        submenu_open: true,
    });

    let _ = ctx.run(Default::default(), |ctx| {
        app.render_sidebar_custom_context_menu(ctx);
    });
    assert!(app.sidebar_context_menu.is_some());

    // 3. Test destructive styling tokens
    assert_eq!(DESTRUCTIVE_TEXT, Color32::from_rgb(238, 29, 54));
    assert_eq!(
        DESTRUCTIVE_HOVER_BG,
        Color32::from_rgba_premultiplied(18, 2, 4, 20)
    );
    assert_eq!(
        DESTRUCTIVE_ACTIVE_BG,
        Color32::from_rgba_premultiplied(35, 4, 8, 38)
    );

    // 4. Test outline icons rendering without panic
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let icons = [
                OutlineIcon::Trash,
                OutlineIcon::Refresh,
                OutlineIcon::ArrowUp,
                OutlineIcon::ArrowDown,
                OutlineIcon::Edit,
                OutlineIcon::RotateCcw,
                OutlineIcon::Key,
                OutlineIcon::GripVertical,
                OutlineIcon::Copy,
                OutlineIcon::Folder,
                OutlineIcon::ChevronRight,
                OutlineIcon::ChevronDown,
                OutlineIcon::ChevronUp,
                OutlineIcon::Check,
                OutlineIcon::X,
            ];
            for (i, icon) in icons.iter().enumerate() {
                let c = Pos2::new(20.0 + i as f32 * 20.0, 20.0);
                draw_outline_icon(ui.painter(), c, *icon, SB_TEXT);
            }
        });
    });
}
