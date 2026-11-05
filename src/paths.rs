//! Authoritative application path resolver with first-class sandbox isolation.
//!
//! All filesystem locations (application data, install directory, Start Menu, Desktop,
//! legacy Shiply locations, and state file candidates) MUST be resolved through this module.
//!
//! Direct calls to `dirs::*` anywhere else in the codebase are forbidden.
//!
//! # Sandbox Mode
//! When the environment variable `APPLOAD_SANDBOX_ROOT` is set, all paths resolve
//! under that root directory, guaranteeing zero side effects on the user's real profile:
//! - Data dir: `<root>/local_app_data/appload`
//! - Legacy data dir: `<root>/local_app_data/shiply`
//! - Install dir: `<root>/Programs/AppLoad`
//! - Legacy install dir: `<root>/Programs/Shiply`
//! - Start menu: `<root>/Start Menu/Programs`
//! - Desktop: `<root>/Desktop`
//!
//! Whenever sandbox mode is active, the resolver prints `SANDBOX MODE: <root>` to stderr
//! at startup.

use std::env;
use std::path::PathBuf;
#[cfg(test)]
use std::sync::Mutex;
use std::sync::Once;

static SANDBOX_LOGGED: Once = Once::new();

/// Returns the current sandbox root directory if `APPLOAD_SANDBOX_ROOT` is set and non-empty.
pub fn sandbox_root() -> Option<PathBuf> {
    if let Ok(val) = env::var("APPLOAD_SANDBOX_ROOT") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            let root = PathBuf::from(trimmed);
            SANDBOX_LOGGED.call_once(|| {
                eprintln!("SANDBOX MODE: {}", root.display());
            });
            return Some(root);
        }
    }
    None
}

/// Evaluates whether the startup guard should refuse execution.
pub fn should_refuse_startup() -> bool {
    let is_test = env::var("APPLOAD_TEST")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    is_test && sandbox_root().is_none()
}

/// Startup safety guard against running tests without sandbox isolation.
///
/// If `APPLOAD_TEST` is set to "1" or "true", `APPLOAD_SANDBOX_ROOT` MUST also be set.
/// If not, the process exits immediately with exit code 3 to protect the real system profile.
pub fn check_startup_guard() {
    if should_refuse_startup() {
        eprintln!(
            "[FATAL] APPLOAD_TEST is set but APPLOAD_SANDBOX_ROOT is not set. Refusing to run against real environment."
        );
        std::process::exit(3);
    }
}

/// Returns the authoritative application data directory where AppLoad stores its database
/// (`appload_state.db`), marker files, and drop-in configurations.
///
/// - In sandbox mode: `<APPLOAD_SANDBOX_ROOT>/local_app_data/appload`
/// - On Windows: `%LOCALAPPDATA%\appload` (fallback to `dirs::data_local_dir()/appload`)
/// - On macOS/Linux: `dirs::data_local_dir()/appload`
pub fn data_dir() -> PathBuf {
    if let Some(root) = sandbox_root() {
        return root.join("local_app_data").join("appload");
    }

    #[cfg(windows)]
    {
        if let Ok(local) = env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local.trim());
            if !p.as_os_str().is_empty() {
                return p.join("appload");
            }
        }
        if let Some(d) = dirs::data_local_dir() {
            return d.join("appload");
        }
    }

    #[cfg(not(windows))]
    {
        if let Some(d) = dirs::data_local_dir() {
            return d.join("appload");
        }
    }

    PathBuf::from("appload-data")
}

/// Returns the legacy Shiply application data directory (`%LOCALAPPDATA%\shiply`).
pub fn legacy_data_dir() -> Option<PathBuf> {
    if let Some(root) = sandbox_root() {
        return Some(root.join("local_app_data").join("shiply"));
    }

    #[cfg(windows)]
    {
        if let Ok(local) = env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local.trim());
            if !p.as_os_str().is_empty() {
                return Some(p.join("shiply"));
            }
        }
        dirs::data_local_dir().map(|d| d.join("shiply"))
    }

    #[cfg(not(windows))]
    {
        dirs::data_local_dir().map(|d| d.join("shiply"))
    }
}

/// Returns the application installation directory where `appload.exe` is installed.
///
/// - In sandbox mode: `<APPLOAD_SANDBOX_ROOT>/Programs/AppLoad`
/// - On Windows: `%LOCALAPPDATA%\Programs\AppLoad`
/// - On macOS: `~/Applications/AppLoad`
/// - On Linux: `~/.local/bin`
pub fn install_dir() -> PathBuf {
    if let Some(root) = sandbox_root() {
        return root.join("Programs").join("AppLoad");
    }

    #[cfg(windows)]
    {
        if let Ok(local) = env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local.trim());
            if !p.as_os_str().is_empty() {
                return p.join("Programs").join("AppLoad");
            }
        }
        if let Some(d) = dirs::data_local_dir() {
            return d.join("Programs").join("AppLoad");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            return home.join("Applications").join("AppLoad");
        }
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        if let Some(home) = dirs::home_dir() {
            return home.join(".local").join("bin");
        }
    }

    PathBuf::from("appload-install")
}

/// Returns the legacy Shiply installation directory (`%LOCALAPPDATA%\Programs\Shiply`).
pub fn legacy_install_dir() -> Option<PathBuf> {
    if let Some(root) = sandbox_root() {
        return Some(root.join("Programs").join("Shiply"));
    }

    #[cfg(windows)]
    {
        if let Ok(local) = env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local.trim());
            if !p.as_os_str().is_empty() {
                return Some(p.join("Programs").join("Shiply"));
            }
        }
        dirs::data_local_dir().map(|d| d.join("Programs").join("Shiply"))
    }

    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|h| h.join("Applications").join("Shiply"))
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        None
    }
}

/// Returns the user's Start Menu programs directory.
///
/// - In sandbox mode: `<APPLOAD_SANDBOX_ROOT>/Start Menu/Programs`
/// - On Windows: `%APPDATA%\Microsoft\Windows\Start Menu\Programs`
pub fn start_menu_dir() -> Option<PathBuf> {
    if let Some(root) = sandbox_root() {
        return Some(root.join("Start Menu").join("Programs"));
    }

    #[cfg(windows)]
    {
        if let Ok(roaming) = env::var("APPDATA") {
            let p = PathBuf::from(roaming.trim());
            if !p.as_os_str().is_empty() {
                return Some(
                    p.join("Microsoft")
                        .join("Windows")
                        .join("Start Menu")
                        .join("Programs"),
                );
            }
        }
        dirs::config_dir().map(|d| {
            d.join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
        })
    }

    #[cfg(not(windows))]
    {
        None
    }
}

/// Returns the user's Desktop directory.
///
/// - In sandbox mode: `<APPLOAD_SANDBOX_ROOT>/Desktop`
/// - On Windows: `%USERPROFILE%\Desktop` (or `dirs::desktop_dir()`)
pub fn desktop_dir() -> Option<PathBuf> {
    if let Some(root) = sandbox_root() {
        return Some(root.join("Desktop"));
    }

    #[cfg(windows)]
    {
        if let Ok(profile) = env::var("USERPROFILE") {
            let p = PathBuf::from(profile.trim());
            if !p.as_os_str().is_empty() {
                return Some(p.join("Desktop"));
            }
        }
        dirs::desktop_dir()
    }

    #[cfg(not(windows))]
    {
        dirs::desktop_dir()
    }
}

/// Returns all legacy state candidate paths in strict priority order (Brief 006):
/// 1. `<legacy_install_dir>\shiply_state.db` (Programs\Shiply)
/// 2. `<legacy_install_dir>\shiply_state.json` (Programs\Shiply)
/// 3. `%LOCALAPPDATA%\shiply\shiply_state.db`
/// 4. `%LOCALAPPDATA%\shiply\shiply_state.json`
/// 5. Same files next to currently running executable (portable mode)
pub fn legacy_data_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    // 1 & 2: Legacy install dir (Programs\Shiply)
    if let Some(leg_install) = legacy_install_dir() {
        candidates.push(leg_install.join("shiply_state.db"));
        candidates.push(leg_install.join("shiply_state.json"));
    }

    // 3 & 4: Legacy app data dir (%LOCALAPPDATA%\shiply)
    if let Some(leg_data) = legacy_data_dir() {
        candidates.push(leg_data.join("shiply_state.db"));
        candidates.push(leg_data.join("shiply_state.json"));
    }

    // 5 & 6: Running executable directory (portable mode)
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // In sandbox mode, only inspect exe_dir if it is inside the sandbox root
            let allow_exe_dir = match sandbox_root() {
                Some(ref root) => exe_dir.starts_with(root),
                None => true,
            };
            if allow_exe_dir {
                candidates.push(exe_dir.join("shiply_state.db"));
                candidates.push(exe_dir.join("shiply_state.json"));
            }
        }
    }

    // Deduplicate while preserving order
    let mut unique = Vec::new();
    for c in candidates {
        if !unique.contains(&c) {
            unique.push(c);
        }
    }

    unique
}

#[cfg(test)]
pub static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sandbox_root_redirects_all_paths() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = TempDir::new().expect("tempdir");
        let sandbox_path = tmp.path().to_path_buf();

        // Set APPLOAD_SANDBOX_ROOT
        env::set_var("APPLOAD_SANDBOX_ROOT", &sandbox_path);

        assert_eq!(sandbox_root(), Some(sandbox_path.clone()));

        let d_dir = data_dir();
        assert_eq!(d_dir, sandbox_path.join("local_app_data").join("appload"));

        let leg_data = legacy_data_dir();
        assert_eq!(
            leg_data,
            Some(sandbox_path.join("local_app_data").join("shiply"))
        );

        let inst_dir = install_dir();
        assert_eq!(inst_dir, sandbox_path.join("Programs").join("AppLoad"));

        let leg_inst = legacy_install_dir();
        assert_eq!(leg_inst, Some(sandbox_path.join("Programs").join("Shiply")));

        let start_menu = start_menu_dir();
        assert_eq!(
            start_menu,
            Some(sandbox_path.join("Start Menu").join("Programs"))
        );

        let desktop = desktop_dir();
        assert_eq!(desktop, Some(sandbox_path.join("Desktop")));

        // Clean up
        env::remove_var("APPLOAD_SANDBOX_ROOT");
        assert_eq!(sandbox_root(), None);
    }

    #[test]
    fn test_legacy_data_candidates_order() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = TempDir::new().expect("tempdir");
        env::set_var("APPLOAD_SANDBOX_ROOT", tmp.path());

        let candidates = legacy_data_candidates();
        assert!(candidates.len() >= 4);

        // Priority 1: Programs\Shiply\shiply_state.db
        assert!(
            candidates[0].ends_with("Programs\\Shiply\\shiply_state.db")
                || candidates[0].ends_with("Programs/Shiply/shiply_state.db")
        );

        // Priority 2: Programs\Shiply\shiply_state.json
        assert!(
            candidates[1].ends_with("Programs\\Shiply\\shiply_state.json")
                || candidates[1].ends_with("Programs/Shiply/shiply_state.json")
        );

        // Priority 3: shiply\shiply_state.db
        assert!(
            candidates[2].ends_with("local_app_data\\shiply\\shiply_state.db")
                || candidates[2].ends_with("local_app_data/shiply/shiply_state.db")
        );

        // Priority 4: shiply\shiply_state.json
        assert!(
            candidates[3].ends_with("local_app_data\\shiply\\shiply_state.json")
                || candidates[3].ends_with("local_app_data/shiply/shiply_state.json")
        );

        env::remove_var("APPLOAD_SANDBOX_ROOT");
    }

    #[test]
    fn test_startup_guard_refuses_when_appload_test_set_without_sandbox() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        env::remove_var("APPLOAD_SANDBOX_ROOT");
        env::remove_var("APPLOAD_TEST");

        assert!(!should_refuse_startup());

        // Set APPLOAD_TEST without sandbox root -> MUST refuse
        env::set_var("APPLOAD_TEST", "1");
        assert!(should_refuse_startup());

        // Now set APPLOAD_SANDBOX_ROOT -> MUST NOT refuse
        let tmp = TempDir::new().expect("tempdir");
        env::set_var("APPLOAD_SANDBOX_ROOT", tmp.path());
        assert!(!should_refuse_startup());

        // Clean up
        env::remove_var("APPLOAD_SANDBOX_ROOT");
        env::remove_var("APPLOAD_TEST");
    }
}
