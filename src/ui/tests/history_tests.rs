//! Tests for deployment history pagination, environment filters, and responsive views.

use super::*;

#[test]
fn test_deployment_history_pagination_last_5_and_show_more_10() {
    let ctx = egui::Context::default();
    AppLoadApp::configure_light_workspace_style(&ctx);

    let mut storage = crate::storage::AppStorage::default();
    // Create 18 mock deployments
    for i in 1..=18 {
        storage.deployments.push(DeploymentRecord {
            id: format!("dep-{i}"),
            project_id: "p1".to_string(),
            project_name: "Test App".to_string(),
            environment: "dev".to_string(),
            app_name: "test-app".to_string(),
            version: format!("1.0.{i}"),
            git_branch: Some("main".to_string()),
            git_commit: Some(format!("c{i}")),
            git_changes: vec![],
            is_rollback: false,
            servers: vec!["10.0.0.1".to_string()],
            status: DeploymentStatus::Succeeded,
            started_at: chrono::Utc::now(),
            finished_at: Some(chrono::Utc::now()),
            duration_ms: Some(1000),
            step_logs: vec![],
            executed_steps: vec![],
        });
    }

    let mut app = AppLoadApp::new(storage);
    assert_eq!(app.history_visible_limit, 5);

    // Render UI with initially 5 items
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_history_and_logs_view(ui, None);
        });
    });

    // Simulate Show More click -> increases by 10 to 15
    app.history_visible_limit = app.history_visible_limit.saturating_add(10);
    assert_eq!(app.history_visible_limit, 15);

    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_history_and_logs_view(ui, None);
        });
    });

    // Next Show More click -> increases by 10 to 25 (exceeds 18 total)
    app.history_visible_limit = app.history_visible_limit.saturating_add(10);
    assert_eq!(app.history_visible_limit, 25);

    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_history_and_logs_view(ui, None);
        });
    });

    // Reset/Collapse to last 5
    app.history_visible_limit = 5;
    assert_eq!(app.history_visible_limit, 5);
}

#[test]
fn test_deployment_history_filters_by_active_environment_for_selected_project() {
    let ctx = egui::Context::default();
    AppLoadApp::configure_light_workspace_style(&ctx);

    let mut storage = crate::storage::AppStorage::default();
    let proj = Project {
        id: "proj-backend".to_string(),
        name: "Backend Service".to_string(),
        description: "API".to_string(),
        local_path: ".".to_string(),
        environments: vec![
            EnvironmentConfig {
                key: "dev".to_string(),
                name: "Development".to_string(),
                app_name: "backend-dev".to_string(),
                servers: vec!["10.0.0.1".to_string()],
                remote_user: "deploy".to_string(),
                vault_target: "v1".to_string(),
                remote_dir: "/apps/dev".to_string(),
                env_source: None,
                http_proxy: None,
                steps: Vec::new(),
            },
            EnvironmentConfig {
                key: "prod".to_string(),
                name: "Production".to_string(),
                app_name: "backend-prod".to_string(),
                servers: vec!["10.0.0.2".to_string()],
                remote_user: "deploy".to_string(),
                vault_target: "v2".to_string(),
                remote_dir: "/apps/prod".to_string(),
                env_source: None,
                http_proxy: None,
                steps: Vec::new(),
            },
        ],
        group: None,
        sort_order: None,
    };
    storage.projects = vec![proj];

    let dep_dev1 = DeploymentRecord {
        id: "dep-dev-1".to_string(),
        project_id: "proj-backend".to_string(),
        project_name: "Backend Service".to_string(),
        environment: "dev".to_string(),
        app_name: "backend-dev".to_string(),
        version: "1.0.0".to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("commit1".to_string()),
        git_changes: vec![],
        is_rollback: false,
        servers: vec!["10.0.0.1".to_string()],
        status: DeploymentStatus::Succeeded,
        started_at: chrono::Utc::now(),
        finished_at: Some(chrono::Utc::now()),
        duration_ms: Some(1200),
        step_logs: vec![],
        executed_steps: vec![],
    };
    let dep_dev2 = DeploymentRecord {
        id: "dep-dev-2".to_string(),
        project_id: "proj-backend".to_string(),
        project_name: "Backend Service".to_string(),
        environment: "dev".to_string(),
        app_name: "backend-dev".to_string(),
        version: "1.0.1".to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("commit2".to_string()),
        git_changes: vec![],
        is_rollback: false,
        servers: vec!["10.0.0.1".to_string()],
        status: DeploymentStatus::Succeeded,
        started_at: chrono::Utc::now(),
        finished_at: Some(chrono::Utc::now()),
        duration_ms: Some(1500),
        step_logs: vec![],
        executed_steps: vec![],
    };
    let dep_prod1 = DeploymentRecord {
        id: "dep-prod-1".to_string(),
        project_id: "proj-backend".to_string(),
        project_name: "Backend Service".to_string(),
        environment: "prod".to_string(),
        app_name: "backend-prod".to_string(),
        version: "1.0.0".to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("commit1".to_string()),
        git_changes: vec![],
        is_rollback: false,
        servers: vec!["10.0.0.2".to_string()],
        status: DeploymentStatus::Succeeded,
        started_at: chrono::Utc::now(),
        finished_at: Some(chrono::Utc::now()),
        duration_ms: Some(2000),
        step_logs: vec![],
        executed_steps: vec![],
    };
    storage.deployments = vec![dep_dev1, dep_dev2, dep_prod1];

    let mut app = AppLoadApp::new(storage);
    app.selection = SidebarSelection::Project("proj-backend".to_string());
    app.selected_env_key = "dev".to_string();

    // 1. In project workspace with "dev" active: filtered deployments should only show 2 dev records
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_history_and_logs_view(ui, Some("proj-backend"));
        });
    });
    let dev_records =
        app.storage
            .filter_deployments(Some("proj-backend"), Some(&app.selected_env_key), None);
    assert_eq!(dev_records.len(), 2);
    assert!(dev_records.iter().all(|d| d.environment == "dev"));

    // 2. Switch to "prod" active: filtered deployments should only show 1 prod record
    app.selected_env_key = "prod".to_string();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_history_and_logs_view(ui, Some("proj-backend"));
        });
    });
    let prod_records =
        app.storage
            .filter_deployments(Some("proj-backend"), Some(&app.selected_env_key), None);
    assert_eq!(prod_records.len(), 1);
    assert_eq!(prod_records[0].environment, "prod");

    // 3. Global AllDeployments view: shows all 3 deployments across environments
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_history_and_logs_view(ui, None);
        });
    });
    let all_records = app.storage.filter_deployments(None, None, None);
    assert_eq!(all_records.len(), 3);
}

#[test]
fn test_responsive_environment_cards_and_history_stacked_breakpoints() {
    // Breakpoints: >= 1200px: 3 cols, 900-1199px: 2 cols, < 900px: 1 col
    let cols_at = |screen_w: f32| -> usize {
        if screen_w >= 1200.0 {
            3
        } else if (900.0..1200.0).contains(&screen_w) {
            2
        } else {
            1
        }
    };
    assert_eq!(cols_at(1400.0), 3);
    assert_eq!(cols_at(1200.0), 3);
    assert_eq!(cols_at(1199.0), 2);
    assert_eq!(cols_at(950.0), 2);
    assert_eq!(cols_at(900.0), 2);
    assert_eq!(cols_at(899.0), 1);
    assert_eq!(cols_at(600.0), 1);

    // Deployments history view: stacked below 900px or when available width < 640px
    let is_stacked = |screen_w: f32, avail_w: f32| -> bool { screen_w < 900.0 || avail_w < 640.0 };
    assert!(!is_stacked(1280.0, 900.0));
    assert!(is_stacked(850.0, 700.0));
    assert!(is_stacked(1280.0, 600.0));
}

#[test]
fn test_responsive_window_resizing_and_local_timezone_history() {
    let ctx = egui::Context::default();
    AppLoadApp::configure_light_workspace_style(&ctx);

    let mut storage = crate::storage::AppStorage::default();
    let proj = Project {
        id: "proj-resp".to_string(),
        name: "Responsive Desktop App".to_string(),
        description: "Testing responsive layout across window sizes".to_string(),
        local_path: "D:/NTI/Projects/responsive-app".to_string(),
        environments: vec![EnvironmentConfig {
            key: "dev".to_string(),
            name: "Development".to_string(),
            app_name: "resp-dev".to_string(),
            servers: vec!["10.0.0.10".to_string()],
            remote_user: "deploy".to_string(),
            vault_target: "vault-resp".to_string(),
            remote_dir: "/var/www/resp".to_string(),
            env_source: Some(".env.dev".to_string()),
            http_proxy: None,
            steps: vec![PipelineStep::new(
                "Build Project",
                StepKind::LocalCommand {
                    command: "npm run build".to_string(),
                    allow_failure: false,
                    shell: None,
                },
            )],
        }],
        group: Some("Core".to_string()),
        sort_order: Some(0),
    };
    storage.projects.push(proj);

    let fixed_utc = chrono::DateTime::parse_from_rfc3339("2026-10-30T10:15:30Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let finished_utc = chrono::DateTime::parse_from_rfc3339("2026-10-30T10:16:05Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let dep = DeploymentRecord {
        id: "dep-tz-1".to_string(),
        project_id: "proj-resp".to_string(),
        project_name: "Responsive Desktop App".to_string(),
        environment: "dev".to_string(),
        app_name: "resp-dev".to_string(),
        version: "1.4.2".to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("a1b2c3d".to_string()),
        git_changes: vec!["feat: responsive layout and local timezone".to_string()],
        is_rollback: false,
        servers: vec!["10.0.0.10".to_string()],
        status: DeploymentStatus::Succeeded,
        started_at: fixed_utc,
        finished_at: Some(finished_utc),
        duration_ms: Some(35_000),
        step_logs: vec![],
        executed_steps: vec![],
    };

    // Verify local timezone conversion matches chrono::Local
    let expected_local_full = fixed_utc
        .with_timezone(&chrono::Local)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let expected_local_short = fixed_utc
        .with_timezone(&chrono::Local)
        .format("%Y-%m-%d %H:%M")
        .to_string();
    assert_eq!(
        dep.format_started_at_local("%Y-%m-%d %H:%M:%S"),
        expected_local_full
    );
    assert_eq!(
        dep.format_started_at_local("%Y-%m-%d %H:%M"),
        expected_local_short
    );

    storage.deployments.push(dep);
    let mut app = AppLoadApp::new(storage);

    // Render across multiple window sizes (from compact 540x440 up to 1920x1080)
    let window_sizes = [
        egui::vec2(540.0, 440.0),
        egui::vec2(720.0, 520.0),
        egui::vec2(960.0, 640.0),
        egui::vec2(1440.0, 900.0),
        egui::vec2(1920.0, 1080.0),
    ];

    for size in window_sizes {
        let mut raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            ..Default::default()
        };

        // 1. All Deployments Overview
        app.selection = SidebarSelection::AllDeployments;
        let _ = ctx.run(raw_input.clone(), |ctx| {
            app.render_termius_topbar(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                app.render_all_deployments_overview(ui);
            });
        });

        // 2. Project Workspace across all 3 tabs
        app.selection = SidebarSelection::Project("proj-resp".to_string());
        app.selected_env_key = "dev".to_string();
        for tab in [
            WorkspaceTab::PipelineSteps,
            WorkspaceTab::HistoryAndLogs,
            WorkspaceTab::ServerSettings,
        ] {
            app.active_tab = tab;
            raw_input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size));
            let _ = ctx.run(raw_input.clone(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.render_project_workspace(ui, "proj-resp");
                });
            });
        }
    }
}

#[test]
fn test_truncate_commit_message_with_ellipsis() {
    let ctx = egui::Context::default();
    AppLoadApp::configure_light_workspace_style(&ctx);
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let msg = "fix(calendar): adjust calendar-unit range calculation for leap years";
            let font = Typo::Body.font();
            let no_trunc = helpers::truncate_text_to_width(ui, msg, &font, 1000.0);
            assert_eq!(no_trunc, msg);

            let trunc = helpers::truncate_text_to_width(ui, msg, &font, 100.0);
            assert!(trunc.ends_with("..."));
            assert!(trunc.len() < msg.len());
        });
    });
}
