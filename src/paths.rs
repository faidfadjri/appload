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
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::Once;

static SANDBOX_LOGGED: Once = Once::new();

/// Global test synchronization lock to serialize all tests that set APPLOAD_SANDBOX_ROOT.
pub static TEST_MUTEX: Mutex<()> = Mutex::new(());
pub static ENV_MUTEX: &Mutex<()> = &TEST_MUTEX;

/// Hard guard: under `cfg(test)` or when `APPLOAD_TEST` is set, all path resolutions
/// MUST panic if no sandbox root is set, guaranteeing zero possibility of touching the real profile.
fn assert_sandbox_guard() {
    let requires_guard = cfg!(test) || env::var("APPLOAD_TEST").is_ok();
    if requires_guard && sandbox_root().is_none() {
        panic!(
            "TEST ISOLATION VIOLATION: paths::* was invoked in test mode without APPLOAD_SANDBOX_ROOT set! Operations must NEVER resolve real user profile paths."
        );
    }
}

/// RAII test sandbox helper: acquires `TEST_MUTEX`, creates a temporary sandbox folder,
/// and sets `APPLOAD_SANDBOX_ROOT` for the lifetime of this guard.
/// On drop, unsets `APPLOAD_SANDBOX_ROOT` and releases `TEST_MUTEX`.
#[cfg(test)]
pub struct TestSandbox {
    _lock: std::sync::MutexGuard<'static, ()>,
    temp_dir: tempfile::TempDir,
}

#[cfg(test)]
impl TestSandbox {
    pub fn new() -> Self {
        let lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let temp_dir = tempfile::TempDir::new().expect("create test sandbox tempdir");
        env::set_var("APPLOAD_SANDBOX_ROOT", temp_dir.path());
        Self {
            _lock: lock,
            temp_dir,
        }
    }

    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }
}

#[cfg(test)]
impl Default for TestSandbox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl Drop for TestSandbox {
    fn drop(&mut self) {
        env::remove_var("APPLOAD_SANDBOX_ROOT");
    }
}

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

fn sandbox_local_data(root: &std::path::Path) -> PathBuf {
    let p_local = root.join("LocalAppData");
    if p_local.exists() {
        p_local
    } else {
        root.join("local_app_data")
    }
}

/// Returns `true` if an explicit `appload.portable` marker file sits next to the currently running executable.
/// In sandbox mode, the executable directory must also be inside the sandbox root.
pub fn is_portable() -> bool {
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let allow_dir = match sandbox_root() {
                Some(ref root) => exe_dir.starts_with(root),
                None => true,
            };
            if allow_dir && exe_dir.join("appload.portable").is_file() {
                return true;
            }
        }
    }
    false
}

/// Returns the authoritative application data directory where AppLoad stores its database
/// (`appload_state.db`), marker files, and drop-in configurations.
///
/// - In portable mode (`appload.portable` marker present next to exe): the executable directory
/// - In sandbox mode: `<APPLOAD_SANDBOX_ROOT>/local_app_data/appload` (or `<root>/LocalAppData/appload`)
/// - On Windows: `%LOCALAPPDATA%\appload` (fallback to `dirs::data_local_dir()/appload`)
/// - On macOS/Linux: `dirs::data_local_dir()/appload`
pub fn data_dir() -> PathBuf {
    assert_sandbox_guard();
    if is_portable() {
        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                return exe_dir.to_path_buf();
            }
        }
    }

    if let Some(root) = sandbox_root() {
        return sandbox_local_data(&root).join("appload");
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
    assert_sandbox_guard();
    if let Some(root) = sandbox_root() {
        return Some(sandbox_local_data(&root).join("shiply"));
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
/// - In sandbox mode: `<APPLOAD_SANDBOX_ROOT>/Programs/AppLoad` (or `<root>/LocalAppData/Programs/AppLoad`)
/// - On Windows: `%LOCALAPPDATA%\Programs\AppLoad`
/// - On macOS: `~/Applications/AppLoad`
/// - On Linux: `~/.local/bin`
pub fn install_dir() -> PathBuf {
    assert_sandbox_guard();
    if let Some(root) = sandbox_root() {
        let p_local = root.join("LocalAppData").join("Programs").join("AppLoad");
        if p_local.exists() || root.join("LocalAppData").exists() {
            return p_local;
        }
        let p_local2 = root.join("local_app_data").join("Programs").join("AppLoad");
        if p_local2.exists() || root.join("local_app_data").exists() {
            return p_local2;
        }
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
    assert_sandbox_guard();
    if let Some(root) = sandbox_root() {
        let p_local = root.join("LocalAppData").join("Programs").join("Shiply");
        if p_local.exists() {
            return Some(p_local);
        }
        let p_local2 = root.join("local_app_data").join("Programs").join("Shiply");
        if p_local2.exists() {
            return Some(p_local2);
        }
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

/// Returns `true` if `path` is located inside any legacy Shiply directory (`legacy_install_dir` or `legacy_data_dir`).
pub fn is_in_legacy_dir(path: &Path) -> bool {
    fn is_subpath(parent: &Path, child: &Path) -> bool {
        if let (Ok(p_canon), Ok(c_canon)) = (parent.canonicalize(), child.canonicalize()) {
            if c_canon.starts_with(&p_canon) {
                return true;
            }
        }
        let parent_norm = parent.to_string_lossy().replace('/', "\\").to_lowercase();
        let child_norm = child.to_string_lossy().replace('/', "\\").to_lowercase();
        let parent_clean = parent_norm.trim_end_matches('\\');
        if child_norm == parent_clean {
            return true;
        }
        if let Some(remainder) = child_norm.strip_prefix(parent_clean) {
            if remainder.starts_with('\\') {
                return true;
            }
        }
        false
    }

    if let Some(leg_install) = legacy_install_dir() {
        if is_subpath(&leg_install, path) {
            return true;
        }
    }
    if let Some(leg_data) = legacy_data_dir() {
        if is_subpath(&leg_data, path) {
            return true;
        }
    }
    false
}

/// Returns the user's Start Menu programs directory.
///
/// - In sandbox mode: `<APPLOAD_SANDBOX_ROOT>/Start Menu/Programs`
/// - On Windows: `%APPDATA%\Microsoft\Windows\Start Menu\Programs`
pub fn start_menu_dir() -> Option<PathBuf> {
    assert_sandbox_guard();
    if let Some(root) = sandbox_root() {
        let p_roaming = root
            .join("AppData")
            .join("Roaming")
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs");
        if p_roaming.exists() {
            return Some(p_roaming);
        }
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
    assert_sandbox_guard();
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
    assert_sandbox_guard();
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
                let leg_db = exe_dir.join("shiply_state.db");
                let leg_json = exe_dir.join("shiply_state.json");
                if is_portable() || leg_db.exists() || leg_json.exists() {
                    candidates.push(leg_db);
                    candidates.push(leg_json);
                }
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

    #[test]
    fn test_is_in_legacy_dir() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        env::set_var("APPLOAD_SANDBOX_ROOT", root);

        let leg_inst = legacy_install_dir().expect("leg_inst");
        let leg_data = legacy_data_dir().expect("leg_data");

        assert!(is_in_legacy_dir(&leg_inst));
        assert!(is_in_legacy_dir(&leg_inst.join("shiply_state.db")));
        assert!(is_in_legacy_dir(&leg_inst.join("nested/folder/file.txt")));
        assert!(is_in_legacy_dir(&leg_data));
        assert!(is_in_legacy_dir(&leg_data.join("shiply_state.json")));

        let safe_data = data_dir();
        assert!(!is_in_legacy_dir(&safe_data));
        assert!(!is_in_legacy_dir(&safe_data.join("appload_state.db")));

        env::remove_var("APPLOAD_SANDBOX_ROOT");
    }

    #[test]
    #[should_panic(expected = "TEST ISOLATION VIOLATION")]
    fn test_paths_panics_in_test_without_sandbox() {
        let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        env::remove_var("APPLOAD_SANDBOX_ROOT");
        let _ = data_dir();
    }
}
