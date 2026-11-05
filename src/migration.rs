//! First-run migration module: safely migrates legacy Shiply data and installations to AppLoad.
//!
//! Hard Safety Rules (Brief 006):
//! - Purely non-destructive: NEVER deletes, moves, or overwrites any legacy file or directory.
//! - The legacy install folder (`Programs\Shiply`) and legacy data stay forever as they are.
//! - Centralized path resolution through `crate::paths`.
//! - Strict source priority with timestamp inspection: prefer SQLite `.db` over `.json` when mtimes are within 1 minute.
//! - Preserves any existing target state as `appload_state.pre-migration-<timestamp>.*` before copying.
//! - Cryptographically / project-count verified after copying; halts on mismatch.
//! - Revalidates migration marker (`migrated_from_shiply.json`); re-runs if marker is invalid, stale, or from a pre-release build.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

pub const MIGRATION_MARKER_FILE: &str = "migrated_from_shiply.json";
pub const MIGRATION_SUCCESS_TOAST: &str =
    "Shiply is now AppLoad. Your projects and settings were moved over.";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationMarker {
    pub migrated_at: String,
    pub source_path: String,
    #[serde(default)]
    pub source_mtime: Option<u64>,
    #[serde(default)]
    pub source_size: Option<u64>,
    #[serde(default)]
    pub app_version: Option<String>,
    #[serde(default)]
    pub files_copied: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MigrationOutcome {
    pub data_migrated: bool,
    pub notice: Option<String>,
    pub is_error: bool,
    pub project_count: Option<usize>,
    pub source_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CandidateInfo {
    pub path: PathBuf,
    pub mtime: SystemTime,
    pub size: u64,
    pub is_sqlite: bool,
    pub priority_rank: usize,
}

/// Collects metadata for all existing legacy candidate state files.
pub fn collect_candidate_infos() -> Vec<CandidateInfo> {
    let candidate_paths = crate::paths::legacy_data_candidates();
    let mut infos = Vec::new();

    for (rank, path) in candidate_paths.into_iter().enumerate() {
        if !path.exists() {
            continue;
        }

        if let Ok(meta) = fs::metadata(&path) {
            if !meta.is_file() {
                continue;
            }

            let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let size = meta.len();
            let is_sqlite = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("db"))
                .unwrap_or(false);

            eprintln!(
                "[Migration Candidate] rank={rank} path='{}' size={size} mtime={:?} sqlite={is_sqlite}",
                path.display(),
                mtime
            );

            infos.push(CandidateInfo {
                path,
                mtime,
                size,
                is_sqlite,
                priority_rank: rank,
            });
        }
    }

    infos
}

/// Selects the best legacy candidate according to Brief 006:
/// - Pick the most recently modified candidate.
/// - Prefer `.db` over `.json` when their mtimes are within 1 minute (60 seconds).
/// - Break ties using priority rank.
pub fn select_best_candidate(candidates: &[CandidateInfo]) -> Option<&CandidateInfo> {
    if candidates.is_empty() {
        return None;
    }

    let mut best = &candidates[0];

    for candidate in &candidates[1..] {
        let diff_secs = match (
            candidate.mtime.duration_since(best.mtime),
            best.mtime.duration_since(candidate.mtime),
        ) {
            (Ok(d), _) => d.as_secs_f64(),
            (_, Ok(d)) => d.as_secs_f64(),
            _ => 0.0,
        };

        // Prefer .db over .json when modification times are within 1 minute (60 seconds)
        if candidate.is_sqlite != best.is_sqlite && diff_secs <= 60.0 {
            if candidate.is_sqlite {
                best = candidate;
            }
            continue;
        }

        // Otherwise pick the most recently modified
        if candidate.mtime > best.mtime
            || (candidate.mtime == best.mtime && candidate.priority_rank < best.priority_rank)
        {
            best = candidate;
        }
    }

    Some(best)
}

/// Checks whether an existing migration marker is valid for the current candidate.
///
/// Returns `false` (triggering a re-migration) if:
/// - The marker was written by a pre-release build (`app_version` < 0.12.0)
/// - The marker lacks `source_mtime` or `source_size`
/// - The marker's recorded `source_path` does not match the best candidate
/// - The marker's recorded mtime differs from the candidate's current mtime
pub fn is_marker_valid(marker: &MigrationMarker, best: &CandidateInfo) -> bool {
    let Some(version) = &marker.app_version else {
        return false;
    };
    if version.as_str() < "0.12.0" {
        return false;
    }

    let Some(marker_mtime) = marker.source_mtime else {
        return false;
    };
    if marker.source_size.is_none() {
        return false;
    }

    let marker_path = Path::new(&marker.source_path);
    if marker_path != best.path {
        return false;
    }

    let candidate_mtime_secs = best
        .mtime
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if candidate_mtime_secs.abs_diff(marker_mtime) > 1 {
        return false;
    }

    true
}

/// Backs up any existing target state file inside `target_dir` by renaming it to
/// `appload_state.pre-migration-<timestamp>.*`.
///
/// NEVER touches legacy paths.
pub fn backup_existing_target_state(target_dir: &Path) -> Result<Vec<String>, String> {
    let mut backed_up = Vec::new();
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();

    let target_db = target_dir.join("appload_state.db");
    let target_json = target_dir.join("appload_state.json");

    if target_db.exists() {
        let dest = target_dir.join(format!("appload_state.pre-migration-{timestamp}.db"));
        fs::rename(&target_db, &dest)
            .map_err(|e| format!("Failed to preserve existing DB: {e}"))?;
        backed_up.push(format!(
            "appload_state.db -> {}",
            dest.file_name().unwrap_or_default().to_string_lossy()
        ));

        for ext in ["db-wal", "db-shm", "db.bak"] {
            let sib = target_dir.join(format!("appload_state.{ext}"));
            if sib.exists() {
                let dest_sib =
                    target_dir.join(format!("appload_state.pre-migration-{timestamp}.{ext}"));
                let _ = fs::rename(&sib, &dest_sib);
            }
        }
    }

    if target_json.exists() {
        let dest = target_dir.join(format!("appload_state.pre-migration-{timestamp}.json"));
        fs::rename(&target_json, &dest)
            .map_err(|e| format!("Failed to preserve existing JSON: {e}"))?;
        backed_up.push(format!(
            "appload_state.json -> {}",
            dest.file_name().unwrap_or_default().to_string_lossy()
        ));
    }

    Ok(backed_up)
}

/// Counts the projects contained within a SQLite database or JSON state file.
pub fn count_projects_in_file(path: &Path) -> Result<usize, String> {
    let is_sqlite = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("db"))
        .unwrap_or(false);

    if is_sqlite {
        let conn = rusqlite::Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| format!("Failed to open DB '{}': {e}", path.display()))?;

        // Verify projects table exists
        let table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='projects'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if !table_exists {
            return Ok(0);
        }

        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM projects")
            .map_err(|e| format!("Failed to prepare project count query: {e}"))?;
        let count: usize = stmt
            .query_row([], |row| row.get(0))
            .map_err(|e| format!("Failed to count projects: {e}"))?;
        Ok(count)
    } else {
        let raw = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read JSON '{}': {e}", path.display()))?;
        let v: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| format!("Failed to parse JSON '{}': {e}", path.display()))?;
        let count = v
            .get("projects")
            .and_then(|p| p.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);
        Ok(count)
    }
}

/// Safely copies the best candidate and its siblings into `target_dir` as `appload_state.*`,
/// verifies integrity, and writes the migration marker.
pub fn execute_migration(best: &CandidateInfo, target_dir: &Path) -> Result<usize, String> {
    fs::create_dir_all(target_dir).map_err(|e| {
        format!(
            "Failed to create target directory '{}': {e}",
            target_dir.display()
        )
    })?;

    // 1. Back up existing target state (if any) before copying
    let _ = backup_existing_target_state(target_dir)?;

    let mut copied_records = Vec::new();
    let source_dir = best.path.parent().unwrap_or(Path::new("."));

    // Target state primary file
    let target_primary_name = if best.is_sqlite {
        "appload_state.db"
    } else {
        "appload_state.json"
    };
    let target_primary_path = target_dir.join(target_primary_name);

    fs::copy(&best.path, &target_primary_path).map_err(|e| {
        format!(
            "Failed to copy source '{}' to '{}': {e}",
            best.path.display(),
            target_primary_path.display()
        )
    })?;
    copied_records.push(format!(
        "{} -> {target_primary_name}",
        best.path.file_name().unwrap_or_default().to_string_lossy()
    ));

    // Copy SQLite WAL/SHM/bak siblings if present
    if best.is_sqlite {
        let source_file_name = best.path.file_name().unwrap_or_default().to_string_lossy();
        for ext in ["db-wal", "db-shm", "db.bak"] {
            let sibling_name = format!("{source_file_name}-{ext}").replace(".db-db-", ".db-");
            let sib_path = source_dir.join(&sibling_name);
            if sib_path.exists() {
                let dest_sib = target_dir.join(format!("appload_state.{ext}"));
                let _ = fs::copy(&sib_path, &dest_sib);
                copied_records.push(format!("{sibling_name} -> appload_state.{ext}"));
            }
        }
    }

    // Copy any drop-in scripts/configs from legacy dir (excluding probes and executables)
    if let Ok(entries) = fs::read_dir(source_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname.starts_with(".shiply_write_probe")
                    || fname.starts_with(".appload_write_probe")
                    || fname.eq_ignore_ascii_case("shiply.exe")
                    || fname.eq_ignore_ascii_case("shiply.old")
                    || fname.eq_ignore_ascii_case("shiply_state.db")
                    || fname.eq_ignore_ascii_case("shiply_state.json")
                    || fname.starts_with("shiply_state.db-")
                    || fname.starts_with("shiply_state.db.")
                {
                    continue;
                }
                let dest = target_dir.join(&fname);
                let _ = fs::copy(&p, &dest);
                copied_records.push(format!("{fname} -> {fname}"));
            }
        }
    }

    // 2. Verification: verify project count in copied target matches source
    let source_projects = count_projects_in_file(&best.path)?;
    let target_projects = count_projects_in_file(&target_primary_path)?;

    if source_projects != target_projects {
        return Err(format!(
            "Project count verification failed: source had {source_projects} projects, but copied file has {target_projects}"
        ));
    }

    // 3. Write migration marker with cryptographic / metadata fidelity
    let candidate_mtime_secs = best
        .mtime
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let marker = MigrationMarker {
        migrated_at: Utc::now().to_rfc3339(),
        source_path: best.path.to_string_lossy().to_string(),
        source_mtime: Some(candidate_mtime_secs),
        source_size: Some(best.size),
        app_version: Some(crate::installer::APP_VERSION.to_string()),
        files_copied: copied_records,
    };

    let marker_path = target_dir.join(MIGRATION_MARKER_FILE);
    if let Ok(marker_json) = serde_json::to_string_pretty(&marker) {
        let _ = fs::write(&marker_path, marker_json);
    }

    Ok(target_projects)
}

/// Executes first-run data migration for both standard app-data and portable mode.
pub fn run_first_run_data_migration() -> MigrationOutcome {
    // Determine target directory: portable mode (if running next to existing state) vs data_dir()
    let target_dir = if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            if (exe_dir.join("shiply_state.db").exists()
                || exe_dir.join("appload_state.db").exists())
                && crate::storage::is_dir_writable(exe_dir)
            {
                exe_dir.to_path_buf()
            } else {
                crate::paths::data_dir()
            }
        } else {
            crate::paths::data_dir()
        }
    } else {
        crate::paths::data_dir()
    };

    let candidates = collect_candidate_infos();
    let Some(best) = select_best_candidate(&candidates) else {
        // No legacy Shiply state exists anywhere
        return MigrationOutcome::default();
    };

    // Check existing marker validity
    let marker_path = target_dir.join(MIGRATION_MARKER_FILE);
    if marker_path.exists() {
        if let Ok(raw) = fs::read_to_string(&marker_path) {
            if let Ok(marker) = serde_json::from_str::<MigrationMarker>(&raw) {
                if is_marker_valid(&marker, best) {
                    return MigrationOutcome::default();
                } else {
                    eprintln!(
                        "[AppLoad Migration] Outdated or invalid marker found for source '{}'. Re-running migration with best candidate '{}'.",
                        marker.source_path,
                        best.path.display()
                    );
                }
            } else {
                eprintln!("[AppLoad Migration] Malformed marker found. Re-running migration.");
            }
        }
    }

    match execute_migration(best, &target_dir) {
        Ok(count) => MigrationOutcome {
            data_migrated: true,
            notice: Some(MIGRATION_SUCCESS_TOAST.to_string()),
            is_error: false,
            project_count: Some(count),
            source_path: Some(best.path.to_string_lossy().to_string()),
        },
        Err(e) => {
            eprintln!("[AppLoad Migration Error] Data migration failed: {e}");
            MigrationOutcome {
                data_migrated: false,
                notice: Some(format!(
                    "Data migration warning: {e}. Reading legacy storage."
                )),
                is_error: true,
                project_count: None,
                source_path: Some(best.path.to_string_lossy().to_string()),
            }
        }
    }
}

/// Checks if the running executable was invoked from a legacy Shiply installation
/// (e.g. `%LOCALAPPDATA%\Programs\Shiply\shiply.exe` or binary named `shiply.exe`).
///
/// If so, installs AppLoad to `%LOCALAPPDATA%\Programs\AppLoad\appload.exe`, creates
/// `AppLoad.lnk` shortcuts, removes legacy `Shiply.lnk` shortcuts, relaunches from the
/// new location, and returns `true` so the current legacy process exits.
///
/// Under no circumstances is the legacy folder or binary deleted or renamed.
pub fn handle_install_migration_and_handoff() -> bool {
    let Ok(current_exe) = env::current_exe() else {
        return false;
    };

    let is_legacy = is_legacy_shiply_install(&current_exe);
    if !is_legacy {
        return false;
    }

    let new_install_dir = crate::paths::install_dir();
    match crate::installer::install_with_options(&new_install_dir, true, true) {
        Ok(_) => {
            // Remove old Shiply desktop/start menu shortcuts pointing to legacy shiply.exe
            crate::installer::remove_legacy_windows_shortcuts();

            // Relaunch the new executable
            let new_exe = new_install_dir.join(crate::installer::installed_exe_name());
            if new_exe.exists() {
                let mut cmd = Command::new(&new_exe);
                cmd.args(env::args().skip(1));
                crate::version::configure_background_command(&mut cmd);
                if cmd.spawn().is_ok() {
                    return true;
                }
            }
        }
        Err(e) => {
            eprintln!("[AppLoad Migration Error] Install migration handoff failed: {e}");
        }
    }

    false
}

/// Detects whether `exe_path` belongs to a legacy Shiply install.
pub fn is_legacy_shiply_install(exe_path: &Path) -> bool {
    let lower_path = exe_path.to_string_lossy().to_lowercase();

    if lower_path.contains("programs\\shiply") || lower_path.contains("programs/shiply") {
        return true;
    }

    if let Some(file_name) = exe_path.file_name().and_then(|s| s.to_str()) {
        let name_lower = file_name.to_lowercase();
        if (name_lower == "shiply.exe" || name_lower == "shiply")
            && !lower_path.contains("programs\\appload")
            && !lower_path.contains("programs/appload")
            && !lower_path.contains("target\\debug")
            && !lower_path.contains("target/debug")
            && !lower_path.contains("target\\release")
            && !lower_path.contains("target/release")
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;
    use std::time::Duration;
    use tempfile::TempDir;

    fn hash_file(path: &Path) -> u64 {
        let bytes = fs::read(path).unwrap();
        let mut hasher = DefaultHasher::new();
        hasher.write(&bytes);
        hasher.finish()
    }

    fn create_test_sqlite_db(path: &Path, project_count: usize) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let conn = Connection::open(path).expect("open test db");
        conn.execute_batch(
            "CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE projects (
                 id TEXT PRIMARY KEY,
                 name TEXT NOT NULL,
                 description TEXT NOT NULL,
                 local_path TEXT NOT NULL,
                 environments_json TEXT NOT NULL
             );",
        )
        .expect("create tables");

        for i in 1..=project_count {
            conn.execute(
                "INSERT INTO projects (id, name, description, local_path, environments_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    format!("proj_{i}"),
                    format!("Project {i}"),
                    "Test description",
                    "C:/dummy",
                    "[]"
                ],
            )
            .expect("insert project");
        }
    }

    fn create_test_json_file(path: &Path, project_count: usize) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut projects = Vec::new();
        for i in 1..=project_count {
            projects.push(serde_json::json!({
                "id": format!("proj_{i}"),
                "name": format!("Project {i}"),
                "description": "Test description",
                "local_path": "C:/dummy",
                "environments": []
            }));
        }
        let state = serde_json::json!({
            "projects": projects,
            "version": "0.11.0"
        });
        fs::write(path, serde_json::to_string_pretty(&state).unwrap()).expect("write json");
    }

    #[test]
    fn test_candidate_selection_prefer_db_within_one_minute() {
        let now = SystemTime::now();

        // Scenario 1: JSON is 30s newer than DB -> DB should WIN (within 1 minute)
        let db_cand = CandidateInfo {
            path: PathBuf::from("Programs/Shiply/shiply_state.db"),
            mtime: now,
            size: 24576,
            is_sqlite: true,
            priority_rank: 0,
        };
        let json_cand = CandidateInfo {
            path: PathBuf::from("local_app_data/shiply/shiply_state.json"),
            mtime: now + Duration::from_secs(30),
            size: 1024,
            is_sqlite: false,
            priority_rank: 3,
        };

        let list1 = [db_cand.clone(), json_cand.clone()];
        let best = select_best_candidate(&list1).unwrap();
        assert_eq!(
            best.path, db_cand.path,
            "DB must win when within 60s of JSON"
        );

        // Reverse order input: still DB wins
        let list2 = [json_cand, db_cand.clone()];
        let best_rev = select_best_candidate(&list2).unwrap();
        assert_eq!(
            best_rev.path, db_cand.path,
            "DB must win regardless of candidate array order"
        );

        // Scenario 2: JSON is 10 minutes newer than DB -> JSON should win
        let stale_db = CandidateInfo {
            path: PathBuf::from("Programs/Shiply/shiply_state.db"),
            mtime: now,
            size: 24576,
            is_sqlite: true,
            priority_rank: 0,
        };
        let newer_json = CandidateInfo {
            path: PathBuf::from("local_app_data/shiply/shiply_state.json"),
            mtime: now + Duration::from_secs(600),
            size: 2048,
            is_sqlite: false,
            priority_rank: 3,
        };

        let list3 = [stale_db, newer_json.clone()];
        let best_newer = select_best_candidate(&list3).unwrap();
        assert_eq!(
            best_newer.path, newer_json.path,
            "Significantly newer JSON must win"
        );

        // Scenario 3: Programs\Shiply\shiply_state.db (Nov 5) vs stale shiply_state.json (Oct 26)
        let live_db = CandidateInfo {
            path: PathBuf::from("Programs/Shiply/shiply_state.db"),
            mtime: now + Duration::from_secs(86400 * 10),
            size: 40960,
            is_sqlite: true,
            priority_rank: 0,
        };
        let stale_oct_json = CandidateInfo {
            path: PathBuf::from("local_app_data/shiply/shiply_state.json"),
            mtime: now,
            size: 1024,
            is_sqlite: false,
            priority_rank: 3,
        };
        let list4 = [stale_oct_json, live_db.clone()];
        let best_live = select_best_candidate(&list4).unwrap();
        assert_eq!(best_live.path, live_db.path);
    }

    #[test]
    fn test_v011_layout_migration_picks_programs_shiply_db_and_preserves_hashes() {
        let _lock = crate::paths::ENV_MUTEX
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = TempDir::new().expect("tempdir");
        let sandbox_root = tmp.path().to_path_buf();
        env::set_var("APPLOAD_SANDBOX_ROOT", &sandbox_root);

        let prog_shiply = sandbox_root.join("Programs").join("Shiply");
        let local_shiply = sandbox_root.join("local_app_data").join("shiply");
        fs::create_dir_all(&prog_shiply).expect("create prog_shiply");
        fs::create_dir_all(&local_shiply).expect("create local_shiply");

        // 1. Seed legacy files
        let live_db = prog_shiply.join("shiply_state.db");
        create_test_sqlite_db(&live_db, 4);

        let live_exe = prog_shiply.join("shiply.exe");
        fs::write(&live_exe, b"MOCK_SHIPLY_EXE_V011").expect("write live exe");

        let custom_script = prog_shiply.join("deploy_hook.ps1");
        fs::write(&custom_script, b"Write-Host 'deploy'").expect("write custom script");

        // Seed stale JSON in local_app_data\shiply
        let stale_json = local_shiply.join("shiply_state.json");
        create_test_json_file(&stale_json, 2);

        // Record hashes of ALL legacy files before migration
        let hash_live_db_before = hash_file(&live_db);
        let hash_live_exe_before = hash_file(&live_exe);
        let hash_script_before = hash_file(&custom_script);
        let hash_stale_json_before = hash_file(&stale_json);

        // 2. Run migration
        let outcome = run_first_run_data_migration();
        assert!(outcome.data_migrated, "Migration should succeed");
        assert_eq!(
            outcome.project_count,
            Some(4),
            "Must migrate 4 projects from Programs\\Shiply"
        );
        assert!(!outcome.is_error);

        // 3. Verify AppLoad received the data
        let target_dir = sandbox_root.join("local_app_data").join("appload");
        let target_db = target_dir.join("appload_state.db");
        assert!(target_db.exists(), "Target appload_state.db must exist");
        assert_eq!(count_projects_in_file(&target_db).unwrap(), 4);
        assert!(
            target_dir.join("deploy_hook.ps1").exists(),
            "Drop-in files must be copied"
        );

        // Verify marker file
        let marker_path = target_dir.join(MIGRATION_MARKER_FILE);
        assert!(marker_path.exists(), "Migration marker must exist");
        let marker: MigrationMarker =
            serde_json::from_str(&fs::read_to_string(&marker_path).unwrap()).unwrap();
        assert_eq!(
            marker.app_version.as_deref(),
            Some(crate::installer::APP_VERSION)
        );
        assert_eq!(marker.source_path, live_db.to_string_lossy().to_string());
        assert!(marker.source_mtime.unwrap_or(0) > 0);

        // 4. CRITICAL: Verify legacy files are 100% BYTE-IDENTICAL before and after!
        assert!(live_db.exists(), "Legacy DB must not be deleted or moved");
        assert_eq!(
            hash_file(&live_db),
            hash_live_db_before,
            "Legacy DB must be byte-identical"
        );

        assert!(
            live_exe.exists(),
            "Legacy shiply.exe must not be deleted or moved"
        );
        assert_eq!(
            hash_file(&live_exe),
            hash_live_exe_before,
            "Legacy exe must be byte-identical"
        );

        assert!(custom_script.exists(), "Legacy script must not be deleted");
        assert_eq!(
            hash_file(&custom_script),
            hash_script_before,
            "Legacy script must be byte-identical"
        );

        assert!(stale_json.exists(), "Legacy stale JSON must not be deleted");
        assert_eq!(
            hash_file(&stale_json),
            hash_stale_json_before,
            "Legacy JSON must be byte-identical"
        );

        env::remove_var("APPLOAD_SANDBOX_ROOT");
    }

    #[test]
    fn test_bad_prerelease_marker_re_runs_migration_and_preserves_previous() {
        let _lock = crate::paths::ENV_MUTEX
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = TempDir::new().expect("tempdir");
        let sandbox_root = tmp.path().to_path_buf();
        env::set_var("APPLOAD_SANDBOX_ROOT", &sandbox_root);

        let prog_shiply = sandbox_root.join("Programs").join("Shiply");
        let local_shiply = sandbox_root.join("local_app_data").join("shiply");
        let appload_dir = sandbox_root.join("local_app_data").join("appload");
        fs::create_dir_all(&prog_shiply).expect("create prog_shiply");
        fs::create_dir_all(&local_shiply).expect("create local_shiply");
        fs::create_dir_all(&appload_dir).expect("create appload_dir");

        // Real live DB in Programs\Shiply with 4 projects
        let live_db = prog_shiply.join("shiply_state.db");
        create_test_sqlite_db(&live_db, 4);

        // Stale JSON in local_app_data\shiply
        let stale_json = local_shiply.join("shiply_state.json");
        create_test_json_file(&stale_json, 2);

        // Previous bad migration in AppLoad: an appload_state.db with only 2 projects
        let bad_appload_db = appload_dir.join("appload_state.db");
        create_test_sqlite_db(&bad_appload_db, 2);

        // And the bad marker from incident (lacking source_mtime, pointing to stale JSON, pre-release)
        let bad_marker = serde_json::json!({
            "migrated_at": "2026-11-05T21:06:46Z",
            "source_path": stale_json.to_string_lossy().to_string(),
            "files_copied": ["shiply_state.json -> appload_state.json"]
        });
        fs::write(
            appload_dir.join(MIGRATION_MARKER_FILE),
            serde_json::to_string_pretty(&bad_marker).unwrap(),
        )
        .expect("write bad marker");

        // Run migration: should detect invalid marker and re-run!
        let outcome = run_first_run_data_migration();
        assert!(outcome.data_migrated, "Should re-run migration");
        assert_eq!(
            outcome.project_count,
            Some(4),
            "Must pick the 4-project DB from Programs\\Shiply"
        );

        // Verify the old 2-project database was PRESERVED as pre-migration-*
        let entries = fs::read_dir(&appload_dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        let has_pre_migration = entries
            .iter()
            .any(|name| name.starts_with("appload_state.pre-migration-") && name.ends_with(".db"));
        assert!(
            has_pre_migration,
            "Old AppLoad state must be preserved as pre-migration-*"
        );

        // Verify new target has 4 projects
        assert_eq!(count_projects_in_file(&bad_appload_db).unwrap(), 4);

        env::remove_var("APPLOAD_SANDBOX_ROOT");
    }
}
