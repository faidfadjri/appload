use super::*;

#[test]
fn test_concurrent_deployments_across_different_projects_prevent_same_project_only() {
    let ctx = egui::Context::default();
    ShiplyApp::configure_light_workspace_style(&ctx);

    let mut storage = crate::storage::AppStorage::default();
    let proj_backend = Project {
        id: "proj-backend".to_string(),
        name: "Backend Service".to_string(),
        description: "API".to_string(),
        local_path: ".".to_string(),
        environments: vec![EnvironmentConfig {
            key: "prod".to_string(),
            name: "Production".to_string(),
            app_name: "backend-prod".to_string(),
            servers: vec!["10.0.0.1".to_string()],
            remote_user: "deploy".to_string(),
            vault_target: "v1".to_string(),
            remote_dir: "/apps/backend".to_string(),
            env_source: None,
            http_proxy: None,
            steps: Vec::new(),
        }],
        group: None,
        sort_order: None,
    };
    let proj_frontend = Project {
        id: "proj-frontend".to_string(),
        name: "Frontend Service".to_string(),
        description: "SPA".to_string(),
        local_path: ".".to_string(),
        environments: vec![EnvironmentConfig {
            key: "prod".to_string(),
            name: "Production".to_string(),
            app_name: "frontend-prod".to_string(),
            servers: vec!["10.0.0.2".to_string()],
            remote_user: "deploy".to_string(),
            vault_target: "v2".to_string(),
            remote_dir: "/apps/frontend".to_string(),
            env_source: None,
            http_proxy: None,
            steps: Vec::new(),
        }],
        group: None,
        sort_order: None,
    };
    storage.projects = vec![proj_backend.clone(), proj_frontend.clone()];

    let mut app = ShiplyApp::new(storage);

    // 1. Initially neither project is deploying
    assert!(!app.is_project_deploying("proj-backend"));
    assert!(!app.is_project_deploying("proj-frontend"));
    assert!(!app.has_active_deployments());

    // 2. Start deployment for backend
    let req_backend = crate::executor::DeploymentRequest {
        deployment_id: "dep-backend-1".to_string(),
        project_id: "proj-backend".to_string(),
        project_name: "Backend Service".to_string(),
        local_path: PathBuf::from("."),
        env_config: proj_backend.environments[0].clone(),
        version: "1.0.0".to_string(),
        is_rollback: false,
        previous_commit: None,
        manual_password: None,
        target_git_branch: None,
        bump_package_json: false,
        create_git_tag_on_success: false,
        skip_ssh_preflight: false,
        rollback_commit: None,
    };
    app.start_deployment(req_backend);

    // Backend is now deploying
    assert!(app.is_project_deploying("proj-backend"));
    assert!(!app.is_project_deploying("proj-frontend")); // Frontend is free!
    assert!(app.has_active_deployments());

    // 3. Attempting to deploy backend AGAIN should be prevented
    let req_backend_again = crate::executor::DeploymentRequest {
        deployment_id: "dep-backend-2".to_string(),
        project_id: "proj-backend".to_string(),
        project_name: "Backend Service".to_string(),
        local_path: PathBuf::from("."),
        env_config: proj_backend.environments[0].clone(),
        version: "1.0.1".to_string(),
        is_rollback: false,
        previous_commit: None,
        manual_password: None,
        target_git_branch: None,
        bump_package_json: false,
        create_git_tag_on_success: false,
        skip_ssh_preflight: false,
        rollback_commit: None,
    };
    app.start_deployment(req_backend_again);
    let banner = app.status_banner.as_ref().unwrap();
    assert!(banner.0.contains("A deployment for 'Backend Service'"));
    assert!(banner.0.contains("is already running"));
    assert!(!banner.1); // false = error banner

    // 4. Deploying frontend AT THE SAME TIME is allowed and succeeds!
    let req_frontend = crate::executor::DeploymentRequest {
        deployment_id: "dep-frontend-1".to_string(),
        project_id: "proj-frontend".to_string(),
        project_name: "Frontend Service".to_string(),
        local_path: PathBuf::from("."),
        env_config: proj_frontend.environments[0].clone(),
        version: "2.0.0".to_string(),
        is_rollback: false,
        previous_commit: None,
        manual_password: None,
        target_git_branch: None,
        bump_package_json: false,
        create_git_tag_on_success: false,
        skip_ssh_preflight: false,
        rollback_commit: None,
    };
    app.start_deployment(req_frontend);

    // BOTH projects are now deploying concurrently!
    assert!(app.is_project_deploying("proj-backend"));
    assert!(app.is_project_deploying("proj-frontend"));
    assert_eq!(app.active_deployments.len(), 2);

    // 5. Simulate finish event for backend
    let finished_backend = DeploymentRecord {
        id: "dep-backend-1".to_string(),
        project_id: "proj-backend".to_string(),
        project_name: "Backend Service".to_string(),
        environment: "prod".to_string(),
        app_name: "backend-prod".to_string(),
        version: "1.0.0".to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("c1".to_string()),
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
    let (tx, rx) = mpsc::channel();
    tx.send(ExecutorEvent::Finished(finished_backend)).unwrap();
    app.event_rx = rx;
    app.poll_executor_events(&ctx);

    // Backend is no longer deploying, but Frontend is STILL deploying!
    assert!(!app.is_project_deploying("proj-backend"));
    assert!(app.is_project_deploying("proj-frontend"));
    assert_eq!(app.active_deployments.len(), 1);

    // Backend can now be deployed again
    let req_backend_new = crate::executor::DeploymentRequest {
        deployment_id: "dep-backend-3".to_string(),
        project_id: "proj-backend".to_string(),
        project_name: "Backend Service".to_string(),
        local_path: PathBuf::from("."),
        env_config: proj_backend.environments[0].clone(),
        version: "1.0.2".to_string(),
        is_rollback: false,
        previous_commit: None,
        manual_password: None,
        target_git_branch: None,
        bump_package_json: false,
        create_git_tag_on_success: false,
        skip_ssh_preflight: false,
        rollback_commit: None,
    };
    app.start_deployment(req_backend_new);
    assert!(app.is_project_deploying("proj-backend"));
}

#[test]
fn test_concurrent_deployments_for_multiple_environments_on_same_project() {
    let ctx = egui::Context::default();
    ShiplyApp::configure_light_workspace_style(&ctx);

    let mut storage = crate::storage::AppStorage::default();
    let proj_multi = Project {
        id: "proj-api".to_string(),
        name: "API Service".to_string(),
        description: "API".to_string(),
        local_path: ".".to_string(),
        environments: vec![
            EnvironmentConfig {
                key: "dev".to_string(),
                name: "Development".to_string(),
                app_name: "api-dev".to_string(),
                servers: vec!["10.0.0.10".to_string()],
                remote_user: "deploy".to_string(),
                vault_target: "v-dev".to_string(),
                remote_dir: "/apps/api-dev".to_string(),
                env_source: None,
                http_proxy: None,
                steps: Vec::new(),
            },
            EnvironmentConfig {
                key: "prod".to_string(),
                name: "Production".to_string(),
                app_name: "api-prod".to_string(),
                servers: vec!["10.0.0.20".to_string()],
                remote_user: "deploy".to_string(),
                vault_target: "v-prod".to_string(),
                remote_dir: "/apps/api-prod".to_string(),
                env_source: None,
                http_proxy: None,
                steps: Vec::new(),
            },
        ],
        group: None,
        sort_order: None,
    };
    storage.projects = vec![proj_multi.clone()];

    let mut app = ShiplyApp::new(storage);

    // 1. Initially neither environment is deploying
    assert!(!app.is_env_deploying("proj-api", "dev"));
    assert!(!app.is_env_deploying("proj-api", "prod"));
    assert!(!app.is_project_deploying("proj-api"));

    // 2. Start deployment for Development environment
    let req_dev = crate::executor::DeploymentRequest {
        deployment_id: "dep-api-dev-1".to_string(),
        project_id: "proj-api".to_string(),
        project_name: "API Service".to_string(),
        local_path: PathBuf::from("."),
        env_config: proj_multi.environments[0].clone(),
        version: "1.0.0".to_string(),
        is_rollback: false,
        previous_commit: None,
        manual_password: None,
        target_git_branch: None,
        bump_package_json: false,
        create_git_tag_on_success: false,
        skip_ssh_preflight: false,
        rollback_commit: None,
    };
    app.start_deployment(req_dev);

    // Development is deploying; Production is free
    assert!(app.is_env_deploying("proj-api", "dev"));
    assert!(!app.is_env_deploying("proj-api", "prod"));
    assert!(app.is_project_deploying("proj-api"));

    // 3. Attempting to deploy Development AGAIN on the same project is prevented
    let req_dev_again = crate::executor::DeploymentRequest {
        deployment_id: "dep-api-dev-2".to_string(),
        project_id: "proj-api".to_string(),
        project_name: "API Service".to_string(),
        local_path: PathBuf::from("."),
        env_config: proj_multi.environments[0].clone(),
        version: "1.0.1".to_string(),
        is_rollback: false,
        previous_commit: None,
        manual_password: None,
        target_git_branch: None,
        bump_package_json: false,
        create_git_tag_on_success: false,
        skip_ssh_preflight: false,
        rollback_commit: None,
    };
    app.start_deployment(req_dev_again);
    let banner = app.status_banner.as_ref().unwrap();
    assert!(banner
        .0
        .contains("A deployment for 'API Service' (DEV) is already running"));
    assert!(!banner.1); // error banner

    // 4. Deploying Production AT THE SAME TIME on the SAME PROJECT is allowed!
    let req_prod = crate::executor::DeploymentRequest {
        deployment_id: "dep-api-prod-1".to_string(),
        project_id: "proj-api".to_string(),
        project_name: "API Service".to_string(),
        local_path: PathBuf::from("."),
        env_config: proj_multi.environments[1].clone(),
        version: "1.0.0".to_string(),
        is_rollback: false,
        previous_commit: None,
        manual_password: None,
        target_git_branch: None,
        bump_package_json: false,
        create_git_tag_on_success: false,
        skip_ssh_preflight: false,
        rollback_commit: None,
    };
    app.start_deployment(req_prod);

    // BOTH Development and Production are now deploying simultaneously on the SAME project!
    assert!(app.is_env_deploying("proj-api", "dev"));
    assert!(app.is_env_deploying("proj-api", "prod"));
    assert_eq!(app.active_deployments.len(), 2);
    assert!(app.is_project_deploying("proj-api"));

    // 5. Attempting to deploy Production AGAIN while running is prevented
    let req_prod_again = crate::executor::DeploymentRequest {
        deployment_id: "dep-api-prod-2".to_string(),
        project_id: "proj-api".to_string(),
        project_name: "API Service".to_string(),
        local_path: PathBuf::from("."),
        env_config: proj_multi.environments[1].clone(),
        version: "1.0.1".to_string(),
        is_rollback: false,
        previous_commit: None,
        manual_password: None,
        target_git_branch: None,
        bump_package_json: false,
        create_git_tag_on_success: false,
        skip_ssh_preflight: false,
        rollback_commit: None,
    };
    app.start_deployment(req_prod_again);
    let banner_prod = app.status_banner.as_ref().unwrap();
    assert!(banner_prod
        .0
        .contains("A deployment for 'API Service' (PROD) is already running"));
    assert!(!banner_prod.1);

    // 6. Development deployment completes
    let finished_dev = DeploymentRecord {
        id: "dep-api-dev-1".to_string(),
        project_id: "proj-api".to_string(),
        project_name: "API Service".to_string(),
        environment: "dev".to_string(),
        app_name: "api-dev".to_string(),
        version: "1.0.0".to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("c1".to_string()),
        git_changes: vec![],
        is_rollback: false,
        servers: vec!["10.0.0.10".to_string()],
        status: DeploymentStatus::Succeeded,
        started_at: chrono::Utc::now(),
        finished_at: Some(chrono::Utc::now()),
        duration_ms: Some(800),
        step_logs: vec![],
        executed_steps: vec![],
    };
    let (tx, rx) = mpsc::channel();
    tx.send(ExecutorEvent::Finished(finished_dev)).unwrap();
    app.event_rx = rx;
    app.poll_executor_events(&ctx);

    // Development is done, but Production is STILL running!
    assert!(!app.is_env_deploying("proj-api", "dev"));
    assert!(app.is_env_deploying("proj-api", "prod"));
    assert_eq!(app.active_deployments.len(), 1);
    assert!(app.is_project_deploying("proj-api"));

    // 7. Development can now be deployed again while Production is still active
    let req_dev_new = crate::executor::DeploymentRequest {
        deployment_id: "dep-api-dev-3".to_string(),
        project_id: "proj-api".to_string(),
        project_name: "API Service".to_string(),
        local_path: PathBuf::from("."),
        env_config: proj_multi.environments[0].clone(),
        version: "1.0.2".to_string(),
        is_rollback: false,
        previous_commit: None,
        manual_password: None,
        target_git_branch: None,
        bump_package_json: false,
        create_git_tag_on_success: false,
        skip_ssh_preflight: false,
        rollback_commit: None,
    };
    app.start_deployment(req_dev_new);
    assert!(app.is_env_deploying("proj-api", "dev"));
    assert!(app.is_env_deploying("proj-api", "prod"));
    assert_eq!(app.active_deployments.len(), 2);
}
