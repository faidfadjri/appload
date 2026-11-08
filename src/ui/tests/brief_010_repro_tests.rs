//! Brief 010 reproduction and regression tests.

use super::*;
use crate::domain::{DeploymentRecord, DeploymentStatus};
use crate::storage::AppStorage;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use std::path::PathBuf;

fn make_test_deployment(
    id: &str,
    project_id: &str,
    project_name: &str,
    environment: &str,
    version: &str,
    status: DeploymentStatus,
) -> DeploymentRecord {
    DeploymentRecord {
        id: id.to_string(),
        project_id: project_id.to_string(),
        project_name: project_name.to_string(),
        environment: environment.to_string(),
        app_name: "testapp".to_string(),
        version: version.to_string(),
        git_branch: Some("main".to_string()),
        git_commit: Some("abcdef1".to_string()),
        git_changes: vec![],
        is_rollback: false,
        servers: vec!["server1".to_string()],
        status,
        started_at: chrono::Utc::now(),
        finished_at: Some(chrono::Utc::now()),
        duration_ms: Some(1000),
        step_logs: vec![],
        executed_steps: vec![],
    }
}

#[test]
fn test_repro_010_inspect_kafka_coordinates_and_kittest_delete() {
    let sandbox = crate::paths::TestSandbox::new();
    let src_db = PathBuf::from(
        r"C:\Users\ThinkPad\AppData\Local\Temp\appload-repro-010-v0121\LocalAppData\appload\appload_state.db",
    );
    if !src_db.exists() {
        return;
    }
    let test_db = sandbox.path().join("appload_state.db");
    std::fs::copy(&src_db, &test_db).expect("copy sandbox db");

    let storage = AppStorage::load_or_default(&test_db);
    assert_eq!(storage.deployments.len(), 165);
    let mut app = AppLoadApp::new(storage);
    app.selection = SidebarSelection::Project("kafka-provisioner".to_string());
    app.active_tab = WorkspaceTab::HistoryAndLogs;
    app.refresh_selected_project_metadata(false);
    app.project_switch_loading_id = None;
    app.project_switch_loading_start = None;

    let mut initialized = false;
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1440.0, 1100.0))
        .build(|ctx| {
            if !initialized {
                AppLoadApp::configure_light_workspace_style(ctx);
                initialized = true;
                return;
            }
            app.project_switch_loading_id = None;
            app.project_switch_loading_start = None;
            app.render_termius_topbar(ctx);
            app.render_sidebar(ctx, 1.0);
            app.render_delete_confirmation_modal(ctx);
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(CANVAS_BG)
                        .inner_margin(Margin::ZERO),
                )
                .show(ctx, |ui| {
                    ScrollArea::vertical()
                        .id_salt("main_workspace_vertical_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let avail_w = ui.available_width();
                            let h_pad = 32.0_f32;
                            let col_w = (avail_w - h_pad * 2.0).clamp(280.0, MAX_CONTENT_WIDTH);
                            let left_offset = ((avail_w - col_w) / 2.0).max(h_pad);
                            ui.add_space(SP_24);
                            ui.horizontal(|ui| {
                                ui.add_space(left_offset);
                                ui.vertical(|ui| {
                                    ui.set_width(col_w);
                                    if let SidebarSelection::Project(pid) = app.selection.clone() {
                                        app.render_project_workspace(ui, &pid);
                                    }
                                });
                            });
                        });
                });
        });
    harness.run_steps(3);

    // Click the 5th row ("v1.0.0 Failed" = index 4) "Row actions" button
    let row_buttons: Vec<_> = harness.query_all_by_label("Row actions").collect();
    assert_eq!(row_buttons.len(), 5);
    row_buttons[4].simulate_click();
    harness.run_steps(2);

    harness.get_by_label("Delete record").simulate_click();
    harness.run_steps(3);

    harness.get_by_label("Delete permanently").simulate_click();
    harness.run_steps(3);

    let row_buttons_after: Vec<_> = harness.query_all_by_label("Row actions").collect();
    eprintln!(
        "After deleting 63047f4e: visible Kafka rows = {}",
        row_buttons_after.len()
    );
    assert_eq!(row_buttons_after.len(), 4);

    drop(harness);

    eprintln!("After delete: status_banner = {:?}", app.status_banner);
    assert_eq!(
        app.status_banner.as_ref().map(|(m, ok)| (m.as_str(), *ok)),
        Some((
            "Deleted deployment record v1.0.0 (Kafka Provisioner - production).",
            true
        ))
    );
    assert_eq!(app.storage.deployments.len(), 164);

    // Run 60 sync cycles (simulating 2 minutes of 2-second sync checks)
    for _ in 0..60 {
        let _ = app.storage.sync_from_app_directory();
        assert_eq!(app.storage.deployments.len(), 164);
    }

    // Simulate app restart
    let reloaded = AppStorage::load_or_default(&test_db);
    assert_eq!(reloaded.deployments.len(), 164);
    assert!(!reloaded
        .deployments
        .iter()
        .any(|d| d.id == "63047f4e-d980-4a9d-8a19-286204c674eb"));
    assert!(reloaded
        .deleted_deployment_ids
        .contains("63047f4e-d980-4a9d-8a19-286204c674eb"));
}

#[test]
fn test_repro_010_stale_external_writer_and_tombstone_defense() {
    let sandbox = crate::paths::TestSandbox::new();
    let src_db = PathBuf::from(
        r"C:\Users\ThinkPad\AppData\Local\Temp\appload-repro-010-v0121\LocalAppData\appload\appload_state.db",
    );
    if !src_db.exists() {
        return;
    }
    let test_db = sandbox.path().join("appload_state.db");
    std::fs::copy(&src_db, &test_db).expect("copy sandbox db");

    let mut storage = AppStorage::load_or_default(&test_db);
    assert_eq!(storage.deployments.len(), 165);

    let target_id = "63047f4e-d980-4a9d-8a19-286204c674eb";
    let deleted_rec = storage
        .remove_deployment(target_id)
        .expect("delete deployment in v0.12.2");
    assert_eq!(storage.deployments.len(), 164);
    assert!(storage.deleted_deployment_ids.contains(target_id));

    // Simulate stale external writer (such as an old v0.12.0 process alive after update)
    // rewriting the table with its 165 stale records via rusqlite
    {
        let conn = rusqlite::Connection::open(&test_db).expect("open test db");
        crate::storage::upsert_deployment_to_db(&conn, &deleted_rec).expect("stale insert");
    }

    // Verify raw DB now has the resurrected row (165 rows)
    {
        let conn = rusqlite::Connection::open(&test_db).expect("open test db");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM deployments", [], |r| r.get(0))
            .expect("count rows");
        assert_eq!(count, 165, "raw DB was overwritten by stale writer");
    }

    // Now test Defense: sync_from_app_directory detects the modification,
    // recognizes the tombstoned ID, prevents resurrection into memory, and purges it from SQLite!
    // We adjust last_state_mtime to force sync reload
    storage.sync_from_app_directory();
    assert_eq!(
        storage.deployments.len(),
        164,
        "tombstone defense must reject resurrected record from memory"
    );
    assert!(!storage.deployments.iter().any(|d| d.id == target_id));

    // Verify raw DB had the resurrected row purged by tombstone defense
    {
        let conn = rusqlite::Connection::open(&test_db).expect("open test db");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM deployments", [], |r| r.get(0))
            .expect("count rows");
        assert_eq!(
            count, 164,
            "tombstone defense must purge resurrected row from SQLite"
        );
    }

    // Simulate restart of v0.12.2: load_or_default must also purge and filter
    let reloaded = AppStorage::load_or_default(&test_db);
    assert_eq!(reloaded.deployments.len(), 164);
    assert!(!reloaded.deployments.iter().any(|d| d.id == target_id));
    assert!(reloaded.deleted_deployment_ids.contains(target_id));
}

#[test]
fn test_repro_010_process_detection_and_wait_helpers() {
    let _sandbox = crate::paths::TestSandbox::new();

    // 1. In sandbox mode, host processes are excluded
    let sandbox_pids = crate::single_instance::find_other_running_instances();
    assert!(
        sandbox_pids.is_empty(),
        "sandbox must never match host processes"
    );

    // 2. Wait for a non-existent or dead PID returns true immediately
    let dead_pid = 99999999;
    let exited = crate::single_instance::wait_for_process_exit(
        dead_pid,
        std::time::Duration::from_millis(50),
    );
    assert!(exited, "dead PID must be reported as exited");

    // 3. wait_for_other_instances_to_exit in empty sandbox returns true
    assert!(crate::single_instance::wait_for_other_instances_to_exit(
        std::time::Duration::from_millis(50)
    ));
}

#[test]
fn test_repro_010_tombstone_metadata_lifecycle() {
    let sandbox = crate::paths::TestSandbox::new();
    let db_path = sandbox.path().join("lifecycle_test.db");

    let mut storage = AppStorage::load_or_default(&db_path);
    assert!(storage.deleted_deployment_ids.is_empty());

    // Create 3 synthetic deployment records
    let dep1 = make_test_deployment(
        "dep-uuid-1",
        "proj-1",
        "Test Project",
        "prod",
        "1.0.0",
        DeploymentStatus::Succeeded,
    );
    let dep2 = make_test_deployment(
        "dep-uuid-2",
        "proj-1",
        "Test Project",
        "prod",
        "1.0.1",
        DeploymentStatus::Failed,
    );
    let dep3 = make_test_deployment(
        "dep-uuid-3",
        "proj-2",
        "Other Project",
        "staging",
        "2.0.0",
        DeploymentStatus::Succeeded,
    );

    let id1 = dep1.id.clone();
    let id2 = dep2.id.clone();
    let id3 = dep3.id.clone();

    storage.upsert_deployment(dep1);
    storage.upsert_deployment(dep2);
    storage.upsert_deployment(dep3);
    assert_eq!(storage.deployments.len(), 3);

    // Delete dep1
    let removed = storage.remove_deployment(&id1).expect("remove dep1");
    assert_eq!(removed.id, id1);
    assert_eq!(storage.deployments.len(), 2);
    assert!(storage.deleted_deployment_ids.contains(&id1));

    // Clear history for proj-1 (should remove dep2)
    let cleared_count = storage
        .clear_deployment_history(Some("proj-1"), Some("prod"))
        .expect("clear proj-1 prod");
    assert_eq!(cleared_count, 1);
    assert_eq!(storage.deployments.len(), 1);
    assert_eq!(storage.deployments[0].id, id3);
    assert!(storage.deleted_deployment_ids.contains(&id1));
    assert!(storage.deleted_deployment_ids.contains(&id2));

    // Verify metadata table directly in SQLite
    {
        let conn = rusqlite::Connection::open(&db_path).expect("open db");
        let tombstones = crate::storage::load_tombstones_from_db(&conn).expect("load tombstones");
        assert!(tombstones.contains(&id1));
        assert!(tombstones.contains(&id2));
        assert!(!tombstones.contains(&id3));
    }

    // Reload from SQLite into a fresh instance
    let mut reloaded = AppStorage::load_or_default(&db_path);
    assert_eq!(reloaded.deployments.len(), 1);
    assert_eq!(reloaded.deployments[0].id, id3);
    assert!(reloaded.deleted_deployment_ids.contains(&id1));
    assert!(reloaded.deleted_deployment_ids.contains(&id2));

    // Try to merge a legacy/imported list containing the deleted dep1 and dep2
    let revive_dep1 = make_test_deployment(
        &id1,
        "proj-1",
        "Test Project",
        "prod",
        "1.0.0",
        DeploymentStatus::Succeeded,
    );
    let merged = reloaded.merge_deployments(vec![revive_dep1]);
    assert_eq!(
        merged, 0,
        "merge_deployments must ignore tombstoned deployment records"
    );
    assert_eq!(reloaded.deployments.len(), 1);
}
