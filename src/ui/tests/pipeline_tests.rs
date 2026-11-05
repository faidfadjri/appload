//! Tests for pipeline syntax highlighting, step reordering, and settings input buffering.

use super::*;

#[test]
fn test_pipeline_syntax_highlight_layout_job() {
    let cmd = "cd {remote_dir} && pm2 restart {app_name} --update-env ./dist/server.js";
    let job = build_pipeline_syntax_layout_job(cmd);
    assert_eq!(job.text, cmd);
    assert!(job.sections.len() >= 7);

    let has_template_purple = job
        .sections
        .iter()
        .any(|s| s.format.color == PURPLE_PILL_TEXT);
    let has_keyword_blue = job
        .sections
        .iter()
        .any(|s| s.format.color == BLUE_PILL_TEXT);
    let has_flag_amber = job
        .sections
        .iter()
        .any(|s| s.format.color == WARN_PILL_TEXT);
    let has_path_teal = job
        .sections
        .iter()
        .any(|s| s.format.color == TEAL_PILL_TEXT);
    let has_operator_crimson = job
        .sections
        .iter()
        .any(|s| s.format.color == FAIL_PILL_TEXT);

    assert!(has_template_purple);
    assert!(has_keyword_blue);
    assert!(has_flag_amber);
    assert!(has_path_teal);
    assert!(has_operator_crimson);
}

#[test]
fn test_server_settings_input_buffering_and_debounced_save() {
    let ctx = egui::Context::default();
    AppLoadApp::configure_light_workspace_style(&ctx);

    let mut storage = crate::storage::AppStorage::default();
    let env = EnvironmentConfig {
        key: "dev".to_string(),
        name: "Development".to_string(),
        app_name: "api-dev".to_string(),
        servers: vec!["10.0.0.1".to_string()],
        remote_user: "deploy".to_string(),
        vault_target: "dev_vault".to_string(),
        remote_dir: "/apps/api".to_string(),
        env_source: None,
        http_proxy: None,
        steps: vec![],
    };
    let project = Project {
        id: "p1".to_string(),
        name: "Test Project".to_string(),
        description: "Desc".to_string(),
        local_path: ".".to_string(),
        environments: vec![env],
        group: None,
        sort_order: None,
    };
    storage.projects.push(project);

    let mut app = AppLoadApp::new(storage);
    app.selected_env_key = "dev".to_string();

    assert!(!app.dirty_save_pending);
    assert!(app.servers_edit_buffer.is_none());

    // Render Server Settings Tab without panic
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_server_settings_tab(ui, 0);
        });
    });

    // The buffer should now be initialized with the joined server string
    assert_eq!(
        app.servers_edit_buffer,
        Some(("dev".to_string(), "10.0.0.1".to_string()))
    );

    // Simulate typing and mark dirty
    app.mark_config_dirty();
    assert!(app.dirty_save_pending);
    assert!(app.last_edit_time.is_some());

    // Flush pending save
    app.flush_pending_save();
    assert!(!app.dirty_save_pending);
    assert!(app.last_edit_time.is_none());
}

#[test]
fn test_draggable_pipeline_step_reorder_ui() {
    let ctx = egui::Context::default();
    AppLoadApp::configure_light_workspace_style(&ctx);

    let mut storage = crate::storage::AppStorage::default();
    let env = EnvironmentConfig {
        key: "dev".to_string(),
        name: "Development".to_string(),
        app_name: "api-dev".to_string(),
        servers: vec!["10.0.0.1".to_string()],
        remote_user: "deploy".to_string(),
        vault_target: "dev_vault".to_string(),
        remote_dir: "/apps/api".to_string(),
        env_source: None,
        http_proxy: None,
        steps: vec![
            PipelineStep::new(
                "Build",
                StepKind::LocalCommand {
                    command: "cargo build".to_string(),
                    allow_failure: false,
                    shell: None,
                },
            ),
            PipelineStep::new(
                "Verify",
                StepKind::VerifyLocalPath {
                    path: "./target".to_string(),
                },
            ),
            PipelineStep::new(
                "Upload",
                StepKind::TransferFiles {
                    sources: vec!["./target".to_string()],
                    stage_env_file: true,
                },
            ),
        ],
    };
    let project = Project {
        id: "p1".to_string(),
        name: "Test Project".to_string(),
        description: "Desc".to_string(),
        local_path: ".".to_string(),
        environments: vec![env],
        group: None,
        sort_order: None,
    };
    storage.projects.push(project);

    let mut app = AppLoadApp::new(storage);
    app.selected_env_key = "dev".to_string();

    // Render pipeline steps tab initially
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_pipeline_steps_tab(ui, 0);
        });
    });

    assert_eq!(
        app.storage.projects[0].environments[0].steps[0].name,
        "Build"
    );
    assert_eq!(
        app.storage.projects[0].environments[0].steps[1].name,
        "Verify"
    );
    assert_eq!(
        app.storage.projects[0].environments[0].steps[2].name,
        "Upload"
    );

    // Simulate dragging step 2 (Upload)
    app.dragged_step_idx = Some(2);
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_pipeline_steps_tab(ui, 0);
        });
    });

    // Test reorder execution
    app.storage.projects[0].environments[0].reorder_step(2, 0);
    app.dragged_step_idx = None;
    app.mark_config_dirty();

    assert_eq!(
        app.storage.projects[0].environments[0].steps[0].name,
        "Upload"
    );
    assert_eq!(
        app.storage.projects[0].environments[0].steps[1].name,
        "Build"
    );
    assert_eq!(
        app.storage.projects[0].environments[0].steps[2].name,
        "Verify"
    );
    assert!(app.dirty_save_pending);
}

#[test]
fn test_pipeline_variables_chips_match_interpolation_support() {
    use crate::domain::EnvironmentConfig;
    use crate::ui::views::pipeline_header::PIPELINE_VARIABLES;

    let env = EnvironmentConfig {
        key: "dev".to_string(),
        name: "Development".to_string(),
        servers: vec!["10.0.0.1".to_string()],
        remote_dir: "/opt/app".to_string(),
        remote_user: "deploy".to_string(),
        app_name: "my-service".to_string(),
        http_proxy: Some("http://proxy:8080".to_string()),
        env_source: Some("./.env.dev".to_string()),
        vault_target: String::new(),
        steps: Vec::new(),
    };

    // Assert there are exactly 8 canonical variables
    assert_eq!(PIPELINE_VARIABLES.len(), 8);

    // Verify all non-secret variables are substituted by interpolate
    let non_secret_vars: Vec<&str> = PIPELINE_VARIABLES
        .iter()
        .map(|(k, _)| *k)
        .filter(|k| !k.starts_with("${{"))
        .collect();
    assert_eq!(non_secret_vars.len(), 6);

    let template = non_secret_vars.join(" ");
    let interpolated = env.interpolate(&template, "1.2.3");

    // Assert no `{...}` remains
    assert!(
        !interpolated.contains('{'),
        "Unsubstituted variable found: {interpolated}"
    );
    assert!(
        !interpolated.contains('}'),
        "Unsubstituted variable found: {interpolated}"
    );
    assert!(interpolated.contains("/opt/app"));
    assert!(interpolated.contains("deploy"));
    assert!(interpolated.contains("my-service"));
    assert!(interpolated.contains("http://proxy:8080"));
    assert!(interpolated.contains("./.env.dev"));
    assert!(interpolated.contains("1.2.3"));

    // Verify secret variables follow ${{ secrets.KEY }} and ${{ secrets.KEY.username }} pattern
    let secret_vars: Vec<&str> = PIPELINE_VARIABLES
        .iter()
        .map(|(k, _)| *k)
        .filter(|k| k.starts_with("${{"))
        .collect();
    assert_eq!(
        secret_vars,
        vec!["${{ secrets.KEY }}", "${{ secrets.KEY.username }}"]
    );
}
