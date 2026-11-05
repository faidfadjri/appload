//! First-run migration module: safely migrates legacy Shiply data and installations to AppLoad.
//!
//! Adrian's requirements (Brief 005):
//! - Idempotent, never destructive: copy (never move) legacy data, leaving the source untouched.
//! - Rename `shiply_state.*` -> `appload_state.*`.
//! - Record `migrated_from_shiply.json` marker with timestamp and source path so it never re-runs.
//! - Relaunch legacy `Programs\Shiply\shiply.exe` installations into `Programs\AppLoad\appload.exe`,
//!   swap desktop/start menu shortcuts (`Shiply.lnk` -> `AppLoad.lnk`), and clean up legacy binary.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const MIGRATION_MARKER_FILE: &str = "migrated_from_shiply.json";
pub const MIGRATION_SUCCESS_TOAST: &str =
    "Shiply is now AppLoad. Your projects and settings were moved over.";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationMarker {
    pub migrated_at: String,
    pub source_path: String,
    pub files_copied: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MigrationOutcome {
    pub data_migrated: bool,
    pub notice: Option<String>,
    pub is_error: bool,
}

/// Migrates data from `legacy_dir` to `new_dir` if `new_dir` does not yet contain state.
///
/// Returns `Ok(true)` if files were migrated, `Ok(false)` if migration was skipped (e.g. already
/// migrated, marker present, or new state already exists), or `Err(...)` on failure.
pub fn migrate_data_directory(legacy_dir: &Path, new_dir: &Path) -> Result<bool, String> {
    if !legacy_dir.exists() {
        return Ok(false);
    }

    // Never re-run if marker exists
    let marker_path = new_dir.join(MIGRATION_MARKER_FILE);
    if marker_path.exists() {
        return Ok(false);
    }

    // Do not overwrite existing AppLoad state
    let target_db = new_dir.join("appload_state.db");
    let target_json = new_dir.join("appload_state.json");
    if target_db.exists() || target_json.exists() {
        return Ok(false);
    }

    // Check if legacy source has any state files or drop-in files to copy
    let Ok(entries) = fs::read_dir(legacy_dir) else {
        return Ok(false);
    };

    let mut to_copy = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            // Don't copy old temp probe files
            if name_str.starts_with(".shiply_write_probe")
                || name_str.starts_with(".appload_write_probe")
            {
                continue;
            }
            to_copy.push((path, name_str.to_string()));
        }
    }

    if to_copy.is_empty() {
        return Ok(false);
    }

    fs::create_dir_all(new_dir).map_err(|e| {
        format!(
            "Failed to create new data directory '{}': {e}",
            new_dir.display()
        )
    })?;

    let mut copied_records = Vec::new();

    for (source_file, original_name) in &to_copy {
        // Map shiply_state.* to appload_state.*
        let dest_name = if original_name.starts_with("shiply_state.") {
            original_name.replacen("shiply_state.", "appload_state.", 1)
        } else if original_name == "shiply_state.db" {
            "appload_state.db".to_string()
        } else if original_name == "shiply_state.json" {
            "appload_state.json".to_string()
        } else {
            original_name.clone()
        };

        let dest_path = new_dir.join(&dest_name);
        if let Err(e) = fs::copy(source_file, &dest_path) {
            return Err(format!(
                "Failed to copy legacy file '{}' to '{}': {e}",
                source_file.display(),
                dest_path.display()
            ));
        }
        copied_records.push(format!("{original_name} -> {dest_name}"));
    }

    // Write marker file to ensure idempotence
    let marker = MigrationMarker {
        migrated_at: Utc::now().to_rfc3339(),
        source_path: legacy_dir.to_string_lossy().to_string(),
        files_copied: copied_records,
    };

    if let Ok(marker_json) = serde_json::to_string_pretty(&marker) {
        let _ = fs::write(&marker_path, marker_json);
    }

    Ok(true)
}

/// Executes first-run data migration for both standard app-data and portable mode.
pub fn run_first_run_data_migration() -> MigrationOutcome {
    // 1. Portable mode: check if running portably next to the executable
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let legacy_portable_db = exe_dir.join("shiply_state.db");
            let legacy_portable_json = exe_dir.join("shiply_state.json");
            let new_portable_db = exe_dir.join("appload_state.db");
            let new_portable_json = exe_dir.join("appload_state.json");

            if (legacy_portable_db.exists() || legacy_portable_json.exists())
                && !new_portable_db.exists()
                && !new_portable_json.exists()
                && !exe_dir.join(MIGRATION_MARKER_FILE).exists()
            {
                match migrate_data_directory(exe_dir, exe_dir) {
                    Ok(true) => {
                        return MigrationOutcome {
                            data_migrated: true,
                            notice: Some(MIGRATION_SUCCESS_TOAST.to_string()),
                            is_error: false,
                        };
                    }
                    Err(e) => {
                        eprintln!("[AppLoad Migration Error] Portable data migration failed: {e}");
                        return MigrationOutcome {
                            data_migrated: false,
                            notice: Some(format!("Migration warning: {e}")),
                            is_error: true,
                        };
                    }
                    Ok(false) => {}
                }
            }
        }
    }

    // 2. Standard application data folder (%LOCALAPPDATA%\appload vs %LOCALAPPDATA%\shiply)
    #[cfg(windows)]
    let base_data_dir = env::var("LOCALAPPDATA")
        .ok()
        .map(|s| PathBuf::from(s.trim()))
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(dirs::data_local_dir);
    #[cfg(not(windows))]
    let base_data_dir = dirs::data_local_dir();

    if let Some(base_data_dir) = base_data_dir {
        let legacy_data_dir = base_data_dir.join("shiply");
        let new_data_dir = base_data_dir.join("appload");

        match migrate_data_directory(&legacy_data_dir, &new_data_dir) {
            Ok(true) => {
                return MigrationOutcome {
                    data_migrated: true,
                    notice: Some(MIGRATION_SUCCESS_TOAST.to_string()),
                    is_error: false,
                };
            }
            Err(e) => {
                eprintln!("[AppLoad Migration Error] App-data migration failed: {e}");
                return MigrationOutcome {
                    data_migrated: false,
                    notice: Some(format!(
                        "Data migration warning: {e}. Reading legacy storage."
                    )),
                    is_error: true,
                };
            }
            Ok(false) => {}
        }
    }

    MigrationOutcome::default()
}

/// Checks if the running executable was invoked from a legacy Shiply installation
/// (e.g. `%LOCALAPPDATA%\Programs\Shiply\shiply.exe` or binary named `shiply.exe`).
///
/// If so, installs AppLoad to `%LOCALAPPDATA%\Programs\AppLoad\appload.exe`, creates
/// `AppLoad.lnk` shortcuts, deletes `Shiply.lnk` shortcuts, relaunches from the new
/// location, and returns `true` so the current legacy process exits.
pub fn handle_install_migration_and_handoff() -> bool {
    let Ok(current_exe) = env::current_exe() else {
        return false;
    };

    let is_legacy_shiply = is_legacy_shiply_install(&current_exe);
    if !is_legacy_shiply {
        // We are already running from the new AppLoad location or a portable/development path.
        // Perform best-effort cleanup of empty/outdated legacy Programs\Shiply folder.
        cleanup_legacy_install_dir_best_effort();
        return false;
    }

    // Perform self-installation to the new AppLoad install directory
    let new_install_dir = crate::installer::default_install_dir();
    match crate::installer::install_with_options(&new_install_dir, true, true) {
        Ok(_) => {
            // Remove old Shiply desktop/start menu shortcuts
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
            eprintln!("[AppLoad Migration Error] Install migration failed: {e}");
        }
    }

    false
}

/// Detects whether `exe_path` belongs to a legacy Shiply install.
pub fn is_legacy_shiply_install(exe_path: &Path) -> bool {
    let lower_path = exe_path.to_string_lossy().to_lowercase();

    // Check if located within Programs\Shiply
    if lower_path.contains("programs\\shiply") || lower_path.contains("programs/shiply") {
        return true;
    }

    // Check if named shiply.exe (while not inside Programs\AppLoad)
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

/// Removes the legacy `%LOCALAPPDATA%\Programs\Shiply` folder if it exists.
///
/// This is best-effort: if locked by another process or Windows pending file operations,
/// it safely ignores errors and can be retried on subsequent starts.
/// It NEVER touches the legacy data directory (`%LOCALAPPDATA%\shiply`).
pub fn cleanup_legacy_install_dir_best_effort() {
    if let Some(legacy_dir) = crate::installer::legacy_install_dir() {
        if legacy_dir.exists() {
            let _ = fs::remove_dir_all(&legacy_dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_data_migration_copies_and_renames_files() {
        let tmp = TempDir::new().expect("tempdir");
        let legacy_dir = tmp.path().join("shiply");
        let new_dir = tmp.path().join("appload");
        fs::create_dir_all(&legacy_dir).expect("create legacy");

        // Seed legacy files
        fs::write(legacy_dir.join("shiply_state.db"), "SQLITE_DATA").expect("write db");
        fs::write(legacy_dir.join("shiply_state.db-wal"), "WAL_DATA").expect("write wal");
        fs::write(legacy_dir.join("shiply_state.json"), "{\"projects\":[]}").expect("write json");
        fs::write(legacy_dir.join("custom_script.ps1"), "Write-Host 'hello'").expect("write ps1");

        // Run migration
        let migrated = migrate_data_directory(&legacy_dir, &new_dir).expect("migrate");
        assert!(migrated, "Expected migration to succeed");

        // 1. Verify files copied and renamed in new_dir
        assert!(new_dir.join("appload_state.db").exists());
        assert_eq!(
            fs::read_to_string(new_dir.join("appload_state.db")).unwrap(),
            "SQLITE_DATA"
        );
        assert!(new_dir.join("appload_state.db-wal").exists());
        assert_eq!(
            fs::read_to_string(new_dir.join("appload_state.db-wal")).unwrap(),
            "WAL_DATA"
        );
        assert!(new_dir.join("appload_state.json").exists());
        assert!(new_dir.join("custom_script.ps1").exists());

        // 2. Verify marker file created
        let marker_file = new_dir.join(MIGRATION_MARKER_FILE);
        assert!(marker_file.exists());
        let marker_content = fs::read_to_string(&marker_file).unwrap();
        assert!(marker_content.contains("shiply_state.db -> appload_state.db"));

        // 3. Verify legacy directory is completely intact (never moved or deleted)
        assert!(legacy_dir.join("shiply_state.db").exists());
        assert!(legacy_dir.join("shiply_state.json").exists());
        assert!(legacy_dir.join("custom_script.ps1").exists());
    }

    #[test]
    fn test_data_migration_is_idempotent_and_skips_when_marker_exists() {
        let tmp = TempDir::new().expect("tempdir");
        let legacy_dir = tmp.path().join("shiply");
        let new_dir = tmp.path().join("appload");
        fs::create_dir_all(&legacy_dir).expect("create legacy");
        fs::create_dir_all(&new_dir).expect("create new");

        fs::write(legacy_dir.join("shiply_state.db"), "OLD_DATA").expect("write db");
        fs::write(new_dir.join(MIGRATION_MARKER_FILE), "{}").expect("write marker");

        let migrated = migrate_data_directory(&legacy_dir, &new_dir).expect("migrate");
        assert!(
            !migrated,
            "Should not run migration when marker already exists"
        );
        assert!(!new_dir.join("appload_state.db").exists());
    }

    #[test]
    fn test_data_migration_does_not_overwrite_existing_new_state() {
        let tmp = TempDir::new().expect("tempdir");
        let legacy_dir = tmp.path().join("shiply");
        let new_dir = tmp.path().join("appload");
        fs::create_dir_all(&legacy_dir).expect("create legacy");
        fs::create_dir_all(&new_dir).expect("create new");

        fs::write(legacy_dir.join("shiply_state.db"), "OLD_DATA").expect("write db");
        fs::write(new_dir.join("appload_state.db"), "NEW_EXISTING_DATA").expect("write new db");

        let migrated = migrate_data_directory(&legacy_dir, &new_dir).expect("migrate");
        assert!(!migrated, "Should not overwrite existing AppLoad state");
        assert_eq!(
            fs::read_to_string(new_dir.join("appload_state.db")).unwrap(),
            "NEW_EXISTING_DATA"
        );
    }

    #[test]
    fn test_is_legacy_shiply_install_detection() {
        assert!(is_legacy_shiply_install(Path::new(
            r"C:\Users\test\AppData\Local\Programs\Shiply\shiply.exe"
        )));
        assert!(is_legacy_shiply_install(Path::new(
            "/home/user/Applications/Shiply/shiply"
        )));
        assert!(!is_legacy_shiply_install(Path::new(
            r"C:\Users\test\AppData\Local\Programs\AppLoad\appload.exe"
        )));
        assert!(!is_legacy_shiply_install(Path::new(
            r"D:\NTI\Labs\shiply\target\debug\appload.exe"
        )));
    }
}
