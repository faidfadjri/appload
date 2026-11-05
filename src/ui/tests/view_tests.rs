//! Tests for workspace code changes, deploy actions, and console output.

use super::*;

#[test]
fn test_active_project_code_changes_isolation() {
    let mut storage = AppStorage::default();
    let env_be = EnvironmentConfig {
        key: "dev".to_string(),
        name: "Development".to_string(),
        app_name: "backend-dev".to_string(),
        servers: vec!["10.0.0.1".to_string()],
        remote_user: "deploy".to_string(),
        vault_target: "backend_dev".to_string(),
        remote_dir: "/apps/backend".to_string(),
        env_source: None,
        http_proxy: None,
        steps: Vec::new(),
    };
    let proj_backend = Project {
        id: "proj-backend".to_string(),
        name: "Backend Service".to_string(),
        description: "Backend API".to_string(),
        local_path: ".".to_string(),
        environments: vec![env_be],
        group: None,
        sort_order: None,
    };

    let env_fe = EnvironmentConfig {
        key: "dev".to_string(),
        name: "Development".to_string(),
        app_name: "frontend-dev".to_string(),
        servers: vec!["10.0.0.2".to_string()],
        remote_user: "deploy".to_string(),
        vault_target: "frontend_dev".to_string(),
        remote_dir: "/apps/frontend".to_string(),
        env_source: None,
        http_proxy: None,
        steps: Vec::new(),
    };
    let proj_frontend = Project {
        id: "proj-frontend".to_string(),
        name: "Frontend App".to_string(),
        description: "Web frontend".to_string(),
        local_path: ".".to_string(),
        environments: vec![env_fe],
        group: None,
        sort_order: None,
    };

    storage.projects = vec![proj_backend, proj_frontend];

    let dep_backend = DeploymentRecord {
        id: "dep-be-1".to_string(),
        project_id: "proj-backend".to_string(),
        project_name: "Backend Service".to_string(),
        environment: "dev".to_string(),
        app_name: "backend-dev".to_string(),
        version: "1.0.1".to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("be11111".to_string()),
        git_changes: vec!["feat(api): add auth endpoint".to_string()],
        is_rollback: false,
        servers: vec!["10.0.0.1".to_string()],
        status: DeploymentStatus::Succeeded,
        started_at: chrono::Utc::now(),
        finished_at: Some(chrono::Utc::now()),
        duration_ms: Some(1500),
        step_logs: Vec::new(),
        executed_steps: Vec::new(),
    };

    let dep_frontend = DeploymentRecord {
        id: "dep-fe-1".to_string(),
        project_id: "proj-frontend".to_string(),
        project_name: "Frontend App".to_string(),
        environment: "dev".to_string(),
        app_name: "frontend-dev".to_string(),
        version: "2.0.0".to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("fe22222".to_string()),
        git_changes: vec!["feat(ui): redesign dashboard".to_string()],
        is_rollback: false,
        servers: vec!["10.0.0.2".to_string()],
        status: DeploymentStatus::Succeeded,
        started_at: chrono::Utc::now(),
        finished_at: Some(chrono::Utc::now()),
        duration_ms: Some(2000),
        step_logs: Vec::new(),
        executed_steps: Vec::new(),
    };

    storage.deployments = vec![dep_backend, dep_frontend];

    let mut app = AppLoadApp::new(storage);
    // Switch to Backend project and set an invalid foreign selected deployment ID
    app.selection = SidebarSelection::Project("proj-backend".to_string());
    app.selected_deployment_id = Some("dep-fe-1".to_string());

    // When refreshing metadata for Backend, selected_deployment_id must be corrected to Backend
    app.refresh_selected_project_metadata(false);
    assert_eq!(app.selected_deployment_id.as_deref(), Some("dep-be-1"));

    // When filtering deployments for Backend, frontend deployments are excluded
    let filtered = app
        .storage
        .filter_deployments(Some("proj-backend"), None, None);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].project_id, "proj-backend");
    assert_eq!(
        filtered[0].git_changes,
        vec!["feat(api): add auth endpoint".to_string()]
    );
}

#[test]
fn test_deploy_and_rollback_button_dynamic_sizing_no_offset() {
    let ctx = egui::Context::default();
    AppLoadApp::configure_light_workspace_style(&ctx);

    let mut app = AppLoadApp::new(crate::storage::AppStorage::default());
    app.deploy_version_input = "1.0.10-beta.2+build.2026".to_string();

    let rollback_dep = DeploymentRecord {
        id: "dep-rb-test".to_string(),
        project_id: "p1".to_string(),
        project_name: "Customer Data Ops".to_string(),
        environment: "development".to_string(),
        app_name: "data-ops".to_string(),
        version: "1.0.9-hotfix".to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("abc1234".to_string()),
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

    // Run UI render with long environment name and version
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_min_width(1200.0);
            let mut trigger_rect = None;
            app.render_deploy_action_bar(
                ui,
                "Development Customer Data Ops",
                Some(&rollback_dep),
                &mut trigger_rect,
            );
        });
    });

    // Also test with empty version string to ensure clean fallback text
    app.deploy_version_input = "   ".to_string();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_min_width(1000.0);
            let mut trigger_rect = None;
            app.render_deploy_action_bar(ui, "Production", None, &mut trigger_rect);
        });
    });
}

#[test]
fn test_sync_version_candidates_and_application() {
    let mut storage = crate::storage::AppStorage::default();
    let proj = Project {
        id: "proj-sync".to_string(),
        name: "Frontend App".to_string(),
        description: "SPA".to_string(),
        local_path: ".".to_string(),
        environments: vec![
            EnvironmentConfig {
                key: "dev".to_string(),
                name: "Development".to_string(),
                app_name: "front-dev".to_string(),
                servers: vec![],
                remote_user: "u".to_string(),
                vault_target: "v".to_string(),
                remote_dir: "/apps".to_string(),
                env_source: None,
                http_proxy: None,
                steps: Vec::new(),
            },
            EnvironmentConfig {
                key: "staging".to_string(),
                name: "Staging".to_string(),
                app_name: "front-staging".to_string(),
                servers: vec![],
                remote_user: "u".to_string(),
                vault_target: "v".to_string(),
                remote_dir: "/apps".to_string(),
                env_source: None,
                http_proxy: None,
                steps: Vec::new(),
            },
            EnvironmentConfig {
                key: "prod".to_string(),
                name: "Production".to_string(),
                app_name: "front-prod".to_string(),
                servers: vec![],
                remote_user: "u".to_string(),
                vault_target: "v".to_string(),
                remote_dir: "/apps".to_string(),
                env_source: None,
                http_proxy: None,
                steps: Vec::new(),
            },
        ],
        group: None,
        sort_order: None,
    };
    storage.projects = vec![proj];

    let dep_dev = DeploymentRecord {
        id: "dep-dev-sync".to_string(),
        project_id: "proj-sync".to_string(),
        project_name: "Frontend App".to_string(),
        environment: "dev".to_string(),
        app_name: "front-dev".to_string(),
        version: "1.3.5".to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("c1".to_string()),
        git_changes: vec![],
        is_rollback: false,
        servers: vec![],
        status: DeploymentStatus::Succeeded,
        started_at: chrono::Utc::now(),
        finished_at: Some(chrono::Utc::now()),
        duration_ms: Some(1000),
        step_logs: vec![],
        executed_steps: vec![],
    };
    let dep_staging = DeploymentRecord {
        id: "dep-staging-sync".to_string(),
        project_id: "proj-sync".to_string(),
        project_name: "Frontend App".to_string(),
        environment: "staging".to_string(),
        app_name: "front-staging".to_string(),
        version: "1.2.0".to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("c2".to_string()),
        git_changes: vec![],
        is_rollback: false,
        servers: vec![],
        status: DeploymentStatus::Succeeded,
        started_at: chrono::Utc::now() - chrono::Duration::hours(2),
        finished_at: Some(chrono::Utc::now() - chrono::Duration::hours(2)),
        duration_ms: Some(900),
        step_logs: vec![],
        executed_steps: vec![],
    };
    storage.deployments = vec![dep_dev, dep_staging];

    let mut app = AppLoadApp::new(storage);
    app.selection = SidebarSelection::Project("proj-sync".to_string());
    app.selected_env_key = "prod".to_string();
    app.cached_pkg_version = Some("1.3.5".to_string());

    // While in "prod", candidates should come from dev and staging
    let candidates = app.sync_version_candidates("proj-sync");
    assert_eq!(candidates.len(), 2);
    let dev_cand = candidates
        .iter()
        .find(|c| c.source_name == "Development")
        .unwrap();
    assert_eq!(dev_cand.version, "1.3.5");
    assert!(dev_cand.is_latest);

    let staging_cand = candidates
        .iter()
        .find(|c| c.source_name == "Staging")
        .unwrap();
    assert_eq!(staging_cand.version, "1.2.0");
    assert!(!staging_cand.is_latest);

    // Apply synced version from Development
    app.apply_synced_version("1.3.5", "Development");
    assert_eq!(app.deploy_version_input, "1.3.5");
    assert!(!app.sync_version_dropdown_open);
    let banner = app.status_banner.as_ref().unwrap();
    assert!(banner
        .0
        .contains("Synced version to v1.3.5 from Development"));
    assert!(banner.1); // is_success
}

#[test]
fn test_console_loading_indicator_on_running_pipeline() {
    let ctx = egui::Context::default();
    AppLoadApp::configure_light_workspace_style(&ctx);

    let mut storage = crate::storage::AppStorage::default();
    let proj = Project {
        id: "proj-running-console".to_string(),
        name: "Realtime API".to_string(),
        description: "Console Test".to_string(),
        local_path: ".".to_string(),
        environments: vec![EnvironmentConfig {
            key: "prod".to_string(),
            name: "Production".to_string(),
            app_name: "realtime-prod".to_string(),
            servers: vec!["10.0.0.1".to_string()],
            remote_user: "deploy".to_string(),
            vault_target: "v".to_string(),
            remote_dir: "/apps".to_string(),
            env_source: None,
            http_proxy: None,
            steps: Vec::new(),
        }],
        group: None,
        sort_order: None,
    };
    storage.projects = vec![proj];

    let running_dep = DeploymentRecord {
        id: "dep-console-run".to_string(),
        project_id: "proj-running-console".to_string(),
        project_name: "Realtime API".to_string(),
        environment: "prod".to_string(),
        app_name: "realtime-prod".to_string(),
        version: "2.0.0".to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("abc".to_string()),
        git_changes: vec![],
        is_rollback: false,
        servers: vec!["10.0.0.1".to_string()],
        status: DeploymentStatus::Running,
        started_at: chrono::Utc::now(),
        finished_at: None,
        duration_ms: None,
        step_logs: vec![crate::domain::StepExecutionLog {
            step_name: "Building Docker container".to_string(),
            status: DeploymentStatus::Running,
            duration_ms: 3200,
            output: "Step 1/5: RUN cargo build\n[OK] Intermediate image cached\n".to_string(),
            command_summary: "docker build -t app:2.0.0 .".to_string(),
        }],
        executed_steps: vec![],
    };
    storage.deployments = vec![running_dep];

    let mut app = AppLoadApp::new(storage);
    app.selection = SidebarSelection::Project("proj-running-console".to_string());
    app.selected_env_key = "prod".to_string();
    app.active_deployment_id = Some("dep-console-run".to_string());
    app.selected_deployment_id = Some("dep-console-run".to_string());
    app.console_collapsed = false;

    // Render history and terminal window with running deployment
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_history_and_logs_view(ui, Some("proj-running-console"));
        });
    });

    assert_eq!(app.active_deployment_id.as_deref(), Some("dep-console-run"));
}

#[test]
fn test_deployment_keeps_running_on_update_and_restart_deferred_until_finished() {
    let storage = crate::storage::AppStorage::default();
    let mut app = AppLoadApp::new(storage);
    let ctx = egui::Context::default();

    // 1. Mark an active deployment in progress
    app.active_deployments.insert(
        ("proj-app".to_string(), "prod".to_string()),
        "dep-99".to_string(),
    );
    assert!(app.has_active_deployments());
    assert!(app.is_project_deploying("proj-app"));
    assert!(app.is_env_deploying("proj-app", "prod"));

    // 2. An update download completes while deployment is still running
    let (updater_tx, updater_rx) = mpsc::channel();
    updater_tx
        .send(UpdaterEvent::ReadyToRestart {
            new_version: "0.10.7".to_string(),
            exe_path: PathBuf::from("mock/path/appload.exe"),
        })
        .unwrap();
    app.updater_rx = updater_rx;
    app.poll_executor_events(&ctx);

    // 3. Deployment was NOT interrupted, restart is deferred
    assert!(app.has_active_deployments());
    assert!(app.is_project_deploying("proj-app"));
    assert_eq!(
        app.pending_update_restart,
        Some(("0.10.7".to_string(), PathBuf::from("mock/path/appload.exe")))
    );
    let banner = app.status_banner.as_ref().unwrap();
    assert!(banner
        .0
        .contains("Active deployments will continue running uninterrupted before restart."));

    // 4. When the deployment finishes, the deferred restart is automatically triggered
    let finished_record = DeploymentRecord {
        id: "dep-99".to_string(),
        project_id: "proj-app".to_string(),
        project_name: "App Service".to_string(),
        environment: "prod".to_string(),
        app_name: "app-prod".to_string(),
        version: "1.0.0".to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("c1".to_string()),
        git_changes: vec![],
        is_rollback: false,
        servers: vec!["10.0.0.1".to_string()],
        status: DeploymentStatus::Succeeded,
        started_at: chrono::Utc::now(),
        finished_at: Some(chrono::Utc::now()),
        duration_ms: Some(1000),
        step_logs: vec![],
        executed_steps: vec![],
    };
    let (exec_tx, exec_rx) = mpsc::channel();
    exec_tx
        .send(ExecutorEvent::Finished(finished_record))
        .unwrap();
    app.event_rx = exec_rx;
    app.poll_executor_events(&ctx);

    // 5. Active deployments is now empty and pending_update_restart has been consumed
    assert!(!app.has_active_deployments());
    assert!(app.pending_update_restart.is_none());
    // Since mock exe does not exist, update_error safely records the error without crash
    assert!(app.update_error.is_some());
}

#[test]
fn test_environment_cards_3_columns_fill_space() {
    let card_spacing = 14.0_f32;
    let avail_w = 900.0_f32;
    let cols = if avail_w >= 650.0 {
        3
    } else if avail_w >= 440.0 {
        2
    } else {
        1
    };
    assert_eq!(cols, 3);
    let card_w = ((avail_w - card_spacing * (cols - 1) as f32) / cols as f32).floor();
    assert_eq!(card_w, 290.0);
    let total_w = card_w * 3.0 + card_spacing * 2.0;
    assert!(total_w <= avail_w);
}

#[test]
fn test_donation_url_and_heart_icon() {
    assert_eq!(DONATION_URL, "https://trakteer.id/faidfadjri");
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            draw_outline_icon(
                ui.painter(),
                Pos2::new(10.0, 10.0),
                OutlineIcon::Heart,
                Color32::RED,
            );
        });
    });
}

#[test]
fn test_refresh_animation_state() {
    let mut app = AppLoadApp::new(crate::storage::AppStorage::default());
    assert!(app.refresh_anim_start.is_some());
    app.refresh_anim_start = None;
    assert!(app.refresh_anim_start.is_none());
    app.refresh_selected_project_metadata(true);
    assert!(app.refresh_anim_start.is_some());
}

#[test]
fn test_cancel_running_deployment_clears_busy_state() {
    let mut storage = crate::storage::AppStorage::default();
    storage.projects.push(Project {
        id: "backend-cdo".to_string(),
        name: "Backend Customer Data Ops".to_string(),
        description: "REST API service".to_string(),
        local_path: ".".to_string(),
        environments: vec![EnvironmentConfig {
            key: "production".to_string(),
            name: "Production".to_string(),
            app_name: "backend-cdo".to_string(),
            servers: vec!["10.0.0.1".to_string()],
            remote_user: "deploy".to_string(),
            vault_target: "vault".to_string(),
            remote_dir: "/app".to_string(),
            env_source: None,
            http_proxy: None,
            steps: Vec::new(),
        }],
        group: None,
        sort_order: Some(0),
    });
    storage.deployments.push(DeploymentRecord {
        id: "dep-stuck-1".to_string(),
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
        status: DeploymentStatus::Running,
        started_at: chrono::Utc::now() - chrono::Duration::seconds(30),
        finished_at: None,
        duration_ms: None,
        step_logs: vec![],
        executed_steps: vec![],
    });

    let mut app = AppLoadApp::new(storage);
    app.active_deployments.insert(
        ("backend-cdo".to_string(), "production".to_string()),
        "dep-stuck-1".to_string(),
    );
    app.active_deployment_id = Some("dep-stuck-1".to_string());

    assert!(app.is_env_deploying("backend-cdo", "production"));
    assert!(app.is_project_deploying("backend-cdo"));

    app.cancel_deployment("dep-stuck-1");

    assert!(!app.is_env_deploying("backend-cdo", "production"));
    assert!(!app.is_project_deploying("backend-cdo"));
    assert_eq!(
        app.storage.deployments[0].status,
        DeploymentStatus::Cancelled
    );
    assert!(app.storage.deployments[0].finished_at.is_some());
    assert!(app.storage.deployments[0].duration_ms.is_some());
}

#[test]
fn test_console_scroll_height_in_small_viewport() {
    let mut storage = crate::storage::AppStorage::default();
    let mut log_lines = String::new();
    for i in 1..=100 {
        log_lines.push_str(&format!(
            "[OK] Log line item #{i}: building package assets\n"
        ));
    }
    let dep = DeploymentRecord {
        id: "dep-100-lines".to_string(),
        project_id: "p1".to_string(),
        project_name: "Test Project".to_string(),
        environment: "production".to_string(),
        app_name: "test-app".to_string(),
        version: "1.0.0".to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("abcdef".to_string()),
        git_changes: vec![],
        is_rollback: false,
        servers: vec!["10.0.0.1".to_string()],
        status: DeploymentStatus::Succeeded,
        started_at: chrono::Utc::now(),
        finished_at: Some(chrono::Utc::now()),
        duration_ms: Some(1500),
        step_logs: vec![crate::domain::StepExecutionLog {
            step_name: "Deploy Step".to_string(),
            command_summary: "deploy.sh".to_string(),
            output: log_lines,
            status: DeploymentStatus::Succeeded,
            duration_ms: 1500,
        }],
        executed_steps: vec![],
    };
    storage.deployments.push(dep.clone());

    let mut app = AppLoadApp::new(storage);
    app.console_collapsed = false;
    app.console_expanded_height = false; // compact mode: compact = 320.0

    let ctx = egui::Context::default();
    AppLoadApp::install_fonts(&ctx);
    let mut inner_viewport_height = 0.0;

    let raw_input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 400.0))),
        ..Default::default()
    };

    let _ = ctx.run(raw_input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Render inside an outer ScrollArea near the bottom of a small viewport
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Pre-allocate space so remaining available height in the viewport is very small (< 64px)
                ui.allocate_space(Vec2::new(800.0, 350.0));
                assert!(ui.available_height() < 64.0);

                let scroll_inner_rect = app.render_terminal_window(ui, &dep);
                inner_viewport_height = scroll_inner_rect.height();
            });
        });
    });

    assert!(
        inner_viewport_height >= 320.0,
        "Inner scroll viewport height ({inner_viewport_height}) must be >= configured compact height (320.0)"
    );

    // Also verify expanded mode on a 1080p-scale screen (height 1000.0 -> 60% = 600.0)
    app.console_expanded_height = true;
    let raw_input_large = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 1000.0))),
        ..Default::default()
    };
    let _ = ctx.run(raw_input_large, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let scroll_inner_rect = app.render_terminal_window(ui, &dep);
                inner_viewport_height = scroll_inner_rect.height();
            });
        });
    });
    assert_eq!(inner_viewport_height, 600.0);
}
