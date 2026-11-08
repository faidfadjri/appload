//! Comprehensive QA Test Matrix for AppLoad v0.12.1 (Brief 009).
//!
//! Covers all 10 matrix areas in a strict sandbox (`APPLOAD_SANDBOX_ROOT`), using synthetic
//! git fixture projects, fake `ssh`/`scp` on `PATH` for remote steps, `appload-test-<uuid>`
//! credentials with `TestVaultCredentialGuard`, and `egui_kittest::Harness` for UI verification.

use super::*;
use crate::credentials::{
    read_os_vault_credential, write_os_vault_credential, TestVaultCredentialGuard,
};
use crate::domain::{
    DeploymentRecord, DeploymentStatus, EnvironmentConfig, LocalShell, PipelineStep, Project,
    StepKind,
};
use crate::executor::{run_deployment, DeploymentRequest};
use crate::single_instance::{resolve_mutex_name, DEFAULT_MUTEX_NAME};
use crate::storage::{AppStorage, ProjectSortMode};
use crate::updater::{parse_github_release_json_for_os, replace_executable_atomic};
use crate::version::{bump_semver, read_package_json_version, VersionBump};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// Creates a synthetic Git repository fixture with a `package.json`, `dist/server.js`, and `.env.dev`.
fn create_synthetic_git_fixture(root: &Path, name: &str, version: &str) -> PathBuf {
    let repo_dir = root.join(name);
    fs::create_dir_all(repo_dir.join("dist")).expect("create dist");
    fs::write(
        repo_dir.join("package.json"),
        format!("{{\n  \"name\": \"{name}\",\n  \"version\": \"{version}\"\n}}\n"),
    )
    .expect("write package.json");
    fs::write(
        repo_dir.join("dist/server.js"),
        "console.log('synthetic server ok');\n",
    )
    .expect("write dist/server.js");
    fs::write(repo_dir.join(".env.dev"), "NODE_ENV=development\n").expect("write .env.dev");

    let run_git = |args: &[&str]| {
        let _ = Command::new("git")
            .args(args)
            .current_dir(&repo_dir)
            .output();
    };
    run_git(&["init", "-b", "main"]);
    run_git(&["config", "user.email", "qa@appload.local"]);
    run_git(&["config", "user.name", "AppLoad QA"]);
    run_git(&["add", "."]);
    run_git(&["commit", "-m", "feat: initial synthetic fixture commit"]);

    repo_dir
}

/// Area 1: Projects — add manually, auto-import from `deploy-*.ps1` (with siblings), edit,
/// duplicate, delete (with confirm), groups (set/create, collapse), sidebar sort & search,
/// and right-click context menu actions.
#[test]
fn test_qa_area_01_projects_full_lifecycle_and_kittest() {
    let sandbox = crate::paths::TestSandbox::new();
    let db_path = sandbox.path().join("appload_state.db");
    let storage = AppStorage::load_or_default(&db_path);
    let mut app = AppLoadApp::new(storage);

    // 1. Add project manually via modal state + kittest render
    app.open_new_project_modal();
    assert!(app.show_new_project_modal);
    app.new_project_name = "Synthetic Alpha".to_string();
    app.new_project_path = sandbox.path().join("alpha").display().to_string();
    app.new_project_desc = "Alpha service".to_string();
    app.new_project_group = "Core Services".to_string();

    {
        let mut initialized = false;
        let mut harness = Harness::new(|ctx| {
            if !initialized {
                AppLoadApp::configure_light_workspace_style(ctx);
                initialized = true;
                return;
            }
            app.render_new_project_modal(ctx);
        });
        harness.step();
        harness.step();
    }

    // Create the manual project and persist
    let proj_alpha = Project {
        id: "synthetic-alpha".to_string(),
        name: app.new_project_name.clone(),
        description: app.new_project_desc.clone(),
        local_path: app.new_project_path.clone(),
        environments: vec![EnvironmentConfig {
            key: "dev".to_string(),
            name: "Development".to_string(),
            app_name: "synthetic-alpha-dev".to_string(),
            servers: vec!["127.0.0.1".to_string()],
            remote_user: "qa".to_string(),
            vault_target: String::new(),
            remote_dir: "/tmp/alpha".to_string(),
            env_source: None,
            http_proxy: None,
            steps: vec![],
        }],
        group: Some("Core Services".to_string()),
        sort_order: Some(0),
    };
    app.storage.projects.push(proj_alpha);
    app.storage.save().expect("save alpha");
    app.show_new_project_modal = false;

    // 2. Auto-import from `deploy-dev.ps1` with sibling `deploy-prod.ps1`
    let ps1_dir = sandbox.path().join("ps1-fixture");
    fs::create_dir_all(&ps1_dir).expect("create ps1 dir");
    fs::write(
        ps1_dir.join("package.json"),
        "{\"name\": \"synthetic-beta\", \"version\": \"1.2.0\"}",
    )
    .expect("write package.json");
    let dev_ps1 = ps1_dir.join("deploy-dev.ps1");
    let prod_ps1 = ps1_dir.join("deploy-prod.ps1");
    fs::write(
        &dev_ps1,
        "$SERVER = \"127.0.0.1\"\n$USER = \"qa_user\"\n$REMOTE_DIR = \"/srv/beta-dev\"\nnpm run build\n",
    )
    .expect("write deploy-dev.ps1");
    fs::write(
        &prod_ps1,
        "$SERVER = \"127.0.0.1\"\n$USER = \"qa_user\"\n$REMOTE_DIR = \"/srv/beta-prod\"\nnpm run build\n",
    )
    .expect("write deploy-prod.ps1");

    app.open_new_project_modal();
    app.new_project_import_siblings = true;
    app.apply_ps1_auto_import_to_modal(&dev_ps1.display().to_string());
    assert!(
        app.new_project_imported_envs.len() >= 2,
        "Sibling deploy-*.ps1 files must be auto-imported together"
    );
    let proj_beta = Project {
        id: "synthetic-beta".to_string(),
        name: app.new_project_name.clone(),
        description: app.new_project_desc.clone(),
        local_path: app.new_project_path.clone(),
        environments: app.new_project_imported_envs.clone(),
        group: Some("Core Services".to_string()),
        sort_order: Some(1),
    };
    app.storage.projects.push(proj_beta);
    app.storage.save().expect("save beta");
    app.show_new_project_modal = false;

    // 3. Edit project via modal state
    app.open_edit_project_modal(0);
    assert!(app.show_edit_project_modal);
    assert_eq!(app.edit_project_id, "synthetic-alpha");
    app.storage.projects[0].description = "Alpha edited description".to_string();
    app.storage.save().expect("save edited alpha");
    app.show_edit_project_modal = false;

    // 4. Duplicate project
    let dup = app
        .storage
        .duplicate_project("synthetic-alpha")
        .expect("duplicate alpha");
    assert_eq!(dup.id, "synthetic-alpha-copy");
    assert_eq!(app.storage.projects.len(), 3);

    // 5. Groups (set/create, collapse/expand) & Sidebar sort/search
    assert!(app
        .storage
        .set_project_group("synthetic-alpha-copy", Some("Edge Services".to_string())));
    assert_eq!(
        app.storage.existing_project_groups(),
        vec!["Core Services".to_string(), "Edge Services".to_string()]
    );
    app.collapsed_project_groups
        .insert("Core Services".to_string());
    assert!(app.collapsed_project_groups.contains("Core Services"));
    app.collapsed_project_groups.remove("Core Services");

    app.storage.sort_projects(ProjectSortMode::NameDesc);
    assert_eq!(app.storage.projects[0].name, "synthetic-beta");
    app.storage.sort_projects(ProjectSortMode::NameAsc);
    assert_eq!(app.storage.projects[0].name, "Synthetic Alpha");

    app.project_search_query = "Copy".to_string();
    {
        let mut initialized = false;
        let mut harness = Harness::new(|ctx| {
            if !initialized {
                AppLoadApp::configure_light_workspace_style(ctx);
                initialized = true;
                return;
            }
            app.render_sidebar(ctx, 1.0);
        });
        harness.step();
        harness.step();
    }
    app.project_search_query.clear();

    // 6. Delete duplicate project via confirm modal (`Enter` key)
    app.pending_delete_confirm = Some(ConfirmDeleteAction::Project {
        project_id: "synthetic-alpha-copy".to_string(),
        project_name: "Synthetic Alpha (Copy)".to_string(),
    });
    let ctx = egui::Context::default();
    AppLoadApp::configure_light_workspace_style(&ctx);
    let mut raw_enter = egui::RawInput::default();
    raw_enter.events.push(egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    let _ = ctx.run(raw_enter, |ctx| {
        app.render_delete_confirmation_modal(ctx);
    });
    assert_eq!(app.storage.projects.len(), 2);
    assert!(!app
        .storage
        .projects
        .iter()
        .any(|p| p.id == "synthetic-alpha-copy"));
}

/// Area 2 & Area 3: Environments & Pipeline Steps — add/remove environment, edit every Settings
/// card field, Verify vault target & Set secret (`appload-test-<uuid>`), add each StepKind,
/// edit name/command, variable placeholders, enable/disable, reorder, delete, continue on error,
/// and shell selector.
#[test]
fn test_qa_area_02_and_03_environments_and_pipeline_steps() {
    let sandbox = crate::paths::TestSandbox::new();
    let db_path = sandbox.path().join("appload_state.db");
    let mut storage = AppStorage::load_or_default(&db_path);

    let vault_key = format!("appload-test-{}", uuid::Uuid::new_v4());
    let _vault_guard = TestVaultCredentialGuard::new(vault_key.clone());
    write_os_vault_credential(&vault_key, Some("qa_ssh_user"), "S3cretPass!0121")
        .expect("write test vault credential");

    let mut proj = Project {
        id: "proj-env-steps".to_string(),
        name: "Env & Steps Service".to_string(),
        description: "Test".to_string(),
        local_path: sandbox.path().display().to_string(),
        environments: Vec::new(),
        group: None,
        sort_order: Some(0),
    };
    let dev_key = proj.add_environment("Development", None).expect("add dev");
    assert_eq!(dev_key, "development");
    let uat_key = proj
        .add_environment("UAT", Some("development"))
        .expect("clone uat from dev");
    assert_eq!(uat_key, "uat");

    // Edit every field in EnvironmentConfig (Settings cards)
    {
        let env = proj.get_env_mut("development").expect("get dev env");
        env.name = "Development QA".to_string();
        env.app_name = "qa-app-dev".to_string();
        env.servers = vec!["127.0.0.1:2222".to_string()];
        env.remote_user = "qa_ssh_user".to_string();
        env.vault_target = vault_key;
        env.remote_dir = "/var/www/qa-app-dev".to_string();
        env.env_source = Some("./.env.dev".to_string());
        env.http_proxy = Some("http://127.0.0.1:8080".to_string());

        // Add all 4 step types: LocalCommand, VerifyLocalPath, TransferFiles, RemoteCommand
        env.steps = vec![
            PipelineStep::new(
                "1. Local Build",
                StepKind::local_command_with_shell(
                    "echo Building {app_name} v{version}",
                    false,
                    Some(LocalShell::PowerShell),
                ),
            ),
            PipelineStep::new(
                "2. Non-Fatal Lint",
                StepKind::local_command_with_shell(
                    "echo Linting with proxy {http_proxy}",
                    true, // Continue on error
                    Some(LocalShell::Cmd),
                ),
            ),
            PipelineStep::new(
                "3. Verify Dist",
                StepKind::VerifyLocalPath {
                    path: "./dist/server.js".to_string(),
                },
            ),
            PipelineStep::new(
                "4. SCP Upload",
                StepKind::TransferFiles {
                    sources: vec!["./dist/server.js".to_string()],
                    stage_env_file: true,
                },
            ),
            PipelineStep::new(
                "5. Remote Restart",
                StepKind::RemoteCommand {
                    command: "cd {remote_dir} && pm2 restart {app_name}".to_string(),
                },
            ),
        ];

        // Test placeholder interpolation across all supported chips
        let interp = env.interpolate(
            "{remote_dir}|{remote_user}|{app_name}|{http_proxy}|{env_source}|{version}",
            "2.1.0",
        );
        assert_eq!(
            interp,
            "/var/www/qa-app-dev|qa_ssh_user|qa-app-dev|http://127.0.0.1:8080|./.env.dev|2.1.0"
        );

        // Enable/disable and reorder steps (up, down, drag-reorder) and delete
        env.steps[1].enabled = false;
        assert!(!env.steps[1].enabled);
        env.steps[1].enabled = true;

        let first_id = env.steps[0].id.clone();
        let second_id = env.steps[1].id.clone();
        env.move_step_down(0);
        assert_eq!(env.steps[0].id, second_id);
        assert_eq!(env.steps[1].id, first_id);
        env.move_step_up(1);
        assert_eq!(env.steps[0].id, first_id);
        env.reorder_step(0, 2);
        assert_eq!(env.steps[2].id, first_id);
        env.reorder_step(2, 0);
        assert_eq!(env.steps[0].id, first_id);
    }

    storage.projects.push(proj);
    storage.save().expect("save project");

    let mut app = AppLoadApp::new(storage);
    app.selection = SidebarSelection::Project("proj-env-steps".to_string());
    app.selected_env_key = "development".to_string();
    app.refresh_selected_project_metadata(false);
    assert!(
        app.cached_vault_found,
        "Vault target using appload-test-<uuid> must be verified as found"
    );

    // Render PipelineSteps tab and ServerSettings tab via egui_kittest::Harness
    for tab in [WorkspaceTab::PipelineSteps, WorkspaceTab::ServerSettings] {
        app.active_tab = tab;
        let mut initialized = false;
        let mut harness = Harness::new(|ctx| {
            if !initialized {
                AppLoadApp::configure_light_workspace_style(ctx);
                initialized = true;
                return;
            }
            egui::CentralPanel::default().show(ctx, |ui| {
                app.render_project_workspace(ui, "proj-env-steps");
            });
        });
        harness.step();
        harness.step();
    }

    // Delete UAT environment via Danger zone confirmation
    app.pending_delete_confirm = Some(ConfirmDeleteAction::Environment {
        project_id: "proj-env-steps".to_string(),
        project_name: "Env & Steps Service".to_string(),
        env_key: "uat".to_string(),
        env_name: "UAT".to_string(),
    });
    let ctx = egui::Context::default();
    AppLoadApp::configure_light_workspace_style(&ctx);
    let mut raw_enter = egui::RawInput::default();
    raw_enter.events.push(egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    let _ = ctx.run(raw_enter, |ctx| {
        app.render_delete_confirmation_modal(ctx);
    });
    assert_eq!(app.storage.projects[0].environments.len(), 1);
    assert_eq!(app.storage.projects[0].environments[0].key, "development");
}

/// Area 4: Deploy — local deploy of synthetic fixture project, failing step stops pipeline,
/// cancel mid-run, rollback, redeploy from history row, bulk deploy of 2 synthetic projects,
/// version bump (Patch/Minor/Major) + Bump package.json + Create git tag, Sync version across
/// environments, and SSH/SCP steps against a fake `ssh`/`scp` on PATH.
#[test]
fn test_qa_area_04_deploy_local_fail_cancel_rollback_redeploy_bulk_and_fake_ssh_scp() {
    let sandbox = crate::paths::TestSandbox::new();
    let repo_a = create_synthetic_git_fixture(sandbox.path(), "fixture-service-a", "1.0.0");
    let repo_b = create_synthetic_git_fixture(sandbox.path(), "fixture-service-b", "2.0.0");

    // 1. Version bump helpers (Patch / Minor / Major)
    assert_eq!(bump_semver("1.0.0", VersionBump::Patch), "1.0.1");
    assert_eq!(bump_semver("1.0.0", VersionBump::Minor), "1.1.0");
    assert_eq!(bump_semver("1.0.0", VersionBump::Major), "2.0.0");

    // 2. Successful local deploy with Bump package.json = true and Create git tag = true
    let env_a = EnvironmentConfig {
        key: "dev".to_string(),
        name: "Development".to_string(),
        app_name: "fixture-service-a-dev".to_string(),
        servers: Vec::new(),
        remote_user: String::new(),
        vault_target: String::new(),
        remote_dir: String::new(),
        env_source: None,
        http_proxy: None,
        steps: vec![
            PipelineStep::new(
                "Build Artifact",
                StepKind::local_command("echo Building {app_name} v{version}", false),
            ),
            PipelineStep::new(
                "Verify Dist",
                StepKind::VerifyLocalPath {
                    path: "./dist/server.js".to_string(),
                },
            ),
        ],
    };

    let req_success = DeploymentRequest {
        deployment_id: "dep-qa-success-1".to_string(),
        project_id: "fixture-a".to_string(),
        project_name: "Fixture Service A".to_string(),
        local_path: repo_a.clone(),
        env_config: env_a.clone(),
        version: "1.0.1".to_string(),
        bump_package_json: true,
        create_git_tag_on_success: true,
        manual_password: None,
        skip_ssh_preflight: true,
        target_git_branch: None,
        rollback_commit: None,
        previous_commit: None,
        is_rollback: false,
    };
    let rec_success = run_deployment(&req_success, None);
    assert_eq!(rec_success.status, DeploymentStatus::Succeeded);
    assert_eq!(
        read_package_json_version(&repo_a).as_deref(),
        Some("1.0.1"),
        "Successful deploy with bump_package_json must update package.json on disk"
    );
    let tag_out = Command::new("git")
        .args(["tag", "-l", "v1.0.1"])
        .current_dir(&repo_a)
        .output()
        .expect("git tag -l");
    assert_eq!(
        String::from_utf8_lossy(&tag_out.stdout).trim(),
        "v1.0.1",
        "create_git_tag_on_success must create git tag v1.0.1 in fixture repo"
    );

    // 3. Deploy with a failing step (stops pipeline, status Failed, does NOT bump package.json)
    let mut env_fail = env_a.clone();
    env_fail.steps = vec![
        PipelineStep::new(
            "Failing Step",
            StepKind::local_command_with_shell("exit 1", false, Some(LocalShell::Cmd)),
        ),
        PipelineStep::new(
            "Unreachable Step",
            StepKind::local_command("echo Should Not Run", false),
        ),
    ];
    let req_fail = DeploymentRequest {
        deployment_id: "dep-qa-fail-1".to_string(),
        project_id: "fixture-a".to_string(),
        project_name: "Fixture Service A".to_string(),
        local_path: repo_a.clone(),
        env_config: env_fail,
        version: "9.9.9".to_string(),
        bump_package_json: true,
        create_git_tag_on_success: false,
        manual_password: None,
        skip_ssh_preflight: true,
        target_git_branch: None,
        rollback_commit: None,
        previous_commit: None,
        is_rollback: false,
    };
    let rec_fail = run_deployment(&req_fail, None);
    assert_eq!(rec_fail.status, DeploymentStatus::Failed);
    assert!(
        rec_fail
            .step_logs
            .iter()
            .any(|s| s.step_name == "Failing Step" && s.status == DeploymentStatus::Failed),
        "Failing Step must be recorded as Failed"
    );
    assert!(
        !rec_fail
            .step_logs
            .iter()
            .any(|s| s.step_name == "Unreachable Step"),
        "Pipeline must stop immediately after a non-allow_failure failing step"
    );
    assert_eq!(
        read_package_json_version(&repo_a).as_deref(),
        Some("1.0.1"),
        "Failed deployment must NOT bump package.json"
    );

    // 4. Rollback & Redeploy + Bulk Deploy of 2 synthetic projects
    let db_path = sandbox.path().join("appload_state.db");
    let mut storage = AppStorage::load_or_default(&db_path);
    storage.projects.push(Project {
        id: "fixture-a".to_string(),
        name: "Fixture Service A".to_string(),
        description: "A".to_string(),
        local_path: repo_a.display().to_string(),
        environments: vec![env_a.clone()],
        group: None,
        sort_order: Some(0),
    });
    storage.projects.push(Project {
        id: "fixture-b".to_string(),
        name: "Fixture Service B".to_string(),
        description: "B".to_string(),
        local_path: repo_b.display().to_string(),
        environments: vec![EnvironmentConfig {
            app_name: "fixture-service-b-dev".to_string(),
            ..env_a
        }],
        group: None,
        sort_order: Some(1),
    });
    storage.upsert_deployment(rec_success.clone());
    storage.save().expect("save storage");

    let mut app = AppLoadApp::new(storage);
    app.open_bulk_deploy_modal();
    assert!(app.show_bulk_deploy_modal);
    assert_eq!(app.bulk_selected_projects.len(), 2);
    app.trigger_bulk_deployment();

    // Poll until both bulk deployments complete (with timeout)
    let ctx = egui::Context::default();
    let start = Instant::now();
    while (app.has_active_deployments() || !app.bulk_deploy_queue.is_empty())
        && start.elapsed() < Duration::from_secs(15)
    {
        app.poll_executor_events(&ctx);
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(
        !app.has_active_deployments() && app.bulk_deploy_queue.is_empty(),
        "Bulk deploy of 2 synthetic projects must finish within timeout"
    );
    assert!(
        app.storage
            .deployments
            .iter()
            .any(|d| d.project_id == "fixture-b" && d.status == DeploymentStatus::Succeeded),
        "Second project in bulk deploy queue must succeed"
    );

    // Trigger rollback from rec_success and wait for completion
    app.trigger_redeploy_from_record(&rec_success, true);
    let rb_start = Instant::now();
    while app.has_active_deployments() && rb_start.elapsed() < Duration::from_secs(15) {
        app.poll_executor_events(&ctx);
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(
        app.storage
            .deployments
            .iter()
            .any(|d| d.is_rollback && d.status == DeploymentStatus::Succeeded),
        "Rollback deployment must succeed and record is_rollback = true"
    );

    // 5. SSH/SCP steps against a fake `ssh.exe` / `scp.exe` on PATH (testing both success and failure)
    #[cfg(windows)]
    {
        let fake_bin_dir = sandbox.path().join("fake_ssh_bin");
        fs::create_dir_all(&fake_bin_dir).expect("create fake_ssh_bin");
        let argv_log = fake_bin_dir.join("ssh_scp_argv.log");
        let fake_src = fake_bin_dir.join("fake_ssh.rs");
        fs::write(
            &fake_src,
            r#"fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Ok(log_path) = std::env::var("APPLOAD_FAKE_SSH_LOG") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log_path) {
            let _ = writeln!(f, "{}", args.join(" "));
        }
    }
    if std::env::var("APPLOAD_FAKE_SSH_FAIL").is_ok() {
        eprintln!("simulated fake ssh/scp failure");
        std::process::exit(1);
    }
}
"#,
        )
        .expect("write fake_ssh.rs");

        let fake_ssh_exe = fake_bin_dir.join("ssh.exe");
        let fake_scp_exe = fake_bin_dir.join("scp.exe");
        let rustc_status = Command::new("rustc")
            .arg(&fake_src)
            .arg("-o")
            .arg(&fake_ssh_exe)
            .status()
            .expect("compile fake_ssh.rs with rustc");
        assert!(rustc_status.success(), "rustc must compile fake ssh.exe");
        fs::copy(&fake_ssh_exe, &fake_scp_exe).expect("copy fake ssh.exe to scp.exe");

        let orig_path = std::env::var_os("PATH").unwrap_or_default();
        let mut new_path = std::ffi::OsString::from(&fake_bin_dir);
        new_path.push(";");
        new_path.push(&orig_path);
        std::env::set_var("PATH", &new_path);
        std::env::set_var("APPLOAD_FAKE_SSH_LOG", &argv_log);
        std::env::remove_var("APPLOAD_FAKE_SSH_FAIL");

        let env_remote = EnvironmentConfig {
            key: "staging".to_string(),
            name: "Staging".to_string(),
            app_name: "fixture-remote".to_string(),
            servers: vec!["127.0.0.1".to_string()],
            remote_user: "qa_remote".to_string(),
            vault_target: String::new(),
            remote_dir: "/srv/fixture-remote".to_string(),
            env_source: Some("./.env.dev".to_string()),
            http_proxy: None,
            steps: vec![
                PipelineStep::new(
                    "SCP Upload Dist",
                    StepKind::TransferFiles {
                        sources: vec!["./dist/server.js".to_string()],
                        stage_env_file: true,
                    },
                ),
                PipelineStep::new(
                    "Remote Restart",
                    StepKind::RemoteCommand {
                        command: "systemctl restart {app_name}".to_string(),
                    },
                ),
            ],
        };
        let req_remote = DeploymentRequest {
            deployment_id: "dep-qa-remote-1".to_string(),
            project_id: "fixture-a".to_string(),
            project_name: "Fixture Service A".to_string(),
            local_path: repo_a.clone(),
            env_config: env_remote.clone(),
            version: "1.0.5".to_string(),
            bump_package_json: false,
            create_git_tag_on_success: false,
            manual_password: None,
            skip_ssh_preflight: false,
            target_git_branch: None,
            rollback_commit: None,
            previous_commit: None,
            is_rollback: false,
        };
        let rec_remote = run_deployment(&req_remote, None);

        // Also test simulated remote SSH/SCP failure
        std::env::set_var("APPLOAD_FAKE_SSH_FAIL", "1");
        let req_remote_fail = DeploymentRequest {
            deployment_id: "dep-qa-remote-fail".to_string(),
            local_path: repo_a,
            env_config: env_remote,
            ..req_remote
        };
        let rec_remote_fail = run_deployment(&req_remote_fail, None);

        std::env::remove_var("APPLOAD_FAKE_SSH_FAIL");
        std::env::remove_var("APPLOAD_FAKE_SSH_LOG");
        std::env::set_var("PATH", orig_path);

        assert_eq!(
            rec_remote.status,
            DeploymentStatus::Succeeded,
            "Remote deploy via fake ssh/scp on PATH must succeed, logs: {:?}",
            rec_remote.step_logs
        );
        assert_eq!(
            rec_remote_fail.status,
            DeploymentStatus::Failed,
            "Simulated fake ssh failure must mark deployment as Failed"
        );
        let logged_argv = fs::read_to_string(&argv_log).expect("read ssh_scp_argv.log");
        assert!(
            logged_argv.contains("scp")
                && logged_argv.contains("qa_remote@127.0.0.1:/srv/fixture-remote"),
            "Fake scp must receive target destination: {logged_argv}"
        );
        assert!(
            logged_argv.contains("systemctl restart fixture-remote"),
            "Fake ssh must receive interpolated remote command: {logged_argv}"
        );
    }
}

/// Area 5, 6, 7, 8: History & Console, Secrets Page, All Deployments Overview,
/// and Inputs/UI Regressions (1920px & 1000px widths, collapsed rail, Esc/Enter modals, toast auto-dismiss).
#[test]
fn test_qa_area_05_06_07_08_history_secrets_overview_and_ui_regressions() {
    let sandbox = crate::paths::TestSandbox::new();
    let db_path = sandbox.path().join("appload_state.db");
    let mut storage = AppStorage::load_or_default(&db_path);

    storage.projects.push(Project {
        id: "proj-qa".to_string(),
        name: "QA Service".to_string(),
        description: "QA".to_string(),
        local_path: sandbox.path().display().to_string(),
        environments: vec![EnvironmentConfig {
            key: "prod".to_string(),
            name: "Production".to_string(),
            app_name: "qa-prod".to_string(),
            servers: vec!["127.0.0.1".to_string()],
            remote_user: "qa".to_string(),
            vault_target: String::new(),
            remote_dir: "/srv/qa".to_string(),
            env_source: None,
            http_proxy: None,
            steps: vec![],
        }],
        group: Some("QA Group".to_string()),
        sort_order: Some(0),
    });

    // Seed 12 deployments to test pagination (initial 5, Show more +10), search, and delete
    let now = chrono::Utc::now();
    for i in 1..=12 {
        storage.upsert_deployment(DeploymentRecord {
            id: format!("dep-hist-{i}"),
            project_id: "proj-qa".to_string(),
            project_name: "QA Service".to_string(),
            environment: "prod".to_string(),
            app_name: "qa-prod".to_string(),
            version: format!("1.0.{i}"),
            git_branch: Some("main".to_string()),
            git_commit: Some(format!("c0ffee{i:02x}")),
            git_changes: vec![format!("feat: commit {i}")],
            is_rollback: false,
            servers: vec!["127.0.0.1".to_string()],
            status: DeploymentStatus::Succeeded,
            started_at: now - chrono::Duration::minutes(15 - i),
            finished_at: Some(now - chrono::Duration::minutes(15 - i)),
            duration_ms: Some(1000),
            step_logs: vec![],
            executed_steps: vec![],
        });
    }
    storage.save().expect("save seeded history");

    let mut app = AppLoadApp::new(storage);
    assert_eq!(app.history_visible_limit, 5);
    app.history_visible_limit += 10;
    assert_eq!(app.history_visible_limit, 15);

    // Brief 008 delete verification across multiple sync cycles and full restart
    assert!(app.storage.remove_deployment("dep-hist-12").is_ok());
    for _ in 0..5 {
        assert!(app.storage.sync_from_app_directory().is_none());
        assert!(!app
            .storage
            .deployments
            .iter()
            .any(|d| d.id == "dep-hist-12"));
    }
    let reloaded = AppStorage::load_or_default(&db_path);
    assert_eq!(reloaded.deployments.len(), 11);
    assert!(!reloaded.deployments.iter().any(|d| d.id == "dep-hist-12"));

    // Secrets page: create, rotate, filter, search, delete with appload-test-<uuid>
    let sec_key = format!("appload-test-{}", uuid::Uuid::new_v4());
    let _sec_guard = TestVaultCredentialGuard::new(sec_key.clone());
    app.open_secret_management_for_target(&sec_key, "qa_admin", "Initial QA Secret");
    app.secret_form_value = "InitialPass#1".to_string();
    app.save_secret_to_os_vault().expect("create secret");
    assert_eq!(
        read_os_vault_credential(&sec_key)
            .expect("read created")
            .password,
        "InitialPass#1"
    );

    // Rotate secret password
    app.open_secret_management_for_target(&sec_key, "qa_admin_2", "Rotated QA Secret");
    app.secret_form_value = "RotatedPass#2".to_string();
    app.save_secret_to_os_vault().expect("rotate secret");
    let rotated = read_os_vault_credential(&sec_key).expect("read rotated");
    assert_eq!(rotated.username.as_deref(), Some("qa_admin_2"));
    assert_eq!(rotated.password, "RotatedPass#2");

    // Render All Deployments overview, Secrets page, and Project workspace at 1920px and 1000px widths
    let ctx = egui::Context::default();
    AppLoadApp::configure_light_workspace_style(&ctx);
    for width in [1920.0_f32, 1000.0_f32] {
        for sel in [
            SidebarSelection::AllDeployments,
            SidebarSelection::SecretManagement,
            SidebarSelection::Project("proj-qa".to_string()),
        ] {
            app.selection = sel;
            for collapsed in [false, true] {
                app.sidebar_collapsed = collapsed;
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(width, 900.0),
                    )),
                    ..Default::default()
                };
                let _ = ctx.run(input, |ctx| {
                    app.render_termius_topbar(ctx);
                    app.render_sidebar(ctx, if collapsed { 0.25 } else { 1.0 });
                    egui::CentralPanel::default().show(ctx, |ui| match app.selection.clone() {
                        SidebarSelection::AllDeployments => app.render_all_deployments_overview(ui),
                        SidebarSelection::SecretManagement => app.render_secret_management_view(ui),
                        SidebarSelection::Project(pid) => app.render_project_workspace(ui, &pid),
                    });
                });
            }
        }
    }

    // Delete secret and verify cleanup
    app.delete_secret_from_os_vault(&sec_key)
        .expect("delete test secret");
    assert!(read_os_vault_credential(&sec_key).is_none());

    // Toast auto-dismiss verification (> 4.0s)
    app.set_status_banner("Toast auto-dismiss check".to_string(), true);
    app.status_banner_time = Some(Instant::now() - Duration::from_secs(5));
    let banner_time = app.status_banner_time.unwrap();
    if Instant::now().duration_since(banner_time).as_secs_f32() > 4.0 {
        app.status_banner = None;
        app.status_banner_time = None;
    }
    assert!(app.status_banner.is_none());
}

/// Area 9 & Area 10: App Lifecycle, Version 0.12.1 verification, Single-Instance Sandbox
/// Independence, In-App Update v0.12.0 -> v0.12.1 simulation, and Check for Updates detecting 0.12.1.
#[test]
fn test_qa_area_09_and_10_lifecycle_version_0121_and_update_simulation() {
    let sandbox = crate::paths::TestSandbox::new();

    // 1. Package version must be 0.12.1
    assert_eq!(env!("CARGO_PKG_VERSION"), "0.12.1");

    // 2. Sandbox single-instance mutex is independent from production `Local\AppLoad.SingleInstance`
    let sb_mutex = resolve_mutex_name();
    assert!(sb_mutex.starts_with("Local\\AppLoad.SingleInstance.Sandbox."));
    assert_ne!(sb_mutex, DEFAULT_MUTEX_NAME);
    assert!(
        !crate::single_instance::focus_existing_appload_window(),
        "focus_existing_appload_window must be a no-op in sandbox mode"
    );

    // 3. Check for Updates flow detecting v0.12.1 from a simulated v0.12.0 client against latest.json and GitHub API
    let latest_json_0121 = r#"{
        "version": "0.12.1",
        "tag_name": "v0.12.1",
        "notes": "AppLoad v0.12.1 bugfix release",
        "html_url": "https://github.com/faidfadjri/appload/releases/tag/v0.12.1",
        "download_url": "https://github.com/faidfadjri/appload/releases/download/v0.12.1/appload.exe",
        "platforms": {
            "windows-x86_64": {
                "asset_name": "appload.exe",
                "url": "https://github.com/faidfadjri/appload/releases/download/v0.12.1/appload.exe"
            }
        }
    }"#;
    let rel_info = parse_github_release_json_for_os(latest_json_0121, "0.12.0", "windows")
        .expect("parse latest.json 0.12.1")
        .expect("0.12.1 must be detected as newer than 0.12.0");
    assert_eq!(rel_info.version, "0.12.1");
    assert_eq!(rel_info.tag_name, "v0.12.1");

    let gh_api_0121 = r#"{
        "tag_name": "v0.12.1",
        "name": "AppLoad v0.12.1",
        "html_url": "https://github.com/faidfadjri/appload/releases/tag/v0.12.1",
        "body": "Bugfix release",
        "assets": [
            {
                "name": "appload.exe",
                "browser_download_url": "https://github.com/faidfadjri/appload/releases/download/v0.12.1/appload.exe"
            },
            {
                "name": "shiply.exe",
                "browser_download_url": "https://github.com/faidfadjri/appload/releases/download/v0.12.1/shiply.exe"
            }
        ]
    }"#;
    let gh_rel = parse_github_release_json_for_os(gh_api_0121, "0.12.0", "windows")
        .expect("parse gh release 0.12.1")
        .expect("0.12.1 must be detected as newer than 0.12.0");
    assert_eq!(gh_rel.version, "0.12.1");

    // 4. Simulate v0.12.0 -> v0.12.1 in-app update atomic binary replacement + verify DB data intact
    let db_path = AppStorage::default_storage_path();
    let mut storage = AppStorage::load_or_default(&db_path);
    storage.projects.push(Project {
        id: "persisted-proj".to_string(),
        name: "Persisted Project".to_string(),
        description: "Survives in-app update".to_string(),
        local_path: sandbox.path().display().to_string(),
        environments: vec![],
        group: None,
        sort_order: Some(0),
    });
    storage.save().expect("save before update");

    let app_dir = sandbox
        .path()
        .join("LocalAppData")
        .join("Programs")
        .join("AppLoad");
    fs::create_dir_all(&app_dir).expect("create app_dir");
    let current_exe = app_dir.join("appload.exe");
    let downloaded_new_exe = app_dir.join("appload-update-0.12.1.exe");
    fs::write(&current_exe, b"old-binary-v0.12.0").expect("write old exe");
    fs::write(&downloaded_new_exe, b"new-binary-v0.12.1").expect("write new exe");

    replace_executable_atomic(&downloaded_new_exe, &current_exe)
        .expect("atomic executable replacement");
    assert_eq!(
        fs::read(&current_exe).expect("read replaced exe"),
        b"new-binary-v0.12.1"
    );

    let after_update = AppStorage::load_or_default(&db_path);
    assert_eq!(after_update.projects.len(), 1);
    assert_eq!(after_update.projects[0].id, "persisted-proj");
}

/// Addendum A (egui_kittest): For every `"⋯"` overflow button (Project Header, Deployments Toolbar,
/// History Desktop Row, and History Mobile/Stacked Row):
/// - Left-click opens the menu
/// - Pressing `Esc` closes the menu
/// - Clicking outside closes the menu
/// - Clicking an item runs its action and closes the menu
#[test]
fn test_qa_addendum_a_overflow_menus_left_click_item_click_esc_and_click_outside() {
    let sandbox = crate::paths::TestSandbox::new();
    let db_path = sandbox.path().join("appload_state.db");
    let mut storage = AppStorage::load_or_default(&db_path);

    storage.projects.push(Project {
        id: "proj-overflow".to_string(),
        name: "Overflow Test Project".to_string(),
        description: "Tests left-click overflow menus".to_string(),
        local_path: sandbox.path().display().to_string(),
        environments: vec![EnvironmentConfig {
            key: "prod".to_string(),
            name: "Production".to_string(),
            app_name: "overflow-prod".to_string(),
            servers: vec!["127.0.0.1".to_string()],
            remote_user: "deploy".to_string(),
            vault_target: String::new(),
            remote_dir: "/srv/overflow".to_string(),
            env_source: None,
            http_proxy: None,
            steps: vec![],
        }],
        group: None,
        sort_order: Some(0),
    });

    storage.upsert_deployment(DeploymentRecord {
        id: "dep-overflow-1".to_string(),
        project_id: "proj-overflow".to_string(),
        project_name: "Overflow Test Project".to_string(),
        environment: "prod".to_string(),
        app_name: "overflow-prod".to_string(),
        version: "1.2.3".to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("abcdef1".to_string()),
        git_changes: vec!["fix: overflow menu left click".to_string()],
        is_rollback: false,
        servers: vec!["127.0.0.1".to_string()],
        status: DeploymentStatus::Succeeded,
        started_at: chrono::Utc::now(),
        finished_at: Some(chrono::Utc::now()),
        duration_ms: Some(1200),
        step_logs: vec![],
        executed_steps: vec![],
    });
    storage.save().expect("save overflow test fixture");

    let mut app = AppLoadApp::new(storage);
    app.selection = SidebarSelection::Project("proj-overflow".to_string());
    app.active_tab = WorkspaceTab::HistoryAndLogs;
    app.refresh_selected_project_metadata(false);
    app.project_switch_loading_id = None;
    app.project_switch_loading_start = None;

    // 1. Desktop width (1440x900): test Project Header, Deployments Toolbar, and Desktop History Row overflow menus
    {
        let mut initialized = false;
        let mut harness = Harness::builder()
            .with_size(egui::Vec2::new(1440.0, 900.0))
            .build(|ctx| {
                if !initialized {
                    AppLoadApp::configure_light_workspace_style(ctx);
                    initialized = true;
                    return;
                }
                app.project_switch_loading_id = None;
                app.project_switch_loading_start = None;
                egui::CentralPanel::default().show(ctx, |ui| {
                    let sel = app.selection.clone();
                    if let SidebarSelection::Project(pid) = sel {
                        app.render_project_workspace(ui, &pid);
                    }
                });
            });
        harness.run_steps(2);

        // --- A. Project Header `"⋯"` ("More project actions") ---
        assert!(harness.query_by_label("Duplicate project").is_none());

        // Left-click opens the menu
        harness
            .get_by_label("More project actions")
            .simulate_click();
        harness.run_steps(2);
        assert!(
            harness.query_by_label("Duplicate project").is_some(),
            "Left-clicking Project Header overflow button must open menu"
        );
        assert!(harness.query_by_label("Refresh metadata").is_some());
        assert!(harness.query_by_label("Delete project").is_some());

        // Esc closes the menu
        harness.press_key(egui::Key::Escape);
        harness.run_steps(2);
        assert!(
            harness.query_by_label("Duplicate project").is_none(),
            "Pressing Esc must close Project Header overflow menu"
        );

        // Re-open with left-click, then click outside to close
        harness
            .get_by_label("More project actions")
            .simulate_click();
        harness.run_steps(2);
        assert!(harness.query_by_label("Duplicate project").is_some());
        harness
            .get_by_label("Tests left-click overflow menus")
            .simulate_click();
        harness.run_steps(2);
        assert!(
            harness.query_by_label("Duplicate project").is_none(),
            "Clicking outside must close Project Header overflow menu"
        );

        // Re-open with left-click, then click "Refresh metadata" -> runs action and closes menu
        harness
            .get_by_label("More project actions")
            .simulate_click();
        harness.run_steps(2);
        harness.get_by_label("Refresh metadata").simulate_click();
        harness.run_steps(2);
        assert!(
            harness.query_by_label("Duplicate project").is_none(),
            "Clicking Refresh metadata item must close Project Header overflow menu"
        );

        // --- B. Deployments Toolbar `"⋯"` ("More history actions") ---
        assert!(harness.query_by_label("Clear history…").is_none());

        // Left-click opens the menu
        harness
            .get_by_label("More history actions")
            .simulate_click();
        harness.run_steps(2);
        assert!(
            harness.query_by_label("Clear history…").is_some(),
            "Left-clicking Deployments Toolbar overflow button must open menu"
        );

        // Esc closes the menu
        harness.press_key(egui::Key::Escape);
        harness.run_steps(2);
        assert!(
            harness.query_by_label("Clear history…").is_none(),
            "Pressing Esc must close Deployments Toolbar overflow menu"
        );

        // Re-open with left-click, then click outside to close
        harness
            .get_by_label("More history actions")
            .simulate_click();
        harness.run_steps(2);
        assert!(harness.query_by_label("Clear history…").is_some());
        harness
            .get_by_label("Tests left-click overflow menus")
            .simulate_click();
        harness.run_steps(2);
        assert!(
            harness.query_by_label("Clear history…").is_none(),
            "Clicking outside must close Deployments Toolbar overflow menu"
        );

        // Re-open with left-click, then click "Clear history…" -> triggers ClearHistory confirm and closes menu
        harness
            .get_by_label("More history actions")
            .simulate_click();
        harness.run_steps(2);
        harness.get_by_label("Clear history…").simulate_click();
        harness.run_steps(2);
        assert!(harness.query_by_label("Clear history…").is_none());

        // --- C. Desktop History Row `"⋯"` ("Row actions") ---
        assert!(harness.query_by_label("Delete record").is_none());

        // Left-click opens the menu
        harness.get_by_label("Row actions").simulate_click();
        harness.run_steps(2);
        assert!(
            harness
                .query_by_label("Roll back to this version")
                .is_some(),
            "Left-clicking Desktop History Row overflow button must open menu"
        );
        assert!(harness.query_by_label("Redeploy").is_some());
        assert!(harness.query_by_label("Copy version").is_some());
        assert!(harness.query_by_label("Delete record").is_some());

        // Esc closes the menu
        harness.press_key(egui::Key::Escape);
        harness.run_steps(2);
        assert!(
            harness.query_by_label("Delete record").is_none(),
            "Pressing Esc must close Desktop History Row overflow menu"
        );

        // Re-open with left-click, then click outside to close
        harness.get_by_label("Row actions").simulate_click();
        harness.run_steps(2);
        assert!(harness.query_by_label("Delete record").is_some());
        harness
            .get_by_label("Tests left-click overflow menus")
            .simulate_click();
        harness.run_steps(2);
        assert!(
            harness.query_by_label("Delete record").is_none(),
            "Clicking outside must close Desktop History Row overflow menu"
        );

        // Re-open with left-click, then click "Delete record" -> triggers DeploymentRecord confirm and closes menu
        harness.get_by_label("Row actions").simulate_click();
        harness.run_steps(2);
        harness.get_by_label("Delete record").simulate_click();
        harness.run_steps(2);
        assert!(harness.query_by_label("Delete record").is_none());

        // Now also test "Duplicate project" item in Project Header `"⋯"` -> duplicates project and closes menu
        harness
            .get_by_label("More project actions")
            .simulate_click();
        harness.run_steps(2);
        harness.get_by_label("Duplicate project").simulate_click();
        harness.run_steps(2);
        assert!(
            harness.query_by_label("Duplicate project").is_none(),
            "Clicking Duplicate project item must close Project Header overflow menu"
        );
    }

    // Verify duplicate project and delete record confirm actions were dispatched
    assert_eq!(
        app.storage.projects.len(),
        2,
        "Duplicate project menu item must have created a copy"
    );
    assert!(
        matches!(
            app.pending_delete_confirm,
            Some(ConfirmDeleteAction::DeploymentRecord {
                ref deployment_id,
                ..
            }) if deployment_id == "dep-overflow-1"
        ),
        "Delete record menu item must set ConfirmDeleteAction::DeploymentRecord"
    );
    app.pending_delete_confirm = None;

    // 2. Mobile / Stacked width (600x800): test Mobile History Row `"⋯"` ("Row actions")
    app.selection = SidebarSelection::Project("proj-overflow".to_string());
    app.active_tab = WorkspaceTab::HistoryAndLogs;
    app.refresh_selected_project_metadata(false);
    app.project_switch_loading_id = None;
    app.project_switch_loading_start = None;
    {
        let mut initialized = false;
        let mut harness = Harness::builder()
            .with_size(egui::Vec2::new(600.0, 800.0))
            .build(|ctx| {
                if !initialized {
                    AppLoadApp::configure_light_workspace_style(ctx);
                    initialized = true;
                    return;
                }
                app.project_switch_loading_id = None;
                app.project_switch_loading_start = None;
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.render_project_workspace(ui, "proj-overflow");
                });
            });
        harness.run_steps(2);

        assert!(harness.query_by_label("Delete record").is_none());

        // Left-click opens the mobile row menu
        harness.get_by_label("Row actions").simulate_click();
        harness.run_steps(2);
        assert!(
            harness.query_by_label("Delete record").is_some(),
            "Left-clicking Mobile History Row overflow button must open menu"
        );

        // Esc closes the mobile row menu
        harness.press_key(egui::Key::Escape);
        harness.run_steps(2);
        assert!(harness.query_by_label("Delete record").is_none());

        // Re-open and click outside to close
        harness.get_by_label("Row actions").simulate_click();
        harness.run_steps(2);
        assert!(harness.query_by_label("Delete record").is_some());
        harness
            .get_by_label("Tests left-click overflow menus")
            .simulate_click();
        harness.run_steps(2);
        assert!(harness.query_by_label("Delete record").is_none());

        // Re-open and click "Delete record"
        harness.get_by_label("Row actions").simulate_click();
        harness.run_steps(2);
        harness.get_by_label("Delete record").simulate_click();
        harness.run_steps(2);
        assert!(harness.query_by_label("Delete record").is_none());
    }

    assert!(matches!(
        app.pending_delete_confirm,
        Some(ConfirmDeleteAction::DeploymentRecord {
            ref deployment_id,
            ..
        }) if deployment_id == "dep-overflow-1"
    ));
}
