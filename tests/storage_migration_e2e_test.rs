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

#[test]
fn test_storage_after_migration_loads_from_both_locations_without_legacy_writes() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let recovery_dir =
        PathBuf::from(r"C:\Users\ThinkPad\orca\briefs\shiply\recovery\20261105-215022");
    let recovery_db = recovery_dir.join("shiply_state.debug.db");
    let recovery_json = recovery_dir.join("localappdata-shiply_state.oct26.json");

    assert!(
        recovery_db.exists(),
        "Read-only recovery DB must exist at {}",
        recovery_db.display()
    );
    assert!(
        recovery_json.exists(),
        "Read-only recovery JSON must exist at {}",
        recovery_json.display()
    );

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

    // Populate legacy state (read-only copy from recovery)
    let legacy_db = prog_shiply.join("shiply_state.db");
    fs::copy(&recovery_db, &legacy_db).expect("copy legacy_db");
    let shm_src = recovery_dir.join("shiply_state.debug.db-shm");
    if shm_src.exists() {
        let _ = fs::copy(&shm_src, prog_shiply.join("shiply_state.db-shm"));
    }
    let wal_src = recovery_dir.join("shiply_state.debug.db-wal");
    if wal_src.exists() {
        let _ = fs::copy(&wal_src, prog_shiply.join("shiply_state.db-wal"));
    }

    let legacy_json = local_shiply.join("shiply_state.json");
    fs::copy(&recovery_json, &legacy_json).expect("copy legacy_json");

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
    fs::copy(&recovery_json, local_appload.join("appload_state.json"))
        .expect("copy stale incident json");

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
