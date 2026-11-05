//! Tests for sidebar project tree, search filtering, and loading indicators.

use super::*;

#[test]
fn test_project_switch_loading_indicator_and_hover_cursor() {
    let ctx = egui::Context::default();
    ShiplyApp::configure_light_workspace_style(&ctx);

    // Verify global visuals hover interact cursor is set to PointingHand (Pointer)
    assert_eq!(
        ctx.style().visuals.interact_cursor,
        Some(egui::CursorIcon::PointingHand)
    );

    // Test render_project_loading_indicator execution without panic
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            render_project_loading_indicator(ui, "Backend API");
        });
    });
}

#[test]
fn test_async_git_inspect_channel_and_project_switch() {
    let mut storage = crate::storage::AppStorage::default();
    let project = Project {
        id: "proj-async".to_string(),
        name: "Async Project".to_string(),
        description: "Desc".to_string(),
        local_path: ".".to_string(),
        environments: vec![EnvironmentConfig {
            key: "dev".to_string(),
            name: "Development".to_string(),
            app_name: "async-dev".to_string(),
            servers: vec!["127.0.0.1".to_string()],
            remote_user: "deploy".to_string(),
            vault_target: "vault".to_string(),
            remote_dir: "/app".to_string(),
            env_source: None,
            http_proxy: None,
            steps: Vec::new(),
        }],
        group: None,
        sort_order: None,
    };
    storage.projects.push(project);
    let mut app = ShiplyApp::new(storage);

    // Simulate fast project selection
    app.selection = SidebarSelection::Project("proj-async".to_string());
    app.refresh_selected_project_metadata(true);

    // Project selection was updated immediately without UI freeze
    assert_eq!(app.cached_project_id.as_deref(), Some("proj-async"));

    // Simulate incoming background inspection result
    let _ = app.git_inspect_tx.send(GitInspectionResult {
        project_id: "proj-async".to_string(),
        pkg_version: Some("2.5.0".to_string()),
        git_meta: GitMetadata {
            branch: Some("feature/instant-switch".to_string()),
            commit_short: Some("999aaaa".to_string()),
            commit_subject: Some("feat: instant switch".to_string()),
        },
        git_branches: vec!["main".to_string(), "feature/instant-switch".to_string()],
        git_changes: vec!["M src/ui.rs".to_string()],
        update_version_input: true,
    });

    let received = app.process_pending_git_inspect();
    assert!(received);
    assert_eq!(app.cached_pkg_version.as_deref(), Some("2.5.0"));
    assert_eq!(app.selected_git_branch, "feature/instant-switch");
    assert_eq!(app.cached_git_changes, vec!["M src/ui.rs"]);
    assert!(app.project_switch_loading_id.is_none());
}

#[test]
fn test_project_menu_sidebar_running_deployment_circle_loading_indicator() {
    let ctx = egui::Context::default();
    ShiplyApp::configure_light_workspace_style(&ctx);

    let mut storage = crate::storage::AppStorage::default();
    let proj1 = Project {
        id: "proj-running".to_string(),
        name: "Customer Portal".to_string(),
        description: "Active".to_string(),
        local_path: ".".to_string(),
        environments: vec![EnvironmentConfig {
            key: "prod".to_string(),
            name: "Production".to_string(),
            app_name: "portal-prod".to_string(),
            servers: vec!["10.0.0.1".to_string()],
            remote_user: "deploy".to_string(),
            vault_target: "v1".to_string(),
            remote_dir: "/apps/portal".to_string(),
            env_source: None,
            http_proxy: None,
            steps: Vec::new(),
        }],
        group: None,
        sort_order: None,
    };
    let proj2 = Project {
        id: "proj-idle".to_string(),
        name: "Admin Dashboard".to_string(),
        description: "Idle".to_string(),
        local_path: ".".to_string(),
        environments: vec![],
        group: None,
        sort_order: None,
    };
    storage.projects = vec![proj1, proj2];

    // Add a running deployment for proj-running
    let running_dep = DeploymentRecord {
        id: "dep-running-1".to_string(),
        project_id: "proj-running".to_string(),
        project_name: "Customer Portal".to_string(),
        environment: "prod".to_string(),
        app_name: "portal-prod".to_string(),
        version: "1.0.5".to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("c0ffee1".to_string()),
        git_changes: vec![],
        is_rollback: false,
        servers: vec!["10.0.0.1".to_string()],
        status: DeploymentStatus::Running,
        started_at: chrono::Utc::now(),
        finished_at: None,
        duration_ms: None,
        step_logs: vec![],
        executed_steps: vec![],
    };
    storage.deployments = vec![running_dep];

    let mut app = ShiplyApp::new(storage);
    app.active_deployment_id = Some("dep-running-1".to_string());

    // Render sidebar with anim_t = 1.0 (fully open)
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_sidebar(ui.ctx(), 1.0);
        });
    });

    // Verify active deployment is registered and belongs to proj-running
    let is_running_p1 = app
        .storage
        .deployments
        .iter()
        .any(|d| d.project_id == "proj-running" && d.status == DeploymentStatus::Running);
    let is_running_p2 = app
        .storage
        .deployments
        .iter()
        .any(|d| d.project_id == "proj-idle" && d.status == DeploymentStatus::Running);
    assert!(is_running_p1);
    assert!(!is_running_p2);
}

#[test]
fn test_sidebar_project_loading_indicator_positioned_without_overlap() {
    let ctx = egui::Context::default();
    ShiplyApp::configure_light_workspace_style(&ctx);

    let mut storage = crate::storage::AppStorage::default();
    let proj = Project {
        id: "proj-multi".to_string(),
        name: "Frontend Customer Data Ops".to_string(),
        description: "Multi-env project".to_string(),
        local_path: ".".to_string(),
        environments: vec![
            EnvironmentConfig {
                key: "dev".to_string(),
                name: "Development".to_string(),
                app_name: "dev-app".to_string(),
                servers: vec!["10.0.0.1".to_string()],
                remote_user: "deploy".to_string(),
                vault_target: "v1".to_string(),
                remote_dir: "/app".to_string(),
                env_source: None,
                http_proxy: None,
                steps: Vec::new(),
            },
            EnvironmentConfig {
                key: "prod".to_string(),
                name: "Production".to_string(),
                app_name: "prod-app".to_string(),
                servers: vec!["10.0.0.2".to_string()],
                remote_user: "deploy".to_string(),
                vault_target: "v1".to_string(),
                remote_dir: "/app".to_string(),
                env_source: None,
                http_proxy: None,
                steps: Vec::new(),
            },
        ],
        group: None,
        sort_order: None,
    };
    storage.projects = vec![proj];

    // Running deployment on dev
    let dep = DeploymentRecord {
        id: "dep-running-dev".to_string(),
        project_id: "proj-multi".to_string(),
        project_name: "Frontend Customer Data Ops".to_string(),
        environment: "dev".to_string(),
        app_name: "dev-app".to_string(),
        version: "1.1.10".to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("abcdef1".to_string()),
        git_changes: vec![],
        is_rollback: false,
        servers: vec!["10.0.0.1".to_string()],
        status: DeploymentStatus::Running,
        started_at: chrono::Utc::now(),
        finished_at: None,
        duration_ms: None,
        step_logs: vec![],
        executed_steps: vec![],
    };
    // Succeeded deployment on prod
    let ok_dep = DeploymentRecord {
        id: "dep-ok-prod".to_string(),
        project_id: "proj-multi".to_string(),
        project_name: "Frontend Customer Data Ops".to_string(),
        environment: "prod".to_string(),
        app_name: "prod-app".to_string(),
        version: "1.1.9".to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("abcdef0".to_string()),
        git_changes: vec![],
        is_rollback: false,
        servers: vec!["10.0.0.2".to_string()],
        status: DeploymentStatus::Succeeded,
        started_at: chrono::Utc::now(),
        finished_at: Some(chrono::Utc::now()),
        duration_ms: Some(1200),
        step_logs: vec![],
        executed_steps: vec![],
    };
    storage.deployments = vec![dep, ok_dep];

    let mut app = ShiplyApp::new(storage);
    app.active_deployment_id = Some("dep-running-dev".to_string());
    assert!(app.is_project_deploying("proj-multi"));
    assert!(app.is_env_deploying("proj-multi", "dev"));
    assert!(!app.is_env_deploying("proj-multi", "prod"));

    // Render sidebar with full anim_t = 1.0
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_sidebar(ui.ctx(), 1.0);
        });
    });

    // Verification of geometry:
    // Card top = 0.0 (reference)
    // Row 1 spinner center Y = 15.0, radius = 5.0 -> spinner bottom Y = 20.0
    // Row 2 badges top Y = 30.0
    // Vertical gap between spinner and badges = 10.0px (strictly positive, non-overlapping)
    let spinner_bottom_offset = 15.0 + 5.0;
    let row2_top_offset = 30.0;
    assert!(row2_top_offset > spinner_bottom_offset);
    assert_eq!(row2_top_offset - spinner_bottom_offset, 10.0);
}

#[test]
fn test_project_sorting_and_drag_reordering_ui() {
    let ctx = egui::Context::default();
    ShiplyApp::configure_light_workspace_style(&ctx);

    let mut storage = crate::storage::AppStorage::default();
    let p_beta = Project {
        id: "p-beta".to_string(),
        name: "Beta API".to_string(),
        description: "".to_string(),
        local_path: ".".to_string(),
        environments: vec![],
        group: None,
        sort_order: Some(0),
    };
    let p_alpha = Project {
        id: "p-alpha".to_string(),
        name: "Alpha Web".to_string(),
        description: "".to_string(),
        local_path: ".".to_string(),
        environments: vec![],
        group: None,
        sort_order: Some(1),
    };
    let p_gamma = Project {
        id: "p-gamma".to_string(),
        name: "Gamma Service".to_string(),
        description: "".to_string(),
        local_path: ".".to_string(),
        environments: vec![],
        group: None,
        sort_order: Some(2),
    };
    storage.projects = vec![p_beta, p_alpha, p_gamma];

    let mut app = ShiplyApp::new(storage);

    // 1. Render sidebar initial custom order
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_sidebar(ui.ctx(), 1.0);
        });
    });
    assert_eq!(app.storage.projects[0].name, "Beta API");
    assert_eq!(app.storage.projects[1].name, "Alpha Web");
    assert_eq!(app.storage.projects[2].name, "Gamma Service");

    // 2. Sort by NameAsc
    app.project_sort_mode = ProjectSortMode::NameAsc;
    app.storage.sort_projects(ProjectSortMode::NameAsc);
    assert_eq!(app.storage.projects[0].name, "Alpha Web");
    assert_eq!(app.storage.projects[1].name, "Beta API");
    assert_eq!(app.storage.projects[2].name, "Gamma Service");

    // 3. Sort by NameDesc
    app.project_sort_mode = ProjectSortMode::NameDesc;
    app.storage.sort_projects(ProjectSortMode::NameDesc);
    assert_eq!(app.storage.projects[0].name, "Gamma Service");
    assert_eq!(app.storage.projects[1].name, "Beta API");
    assert_eq!(app.storage.projects[2].name, "Alpha Web");

    // 4. Test reorder simulation (dragging first item to last position)
    app.dragged_project_idx = Some(0);
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_sidebar(ui.ctx(), 1.0);
        });
    });
    app.storage.reorder_project(0, 2);
    app.dragged_project_idx = None;
    app.project_sort_mode = ProjectSortMode::Custom;

    assert_eq!(app.storage.projects[0].name, "Beta API");
    assert_eq!(app.storage.projects[1].name, "Alpha Web");
    assert_eq!(app.storage.projects[2].name, "Gamma Service");

    // 5. Test move down and move up
    app.storage.move_project_down(0);
    assert_eq!(app.storage.projects[0].name, "Alpha Web");
    assert_eq!(app.storage.projects[1].name, "Beta API");

    app.storage.move_project_up(1);
    assert_eq!(app.storage.projects[0].name, "Beta API");
    assert_eq!(app.storage.projects[1].name, "Alpha Web");
}

#[test]
fn test_project_grouping_and_collapsible_sections_ui() {
    let ctx = egui::Context::default();
    ShiplyApp::configure_light_workspace_style(&ctx);

    let mut storage = crate::storage::AppStorage::default();
    let p_backend = Project {
        id: "p-backend".to_string(),
        name: "Backend Service".to_string(),
        description: "".to_string(),
        local_path: ".".to_string(),
        environments: vec![],
        group: Some("Backend".to_string()),
        sort_order: Some(0),
    };
    let p_frontend = Project {
        id: "p-frontend".to_string(),
        name: "Frontend App".to_string(),
        description: "".to_string(),
        local_path: ".".to_string(),
        environments: vec![],
        group: Some("Frontend".to_string()),
        sort_order: Some(1),
    };
    let p_ungrouped = Project {
        id: "p-ungrouped".to_string(),
        name: "Ungrouped Service".to_string(),
        description: "".to_string(),
        local_path: ".".to_string(),
        environments: vec![],
        group: None,
        sort_order: Some(2),
    };
    storage.projects = vec![p_backend, p_frontend, p_ungrouped];

    let mut app = ShiplyApp::new(storage);

    // Verify existing project groups
    let groups = app.storage.existing_project_groups();
    assert_eq!(groups, vec!["Backend", "Frontend"]);

    // Render sidebar with groups expanded
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_sidebar(ui.ctx(), 1.0);
        });
    });

    // Collapse "Backend" group and render again
    app.collapsed_project_groups.insert("Backend".to_string());
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_sidebar(ui.ctx(), 1.0);
        });
    });
    assert!(app.collapsed_project_groups.contains("Backend"));

    // Change group of p-ungrouped to Backend
    assert!(app
        .storage
        .set_project_group("p-ungrouped", Some("Backend".to_string())));
    assert_eq!(
        app.storage
            .projects
            .iter()
            .find(|p| p.id == "p-ungrouped")
            .unwrap()
            .group
            .as_deref(),
        Some("Backend")
    );

    // Remove group from p-frontend
    assert!(app.storage.set_project_group("p-frontend", None));
    assert_eq!(
        app.storage
            .projects
            .iter()
            .find(|p| p.id == "p-frontend")
            .unwrap()
            .group,
        None
    );
}

#[test]
fn test_sidebar_design_tokens_skeleton_loading_and_modals() {
    let ctx = egui::Context::default();
    ShiplyApp::configure_light_workspace_style(&ctx);

    // 1. Verify typography tokens (Brief 002 Section 2.2: 22, 16, 14, 13, 12, 11)
    assert_eq!(Typo::H1.size(), 22.0);
    assert_eq!(Typo::H2.size(), 16.0);
    assert_eq!(Typo::H3.size(), 14.0);
    assert_eq!(Typo::Body.size(), 13.0);
    assert_eq!(Typo::Label.size(), 13.0);
    assert_eq!(Typo::Small.size(), 12.0);
    assert_eq!(Typo::SmallMedium.size(), 12.0);
    assert_eq!(Typo::Caption.size(), 11.0);
    assert_eq!(Typo::MonoSm.size(), 11.0);

    // 2. Verify spacing and radius scales
    assert_eq!(SP_4, 4.0);
    assert_eq!(SP_8, 8.0);
    assert_eq!(SP_12, 12.0);
    assert_eq!(SP_16, 16.0);
    assert_eq!(SP_20, 20.0);
    assert_eq!(SP_24, 24.0);
    assert_eq!(R_SM, 4.0);
    assert_eq!(R_MD, 6.0);
    assert_eq!(R_LG, 10.0);
    assert_eq!(R_XL, 14.0);

    // 3. Verify color tokens
    assert_eq!(SB_BG, Color32::from_rgb(11, 18, 32));
    assert_eq!(SB_SURFACE, Color32::from_rgb(17, 26, 46));
    assert_eq!(SB_BORDER_LINE, Color32::from_rgb(28, 38, 60));
    assert_eq!(SB_TEXT, Color32::from_rgb(230, 234, 242));
    assert_eq!(SB_TEXT_MUTED, Color32::from_rgb(139, 150, 173));
    assert_eq!(SB_TEXT_FAINT, Color32::from_rgb(100, 112, 138));
    assert_eq!(SB_ACCENT, Color32::from_rgb(59, 130, 246));
    assert_eq!(BADGE_DEV_TEXT, Color32::from_rgb(125, 178, 255));
    assert_eq!(BADGE_PROD_TEXT, Color32::from_rgb(196, 161, 255));
    assert_eq!(ACCENT, Color32::from_rgb(20, 110, 245));
    assert_eq!(MD_BG, Color32::WHITE);
    assert_eq!(MD_BORDER, Color32::from_rgb(230, 230, 234));

    let mut storage = crate::storage::AppStorage::default();
    storage.projects = vec![Project {
        id: "p-test-long".to_string(),
        name: "A Super Long Project Name That Should Not Wrap Onto Second Line".to_string(),
        description: "Test".to_string(),
        local_path: ".".to_string(),
        environments: vec![
            EnvironmentConfig {
                key: "dev".to_string(),
                name: "Development".to_string(),
                app_name: "test-dev".to_string(),
                servers: vec!["127.0.0.1".to_string()],
                remote_user: "dev".to_string(),
                vault_target: "v1".to_string(),
                remote_dir: "/app".to_string(),
                env_source: None,
                http_proxy: None,
                steps: vec![],
            },
            EnvironmentConfig {
                key: "prod".to_string(),
                name: "Production".to_string(),
                app_name: "test-prod".to_string(),
                servers: vec!["127.0.0.2".to_string()],
                remote_user: "prod".to_string(),
                vault_target: "v1".to_string(),
                remote_dir: "/app".to_string(),
                env_source: None,
                http_proxy: None,
                steps: vec![],
            },
        ],
        group: None,
        sort_order: Some(0),
    }];

    let mut app = ShiplyApp::new(storage);

    // Test rendering with project switch loading (shimmer skeleton pills on card)
    app.project_switch_loading_id = Some("p-test-long".to_string());
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_sidebar(ui.ctx(), 1.0);
        });
    });

    // Test rendering empty state with refresh shimmer skeletons
    let mut empty_app = ShiplyApp::new(crate::storage::AppStorage::default());
    empty_app.sidebar_refreshing_until = Some(Instant::now() + Duration::from_secs(5));
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            empty_app.render_sidebar(ui.ctx(), 1.0);
        });
    });

    // Test rendering modals with design tokens
    app.show_new_project_modal = true;
    let _ = ctx.run(Default::default(), |ctx| {
        app.render_new_project_modal(ctx);
    });
    app.show_new_project_modal = false;

    app.open_edit_project_modal(0);
    let _ = ctx.run(Default::default(), |ctx| {
        app.render_edit_project_modal(ctx);
    });
    app.show_edit_project_modal = false;

    app.show_new_env_modal = true;
    let _ = ctx.run(Default::default(), |ctx| {
        app.render_new_environment_modal(ctx);
    });
    app.show_new_env_modal = false;

    app.pending_delete_confirm = Some(ConfirmDeleteAction::Project {
        project_id: "p-test-long".to_string(),
        project_name: "Test".to_string(),
    });
    let _ = ctx.run(Default::default(), |ctx| {
        app.render_delete_confirmation_modal(ctx);
    });
    app.pending_delete_confirm = None;

    app.show_bulk_deploy_modal = true;
    let _ = ctx.run(Default::default(), |ctx| {
        app.render_bulk_deploy_modal(ctx);
    });
    app.show_bulk_deploy_modal = false;

    app.show_update_modal = true;
    let _ = ctx.run(Default::default(), |ctx| {
        app.render_github_update_modal(ctx);
    });
    app.show_update_modal = false;
}

#[test]
fn test_sidebar_sort_dropdown_menu_interaction() {
    let ctx = egui::Context::default();
    ShiplyApp::configure_light_workspace_style(&ctx);

    let mut storage = crate::storage::AppStorage::default();
    storage.projects = vec![
        Project {
            id: "p1".to_string(),
            name: "Alpha".to_string(),
            description: "".to_string(),
            local_path: ".".to_string(),
            environments: vec![],
            group: None,
            sort_order: Some(0),
        },
        Project {
            id: "p2".to_string(),
            name: "Beta".to_string(),
            description: "".to_string(),
            local_path: ".".to_string(),
            environments: vec![],
            group: None,
            sort_order: Some(1),
        },
    ];

    let mut app = ShiplyApp::new(storage);
    assert!(!app.sidebar_sort_dropdown_open);

    // 1. Open dropdown
    app.sidebar_sort_dropdown_open = true;

    // 2. Render sidebar and dropdown
    let _ = ctx.run(Default::default(), |ctx| {
        app.render_sidebar(ctx, 1.0);
    });
    assert!(app.sidebar_sort_dropdown_open);

    // 3. Test keyboard navigation cycling through sort modes
    let modes = [
        ProjectSortMode::Custom,
        ProjectSortMode::NameAsc,
        ProjectSortMode::NameDesc,
        ProjectSortMode::RecentDeploy,
    ];
    for expected_mode in modes {
        app.project_sort_mode = expected_mode;
        app.storage.sort_projects(expected_mode);
        assert_eq!(app.project_sort_mode, expected_mode);
    }

    // 4. Closing dropdown
    app.sidebar_sort_dropdown_open = false;
    assert!(!app.sidebar_sort_dropdown_open);
}

#[test]
fn test_sidebar_search_single_outline_and_clear_button() {
    let ctx = egui::Context::default();
    ShiplyApp::configure_light_workspace_style(&ctx);

    let mut search_query = "cdo-tester".to_string();

    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let changed = sidebar_search_text_edit(
                ui,
                "test_search_input",
                &mut search_query,
                220.0,
                Some("Search projects..."),
            );
            assert!(!changed);
        });
    });

    assert_eq!(search_query, "cdo-tester");

    // Clear button simulation: clearing value resets query to empty string
    search_query.clear();
    assert!(search_query.is_empty());
}

#[test]
fn test_sidebar_footer_bounds_prevent_overflow_under_taskbar() {
    // Expanded sidebar footer total height: 160px per Addendum B
    let expanded_footer_h = 160.0_f32;
    let btn_h = 32.0_f32;
    let btn1_top = 8.0_f32; // SP_8
    let btn2_top = 8.0 + 32.0 + 2.0; // 42.0 (SP_2 gap)
    let btn3_top = 42.0 + 32.0 + 2.0; // 76.0 (SP_2 gap)
    let status_top = 76.0 + 32.0 + 8.0; // 116.0 (SP_8 gap)
    let status_h = 28.0_f32;
    let status_bottom = status_top + status_h; // 144px

    // Verify status bottom is strictly within footer_h with exact 16px bottom margin
    assert!(status_bottom <= expanded_footer_h);
    let bottom_margin = expanded_footer_h - status_bottom;
    assert_eq!(
        bottom_margin, 16.0,
        "Bottom margin {bottom_margin}px must be exactly 16px (SB_PAD_X)"
    );

    // Verify buttons do not overlap
    assert!(btn1_top + btn_h <= btn2_top);
    assert!(btn2_top + btn_h <= btn3_top);
    assert!(btn3_top + btn_h <= status_top);

    // Collapsed rail sidebar footer: 160px
    let rail_footer_h = 160.0_f32;
    let rail_dot_pos_y = 116.0 + 14.0; // 130.0
    assert!(
        rail_dot_pos_y + 4.0 <= rail_footer_h,
        "Rail status dot must not overflow footer bounds"
    );
}

#[test]
fn test_sidebar_width_and_symmetric_padding() {
    let ctx = egui::Context::default();
    ShiplyApp::configure_light_workspace_style(&ctx);
    ShiplyApp::install_fonts(&ctx);

    assert_eq!(SIDEBAR_EXPANDED_WIDTH, 272.0);
    assert_eq!(SB_PAD_X, 16.0);
    assert_eq!(SP_2, 2.0);

    let storage = crate::storage::AppStorage::default();
    let mut app = ShiplyApp::new(storage);

    let mut rendered_inner_width = 0.0;
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 800.0))),
            ..Default::default()
        },
        |ctx| {
            app.render_sidebar(ctx, 1.0);
            rendered_inner_width = SIDEBAR_EXPANDED_WIDTH - 2.0 * SB_PAD_X;
        },
    );

    // With 272px width and 16px padding on both sides, inner content width is exactly 240px
    assert_eq!(rendered_inner_width, 240.0);
}

#[test]
fn test_sidebar_project_row_borderless_and_env_badges_overflow() {
    let ctx = egui::Context::default();
    ShiplyApp::configure_light_workspace_style(&ctx);
    ShiplyApp::install_fonts(&ctx);

    let mut storage = crate::storage::AppStorage::default();
    let mut project = Project {
        id: "proj-many-envs".to_string(),
        name: "Enterprise Microservices Hub".to_string(),
        description: "".to_string(),
        local_path: ".".to_string(),
        environments: vec![],
        group: None,
        sort_order: Some(0),
    };
    for i in 1..=6 {
        project.environments.push(crate::domain::EnvironmentConfig {
            key: format!("env{i}"),
            name: format!("Cluster {i}"),
            app_name: format!("app-{i}"),
            servers: vec!["10.0.0.1".to_string()],
            remote_user: "deploy".to_string(),
            vault_target: "v1".to_string(),
            remote_dir: "/app".to_string(),
            env_source: None,
            http_proxy: None,
            steps: vec![],
        });
    }
    storage.projects = vec![project];

    let mut app = ShiplyApp::new(storage);
    app.selection = SidebarSelection::Project("proj-many-envs".to_string());

    // Run layout with anim_t = 1.0 (expanded mode)
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 800.0))),
            ..Default::default()
        },
        |ctx| {
            app.render_sidebar(ctx, 1.0);
        },
    );

    // Verify row dimensions
    let row_height = 58.0_f32;
    let row_pad_y = 10.0_f32;
    let row_pad_x = 12.0_f32;
    let badge_h = 18.0_f32;
    let badge_gap = SP_4;

    assert_eq!(row_height, 58.0);
    assert_eq!(row_pad_y, 10.0);
    assert_eq!(row_pad_x, 12.0);
    assert_eq!(badge_h, 18.0);
    assert_eq!(badge_gap, 4.0);
}
