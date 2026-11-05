//! Mandatory End-to-End Test for Safe Storage Loading and Migration (Brief 006 / 007 Part A)
//!
//! Verifies that:
//! 1. Migration correctly copies legacy DB from Programs\Shiply to %LOCALAPPDATA%\appload\appload_state.db.
//! 2. AppStorage constructs state exactly like `main.rs` does (`AppStorage::load_or_default(AppStorage::default_storage_path())`).
//! 3. 4 projects and 167 deployments are loaded.
//! 4. Running as both (a) `Programs\AppLoad\appload.exe` and (b) `Programs\Shiply\shiply.exe` resolves
//!    authoritative storage from `paths::data_dir()` and successfully loads all 4 projects and 167 deployments.
//! 5. Attempting to save storage to any legacy path is blocked by the safety guard.
//! 6. Legacy directories remain 100% byte-identical, with no new files created.
//!
//! Built self-contained for CI runners, with an `#[ignore]` variant for real recovery validation.

use appload::migration::run_first_run_data_migration;
use appload::storage::AppStorage;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use tempfile::TempDir;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

fn hash_file(path: &Path) -> (u64, u64) {
    let Ok(bytes) = fs::read(path) else {
        return (0, 0);
    };
    let mut h1: u64 = 0xcbf29ce484222325;
    let mut h2: u64 = 0x100000001b3;
    for &b in &bytes {
        h1 = (h1 ^ (b as u64)).wrapping_mul(0x100000001b3);
        h2 = (h2.wrapping_add(b as u64)).rotate_left(5);
    }
    (h1, bytes.len() as u64)
}

fn collect_dir_files(dir: &Path, base: &Path, manifest: &mut BTreeMap<String, (u64, u64)>) {
    if let Ok(entries) = fs::read_dir(dir) {
        let mut sorted_entries: Vec<_> = entries.flatten().collect();
        sorted_entries.sort_by_key(|e| e.path());
        for entry in sorted_entries {
            let path = entry.path();
            if path.is_dir() {
                collect_dir_files(&path, base, manifest);
            } else if path.is_file() {
                let rel = path
                    .strip_prefix(base)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                manifest.insert(rel, hash_file(&path));
            }
        }
    }
}

fn get_dir_manifest(dir: &Path) -> BTreeMap<String, (u64, u64)> {
    let mut manifest = BTreeMap::new();
    collect_dir_files(dir, dir, &mut manifest);
    manifest
}

/// Creates a synthetic legacy SQLite database containing 4 projects, 167 deployments, and 1 secret metadata row.
fn create_synthetic_legacy_db(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut conn = rusqlite::Connection::open(path).expect("create synthetic db");
    appload::storage::init_db(&conn).expect("init db");

    let project_names = [
        ("ndm-dq-subscription", "Backend Customer Data Ops"),
        ("frontend-customer-data-ops", "Frontend Customer Data Ops"),
        ("kafka-provisioner", "Kafka Provisioner"),
        ("ndm-dq-subscription-copy", "Tester CDO"),
    ];
    let mut projects = Vec::new();
    for (id, name) in project_names {
        projects.push(appload::domain::Project {
            id: id.to_string(),
            name: name.to_string(),
            description: "Synthetic project".to_string(),
            local_path: "/opt/app".to_string(),
            environments: Vec::new(),
            group: None,
            sort_order: None,
        });
    }

    let mut deployments = Vec::new();
    for i in 1..=167 {
        deployments.push(appload::domain::DeploymentRecord {
            id: format!("dep-synthetic-{i}"),
            project_id: "ndm-dq-subscription".to_string(),
            project_name: "Backend Customer Data Ops".to_string(),
            environment: "prod".to_string(),
            app_name: "ndm-dq-prod".to_string(),
            version: "1.0.0".to_string(),
            git_branch: Some("main".to_string()),
            git_commit: Some("c0ffee1".to_string()),
            git_changes: Vec::new(),
            is_rollback: false,
            servers: vec!["10.62.70.80".to_string()],
            status: appload::domain::DeploymentStatus::Succeeded,
            started_at: chrono::Utc::now(),
            finished_at: Some(chrono::Utc::now()),
            duration_ms: Some(1200),
            step_logs: Vec::new(),
            executed_steps: Vec::new(),
        });
    }

    let secrets = vec![appload::domain::SecretEntry {
        key: "PROD_VAULT_KEY".to_string(),
        username: Some("appload_admin".to_string()),
        description: "Production Secret".to_string(),
        updated_at: chrono::Utc::now(),
    }];

    appload::storage::save_all_to_db(&mut conn, &projects, &deployments).expect("save all");
    appload::storage::save_secrets_to_db(&mut conn, &secrets).expect("save secrets");
}

fn create_synthetic_stale_json(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let stale_data = serde_json::json!({
        "projects": [
            {
                "id": "stale-proj-1",
                "name": "Stale Proj 1",
                "local_path": "/stale/1",
                "environments": []
            },
            {
                "id": "stale-proj-2",
                "name": "Stale Proj 2",
                "local_path": "/stale/2",
                "environments": []
            }
        ],
        "deployments": []
    });
    fs::write(
        path,
        serde_json::to_string_pretty(&stale_data).expect("serialize"),
    )
    .expect("write stale json");
}

#[test]
fn test_storage_after_migration_loads_from_both_locations_without_legacy_writes() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let sandbox = TempDir::new().expect("tempdir for sandbox");
    let sandbox_root = sandbox.path().to_path_buf();
    env::set_var("APPLOAD_SANDBOX_ROOT", &sandbox_root);

    // 1. Setup sandbox directories
    let prog_shiply = sandbox_root
        .join("LocalAppData")
        .join("Programs")
        .join("Shiply");
    let prog_appload = sandbox_root
        .join("LocalAppData")
        .join("Programs")
        .join("AppLoad");
    let local_shiply = sandbox_root.join("LocalAppData").join("shiply");
    let local_appload = sandbox_root.join("LocalAppData").join("appload");

    fs::create_dir_all(&prog_shiply).expect("create prog_shiply");
    fs::create_dir_all(&prog_appload).expect("create prog_appload");
    fs::create_dir_all(&local_shiply).expect("create local_shiply");
    fs::create_dir_all(&local_appload).expect("create local_appload");

    // Copy binaries
    let bin_path = env!("CARGO_BIN_EXE_appload");
    let shiply_exe = prog_shiply.join("shiply.exe");
    let appload_exe = prog_appload.join("appload.exe");
    fs::copy(bin_path, &shiply_exe).expect("copy shiply_exe");
    fs::copy(bin_path, &appload_exe).expect("copy appload_exe");

    // Populate synthetic legacy state
    let legacy_db = prog_shiply.join("shiply_state.db");
    create_synthetic_legacy_db(&legacy_db);

    let legacy_json = local_shiply.join("shiply_state.json");
    create_synthetic_stale_json(&legacy_json);

    // Populate stale pre-release incident state in AppLoad to verify safe replacement
    let stale_marker = serde_json::json!({
        "migrated_at": "2026-11-05T21:06:46Z",
        "source_path": legacy_json.display().to_string(),
        "files_copied": ["shiply_state.json -> appload_state.json"]
    });
    fs::write(
        local_appload.join("migrated_from_shiply.json"),
        stale_marker.to_string(),
    )
    .expect("write stale marker");
    create_synthetic_stale_json(&local_appload.join("appload_state.json"));

    // Snapshot legacy directories BEFORE migration and loading
    let manifest_prog_before = get_dir_manifest(&prog_shiply);
    let manifest_local_before = get_dir_manifest(&local_shiply);

    // 2. Run first-run data migration
    let outcome = run_first_run_data_migration();
    assert!(
        outcome.data_migrated,
        "Data must be migrated from legacy Shiply: {:?}",
        outcome.notice
    );
    assert!(!outcome.is_error, "Migration must succeed");
    assert_eq!(outcome.project_count, Some(4));

    // 3. In-process: construct storage exactly like main.rs
    let storage_path = AppStorage::default_storage_path();
    assert_eq!(storage_path, local_appload.join("appload_state.db"));
    assert!(storage_path.exists());

    let mut storage = AppStorage::load_or_default(storage_path);
    assert_eq!(
        storage.projects.len(),
        4,
        "Storage must contain exactly 4 projects"
    );
    assert_eq!(
        storage.deployments.len(),
        167,
        "Storage must contain exactly 167 deployments"
    );

    // Verify storage safety guard refuses to write into legacy directories
    let legacy_save_target = prog_shiply.join("appload_state.db");
    let save_err = storage.save_to(&legacy_save_target);
    assert!(
        save_err.is_err(),
        "Storage::save_to must reject saving into legacy directory"
    );

    let legacy_data_save_target = local_shiply.join("appload_state.db");
    let save_err2 = storage.save_to(&legacy_data_save_target);
    assert!(
        save_err2.is_err(),
        "Storage::save_to must reject saving into legacy data directory"
    );

    // Normal storage save must succeed in paths::data_dir()
    assert!(storage.save().is_ok());

    // 4. Test out-of-process subprocess execution when running as (a) Programs\AppLoad\appload.exe
    let out_a = Command::new(&appload_exe)
        .arg("--check-storage")
        .env("APPLOAD_SANDBOX_ROOT", &sandbox_root)
        .output()
        .expect("run appload.exe --check-storage");
    assert!(
        out_a.status.success(),
        "appload.exe --check-storage failed: {}",
        String::from_utf8_lossy(&out_a.stderr)
    );
    let val_a: serde_json::Value =
        serde_json::from_slice(&out_a.stdout).expect("parse appload JSON output");
    assert_eq!(val_a["projects_count"], 4);
    assert_eq!(val_a["deployments_count"], 167);

    // 5. Test out-of-process subprocess execution when running as (b) Programs\Shiply\shiply.exe
    let out_b = Command::new(&shiply_exe)
        .arg("--check-storage")
        .env("APPLOAD_SANDBOX_ROOT", &sandbox_root)
        .output()
        .expect("run shiply.exe --check-storage");
    assert!(
        out_b.status.success(),
        "shiply.exe --check-storage failed: {}",
        String::from_utf8_lossy(&out_b.stderr)
    );
    let val_b: serde_json::Value =
        serde_json::from_slice(&out_b.stdout).expect("parse shiply JSON output");
    assert_eq!(val_b["projects_count"], 4);
    assert_eq!(val_b["deployments_count"], 167);

    // 6. Snapshot legacy directories AFTER all operations and assert byte-identical preservation
    let manifest_prog_after = get_dir_manifest(&prog_shiply);
    let manifest_local_after = get_dir_manifest(&local_shiply);

    assert_eq!(
        manifest_prog_before, manifest_prog_after,
        "Programs\\Shiply files must be 100% byte-identical with zero new or altered files"
    );
    assert_eq!(
        manifest_local_before, manifest_local_after,
        "LocalAppData\\shiply files must be 100% byte-identical with zero new or altered files"
    );

    env::remove_var("APPLOAD_SANDBOX_ROOT");
}

#[test]
#[ignore]
fn test_storage_migration_with_real_recovery_data() {
    let recovery_path_str = match env::var("APPLOAD_RECOVERY_DB") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            println!("Skipping real recovery test: APPLOAD_RECOVERY_DB is not set");
            return;
        }
    };
    let recovery_db = PathBuf::from(recovery_path_str);
    assert!(
        recovery_db.exists(),
        "APPLOAD_RECOVERY_DB file does not exist at {}",
        recovery_db.display()
    );

    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let sandbox = TempDir::new().expect("tempdir for sandbox");
    let sandbox_root = sandbox.path().to_path_buf();
    env::set_var("APPLOAD_SANDBOX_ROOT", &sandbox_root);

    let prog_shiply = sandbox_root
        .join("LocalAppData")
        .join("Programs")
        .join("Shiply");
    let local_appload = sandbox_root.join("LocalAppData").join("appload");
    fs::create_dir_all(&prog_shiply).expect("create prog_shiply");
    fs::create_dir_all(&local_appload).expect("create local_appload");

    let legacy_db = prog_shiply.join("shiply_state.db");
    fs::copy(&recovery_db, &legacy_db).expect("copy recovery db");

    let outcome = run_first_run_data_migration();
    assert!(outcome.data_migrated);
    assert_eq!(outcome.project_count, Some(4));

    let storage_path = AppStorage::default_storage_path();
    let storage = AppStorage::load_or_default(storage_path);
    assert_eq!(storage.projects.len(), 4);
    assert_eq!(storage.deployments.len(), 167);

    env::remove_var("APPLOAD_SANDBOX_ROOT");
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn with_file_exclusive_lock<F: FnOnce()>(path: &Path, f: F) {
    use std::os::windows::io::AsRawHandle;
    extern "system" {
        fn LockFile(
            hFile: *mut std::ffi::c_void,
            dwFileOffsetLow: u32,
            dwFileOffsetHigh: u32,
            nNumberOfBytesToLockLow: u32,
            nNumberOfBytesToLockHigh: u32,
        ) -> i32;
        fn UnlockFile(
            hFile: *mut std::ffi::c_void,
            dwFileOffsetLow: u32,
            dwFileOffsetHigh: u32,
            nNumberOfBytesToLockLow: u32,
            nNumberOfBytesToLockHigh: u32,
        ) -> i32;
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("open file for locking");
    let handle = file.as_raw_handle();
    let res = unsafe { LockFile(handle as *mut _, 0, 0, u32::MAX, 0) };
    assert_ne!(res, 0, "LockFile must succeed");

    f();

    unsafe {
        UnlockFile(handle as *mut _, 0, 0, u32::MAX, 0);
    }
}

#[cfg(not(windows))]
fn with_file_exclusive_lock<F: FnOnce()>(path: &Path, f: F) {
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_readonly(true);
    let _ = fs::set_permissions(path, perms.clone());
    f();
    perms.set_readonly(false);
    let _ = fs::set_permissions(path, perms);
}

#[test]
fn test_scenario_fresh_user_clean_start() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let sandbox = TempDir::new().expect("tempdir");
    let sandbox_root = sandbox.path().to_path_buf();
    env::set_var("APPLOAD_SANDBOX_ROOT", &sandbox_root);

    let local_appload = sandbox_root.join("LocalAppData").join("appload");
    fs::create_dir_all(&local_appload).expect("create appload dir");

    // Migration on fresh user: no legacy files
    let outcome = run_first_run_data_migration();
    assert!(!outcome.data_migrated);
    assert!(!outcome.is_error);
    assert!(outcome.notice.is_none());

    // No migration marker should exist
    assert!(!local_appload.join("migrated_from_shiply.json").exists());

    // Storage starts completely empty
    let storage_path = AppStorage::default_storage_path();
    let storage = AppStorage::load_or_default(&storage_path);
    assert!(storage.projects.is_empty());
    assert!(storage.deployments.is_empty());

    env::remove_var("APPLOAD_SANDBOX_ROOT");
}

#[test]
fn test_scenario_idempotent_second_launch() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let sandbox = TempDir::new().expect("tempdir");
    let sandbox_root = sandbox.path().to_path_buf();
    env::set_var("APPLOAD_SANDBOX_ROOT", &sandbox_root);

    let prog_shiply = sandbox_root
        .join("LocalAppData")
        .join("Programs")
        .join("Shiply");
    let local_appload = sandbox_root.join("LocalAppData").join("appload");
    fs::create_dir_all(&prog_shiply).expect("create prog_shiply");
    fs::create_dir_all(&local_appload).expect("create local_appload");

    let legacy_db = prog_shiply.join("shiply_state.db");
    create_synthetic_legacy_db(&legacy_db);

    // First migration run
    let outcome1 = run_first_run_data_migration();
    assert!(outcome1.data_migrated);
    assert!(!outcome1.is_error);

    let target_db = local_appload.join("appload_state.db");
    assert!(target_db.exists());
    let hash_after_first = hash_file(&target_db);

    let pre_migration_count_1 = fs::read_dir(&local_appload)
        .expect("read_dir")
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("appload_state.pre-migration-")
        })
        .count();

    // Second launch: must be idempotent
    let outcome2 = run_first_run_data_migration();
    assert!(!outcome2.data_migrated, "Second launch must not re-migrate");
    assert!(!outcome2.is_error);
    assert!(outcome2.notice.is_none(), "No second toast on idempotence");

    let hash_after_second = hash_file(&target_db);
    assert_eq!(
        hash_after_first, hash_after_second,
        "Target DB hash must be unchanged"
    );

    let pre_migration_count_2 = fs::read_dir(&local_appload)
        .expect("read_dir")
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("appload_state.pre-migration-")
        })
        .count();
    assert_eq!(
        pre_migration_count_1, pre_migration_count_2,
        "No redundant pre-migration backups created"
    );

    env::remove_var("APPLOAD_SANDBOX_ROOT");
}

#[test]
fn test_scenario_locked_legacy_db_surfaces_error_without_data_loss() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let sandbox = TempDir::new().expect("tempdir");
    let sandbox_root = sandbox.path().to_path_buf();
    env::set_var("APPLOAD_SANDBOX_ROOT", &sandbox_root);

    let prog_shiply = sandbox_root
        .join("LocalAppData")
        .join("Programs")
        .join("Shiply");
    let local_appload = sandbox_root.join("LocalAppData").join("appload");
    fs::create_dir_all(&prog_shiply).expect("create prog_shiply");
    fs::create_dir_all(&local_appload).expect("create local_appload");

    let legacy_db = prog_shiply.join("shiply_state.db");
    create_synthetic_legacy_db(&legacy_db);
    let hash_before = hash_file(&legacy_db);

    // Hold exclusive lock while attempting migration
    with_file_exclusive_lock(&legacy_db, || {
        let outcome = run_first_run_data_migration();
        assert!(
            outcome.is_error,
            "Migration must report error when DB is locked"
        );
        assert!(!outcome.data_migrated);
        assert!(outcome.notice.is_some());
    });

    let hash_after = hash_file(&legacy_db);
    assert_eq!(
        hash_before, hash_after,
        "Locked legacy DB must remain 100% byte-identical and untouched"
    );

    env::remove_var("APPLOAD_SANDBOX_ROOT");
}

#[test]
fn test_scenario_manual_install_path() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let sandbox = TempDir::new().expect("tempdir");
    let sandbox_root = sandbox.path().to_path_buf();
    env::set_var("APPLOAD_SANDBOX_ROOT", &sandbox_root);

    let prog_shiply = sandbox_root
        .join("LocalAppData")
        .join("Programs")
        .join("Shiply");
    let local_shiply = sandbox_root.join("LocalAppData").join("shiply");
    let local_appload = sandbox_root.join("LocalAppData").join("appload");
    fs::create_dir_all(&prog_shiply).expect("create prog_shiply");
    fs::create_dir_all(&local_shiply).expect("create local_shiply");
    fs::create_dir_all(&local_appload).expect("create local_appload");

    let legacy_db = prog_shiply.join("shiply_state.db");
    create_synthetic_legacy_db(&legacy_db);
    let legacy_json = local_shiply.join("shiply_state.json");
    create_synthetic_stale_json(&legacy_json);

    let manifest_prog_before = get_dir_manifest(&prog_shiply);
    let manifest_local_before = get_dir_manifest(&local_shiply);

    // Run installer headlessly (--install) like setup.exe does
    let bin_path = env!("CARGO_BIN_EXE_appload");
    let setup_exe = sandbox_root.join("AppLoad-Setup.exe");
    fs::copy(bin_path, &setup_exe).expect("copy setup binary");

    let install_out = Command::new(&setup_exe)
        .arg("--install")
        .env("APPLOAD_SANDBOX_ROOT", &sandbox_root)
        .output()
        .expect("run setup.exe --install");
    assert!(
        install_out.status.success(),
        "Setup failed: {}",
        String::from_utf8_lossy(&install_out.stderr)
    );

    let installed_exe = sandbox_root
        .join("LocalAppData")
        .join("Programs")
        .join("AppLoad")
        .join("appload.exe");
    assert!(
        installed_exe.exists(),
        "AppLoad must be installed in Programs\\AppLoad"
    );

    // Run migration via installed binary
    let mig_out = Command::new(&installed_exe)
        .arg("--migrate-only")
        .arg("--json")
        .env("APPLOAD_SANDBOX_ROOT", &sandbox_root)
        .output()
        .expect("run appload.exe --migrate-only");
    assert!(mig_out.status.success());
    let mig_val: serde_json::Value =
        serde_json::from_slice(&mig_out.stdout).expect("parse mig json");
    assert_eq!(mig_val["success"], true);
    assert_eq!(mig_val["project_count"], 4);

    // Verify storage loads 4 projects and 167 deployments
    let chk_out = Command::new(&installed_exe)
        .arg("--check-storage")
        .env("APPLOAD_SANDBOX_ROOT", &sandbox_root)
        .output()
        .expect("run appload.exe --check-storage");
    assert!(chk_out.status.success());
    let chk_val: serde_json::Value =
        serde_json::from_slice(&chk_out.stdout).expect("parse storage json");
    assert_eq!(chk_val["projects_count"], 4);
    assert_eq!(chk_val["deployments_count"], 167);

    // Legacy directories must remain 100% byte-identical
    let manifest_prog_after = get_dir_manifest(&prog_shiply);
    let manifest_local_after = get_dir_manifest(&local_shiply);
    assert_eq!(manifest_prog_before, manifest_prog_after);
    assert_eq!(manifest_local_before, manifest_local_after);

    env::remove_var("APPLOAD_SANDBOX_ROOT");
}
