//! Windows single-instance process guard (`Local\AppLoad.SingleInstance`).
//!
//! Ensures two `appload.exe` GUI processes cannot run simultaneously against the same
//! SQLite database (`appload_state.db`). When a second instance is launched, it focuses
//! the existing AppLoad window and exits cleanly.

#![allow(unsafe_code)]

use std::sync::Mutex;

pub const DEFAULT_MUTEX_NAME: &str = r"Local\AppLoad.SingleInstance";

static ACTIVE_MUTEX_HANDLE: Mutex<Option<usize>> = Mutex::new(None);

/// Computes the named mutex identifier for the current environment.
///
/// - In normal operation (no `APPLOAD_SANDBOX_ROOT`): `Local\AppLoad.SingleInstance`
/// - In sandbox mode (`APPLOAD_SANDBOX_ROOT` set): `Local\AppLoad.SingleInstance.Sandbox.<hash>`
///   so two processes launched inside the same sandbox share the mutex while remaining
///   isolated from the host desktop session and other test sandboxes.
pub fn resolve_mutex_name() -> String {
    if let Some(sandbox) = crate::paths::sandbox_root() {
        let norm = sandbox
            .to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase();
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in norm.bytes() {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!(r"{DEFAULT_MUTEX_NAME}.Sandbox.{hash:016x}")
    } else {
        DEFAULT_MUTEX_NAME.to_string()
    }
}

/// RAII guard holding the OS single-instance mutex for the lifetime of the GUI process.
pub struct SingleInstanceGuard {
    #[cfg(windows)]
    handle: usize,
    #[cfg(not(windows))]
    _private: (),
}

impl SingleInstanceGuard {
    /// Attempts to acquire the process-wide single-instance guard for the current environment.
    /// If another instance already holds the mutex, focuses the existing AppLoad window and returns `None`.
    pub fn try_acquire() -> Option<Self> {
        let name = resolve_mutex_name();
        let guard = Self::try_acquire_named(&name)?;
        #[cfg(windows)]
        {
            if let Ok(mut slot) = ACTIVE_MUTEX_HANDLE.lock() {
                *slot = Some(guard.handle);
            }
        }
        Some(guard)
    }

    /// Attempts to acquire a named Windows mutex directly.
    /// Returns `None` if another handle already owns/created `mutex_name`.
    pub fn try_acquire_named(mutex_name: &str) -> Option<Self> {
        #[cfg(windows)]
        {
            acquire_windows_mutex(mutex_name)
        }
        #[cfg(not(windows))]
        {
            let _ = mutex_name;
            Some(Self { _private: () })
        }
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            if self.handle != 0 {
                if let Ok(mut slot) = ACTIVE_MUTEX_HANDLE.lock() {
                    if *slot == Some(self.handle) {
                        *slot = None;
                    }
                }
                close_windows_mutex(self.handle);
                self.handle = 0;
            }
        }
    }
}

/// Releases the active single-instance mutex early (e.g. immediately before spawning a restarted binary on update).
pub fn release_active_guard() {
    #[cfg(windows)]
    {
        if let Ok(mut slot) = ACTIVE_MUTEX_HANDLE.lock() {
            if let Some(h) = slot.take() {
                close_windows_mutex(h);
            }
        }
    }
}

/// Returns the process IDs of any other running AppLoad or Shiply processes.
///
/// - In normal production mode: inspects system processes matching `appload.exe` or `shiply.exe`
///   (excluding the current process PID).
/// - In sandbox mode (`APPLOAD_SANDBOX_ROOT` set): ONLY matches processes whose executable path
///   is located inside the active sandbox root. This guarantees sandboxed test runs never
///   inspect, wait on, or interfere with any real AppLoad instance running on the host system.
pub fn find_other_running_instances() -> Vec<u32> {
    #[cfg(windows)]
    {
        find_windows_running_instances()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// Waits for a specific process ID to terminate, up to `timeout`.
/// Returns `true` if the process has exited (or never existed), `false` if it timed out.
pub fn wait_for_process_exit(pid: u32, timeout: std::time::Duration) -> bool {
    #[cfg(windows)]
    {
        wait_windows_process_exit(pid, timeout)
    }
    #[cfg(not(windows))]
    {
        let _ = (pid, timeout);
        true
    }
}

/// Waits for all other running AppLoad / Shiply instances to exit, up to `timeout`.
/// Returns `true` if all other instances have terminated, `false` if one or more are still running.
pub fn wait_for_other_instances_to_exit(timeout: std::time::Duration) -> bool {
    let other_pids = find_other_running_instances();
    if other_pids.is_empty() {
        return true;
    }
    let start = std::time::Instant::now();
    for pid in other_pids {
        let remaining = timeout.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            return false;
        }
        if !wait_for_process_exit(pid, remaining) {
            return false;
        }
    }
    find_other_running_instances().is_empty()
}

/// Brings an existing running AppLoad window to the foreground if found.
/// In sandbox mode (`APPLOAD_SANDBOX_ROOT` set), this is a no-op so sandboxed runs never
/// focus or interact with a live installed AppLoad window on the host desktop.
pub fn focus_existing_appload_window() -> bool {
    if crate::paths::sandbox_root().is_some() {
        return false;
    }
    #[cfg(windows)]
    {
        focus_windows_appload_window()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn CreateMutexW(
        lpMutexAttributes: *const std::ffi::c_void,
        bInitialOwner: i32,
        lpName: *const u16,
    ) -> *mut std::ffi::c_void;
    fn ReleaseMutex(hMutex: *mut std::ffi::c_void) -> i32;
}

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn EnumWindows(
        lpEnumFunc: unsafe extern "system" fn(*mut std::ffi::c_void, isize) -> i32,
        lParam: isize,
    ) -> i32;
    fn GetWindowTextW(hWnd: *mut std::ffi::c_void, lpString: *mut u16, nMaxCount: i32) -> i32;
    fn IsWindowVisible(hWnd: *mut std::ffi::c_void) -> i32;
    fn IsIconic(hWnd: *mut std::ffi::c_void) -> i32;
    fn ShowWindow(hWnd: *mut std::ffi::c_void, nCmdShow: i32) -> i32;
    fn SetForegroundWindow(hWnd: *mut std::ffi::c_void) -> i32;
}

#[cfg(windows)]
fn acquire_windows_mutex(mutex_name: &str) -> Option<SingleInstanceGuard> {
    use std::ptr;
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS,
    };

    let mut wide_name: Vec<u16> = mutex_name.encode_utf16().collect();
    wide_name.push(0);

    // SAFETY: `wide_name` is a valid null-terminated UTF-16 string.
    let handle = unsafe { CreateMutexW(ptr::null(), 1, wide_name.as_ptr()) };
    if handle.is_null() {
        let err = unsafe { GetLastError() };
        if err == ERROR_ACCESS_DENIED {
            return None;
        }
        return None;
    }

    let last_err = unsafe { GetLastError() };
    if last_err == ERROR_ALREADY_EXISTS {
        unsafe {
            CloseHandle(handle);
        }
        return None;
    }

    Some(SingleInstanceGuard {
        handle: handle as usize,
    })
}

#[cfg(windows)]
fn close_windows_mutex(handle: usize) {
    use windows_sys::Win32::Foundation::CloseHandle;
    if handle != 0 {
        let raw = handle as *mut std::ffi::c_void;
        unsafe {
            let _ = ReleaseMutex(raw);
            let _ = CloseHandle(raw);
        }
    }
}

#[cfg(windows)]
unsafe extern "system" fn enum_windows_proc(hwnd: *mut std::ffi::c_void, lparam: isize) -> i32 {
    if IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    let mut buf = [0u16; 256];
    let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    if len <= 0 {
        return 1;
    }
    let title = String::from_utf16_lossy(&buf[..len as usize]);
    if (title == "AppLoad" || title.starts_with("AppLoad v")) && !title.ends_with("Setup") {
        const SW_RESTORE: i32 = 9;
        if IsIconic(hwnd) != 0 {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = SetForegroundWindow(hwnd);
        let found_ptr = lparam as *mut bool;
        if !found_ptr.is_null() {
            *found_ptr = true;
        }
        return 0;
    }
    1
}

#[cfg(windows)]
fn focus_windows_appload_window() -> bool {
    let mut found = false;
    unsafe {
        let _ = EnumWindows(enum_windows_proc, (&mut found as *mut bool) as isize);
    }
    found
}

#[cfg(windows)]
fn find_windows_running_instances() -> Vec<u32> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let current_pid = std::process::id();
    let sandbox_opt = crate::paths::sandbox_root();
    let mut result = Vec::new();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return result;
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let pid = entry.th32ProcessID;
                if pid != current_pid && pid != 0 {
                    let name_len = entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len());
                    let exe_name = String::from_utf16_lossy(&entry.szExeFile[..name_len]);

                    if exe_name.eq_ignore_ascii_case("appload.exe")
                        || exe_name.eq_ignore_ascii_case("shiply.exe")
                    {
                        if let Some(ref sb) = sandbox_opt {
                            if let Some(proc_path) = query_windows_process_image_path(pid) {
                                if is_path_in_sandbox_dir(&proc_path, sb) {
                                    result.push(pid);
                                }
                            }
                        } else {
                            result.push(pid);
                        }
                    }
                }

                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);
    }

    result
}

#[cfg(windows)]
fn query_windows_process_image_path(pid: u32) -> Option<std::path::PathBuf> {
    use windows_sys::Win32::Foundation::{CloseHandle, FALSE};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
        if handle.is_null() {
            return None;
        }
        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        let success = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
        CloseHandle(handle);
        if success != 0 && size > 0 {
            let path_str = String::from_utf16_lossy(&buf[..size as usize]);
            Some(std::path::PathBuf::from(path_str))
        } else {
            None
        }
    }
}

#[cfg(windows)]
fn is_path_in_sandbox_dir(proc_path: &std::path::Path, sandbox_root: &std::path::Path) -> bool {
    let norm_proc = proc_path
        .to_string_lossy()
        .replace('/', "\\")
        .to_lowercase();
    let norm_sb = sandbox_root
        .to_string_lossy()
        .replace('/', "\\")
        .to_lowercase();
    norm_proc.starts_with(norm_sb.trim_end_matches('\\'))
}

#[cfg(windows)]
fn wait_windows_process_exit(pid: u32, timeout: std::time::Duration) -> bool {
    use std::time::Instant;
    use windows_sys::Win32::Foundation::{CloseHandle, FALSE, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject};

    const SYNCHRONIZE: u32 = 0x0010_0000;

    let start = Instant::now();
    unsafe {
        let handle = OpenProcess(SYNCHRONIZE, FALSE, pid);
        if handle.is_null() {
            return !is_windows_process_alive(pid);
        }

        let remaining = timeout.saturating_sub(start.elapsed());
        let millis = remaining.as_millis().min(u32::MAX as u128) as u32;
        let res = WaitForSingleObject(handle, millis);
        CloseHandle(handle);

        res == WAIT_OBJECT_0
    }
}

#[cfg(windows)]
fn is_windows_process_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, FALSE};
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
        if !handle.is_null() {
            CloseHandle(handle);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_instance_mutex_blocks_second_acquire_until_dropped() {
        let name = format!(
            r"Local\AppLoad.SingleInstance.UnitTest.{}",
            uuid::Uuid::new_v4().simple()
        );

        let guard1 =
            SingleInstanceGuard::try_acquire_named(&name).expect("first instance must acquire");
        let guard2 = SingleInstanceGuard::try_acquire_named(&name);
        assert!(
            guard2.is_none(),
            "second instance with same mutex name must be blocked while first is alive"
        );

        drop(guard1);

        let guard3 = SingleInstanceGuard::try_acquire_named(&name);
        assert!(
            guard3.is_some(),
            "new instance must succeed after previous guard is dropped"
        );
    }

    #[test]
    fn test_sandbox_mutex_name_isolated_from_production_and_other_sandboxes() {
        let sb_a = crate::paths::TestSandbox::new();
        let name_a = resolve_mutex_name();
        assert!(
            name_a.starts_with("Local\\AppLoad.SingleInstance.Sandbox."),
            "sandbox mutex name must include sandbox hash: {name_a}"
        );
        assert_ne!(
            name_a, DEFAULT_MUTEX_NAME,
            "sandbox mutex name must never collide with production mutex"
        );

        let guard_a1 = SingleInstanceGuard::try_acquire().expect("acquire in sandbox A");
        assert!(
            SingleInstanceGuard::try_acquire().is_none(),
            "second instance in same sandbox A must be blocked"
        );
        drop(guard_a1);
        drop(sb_a);

        let _sb_b = crate::paths::TestSandbox::new();
        let name_b = resolve_mutex_name();
        assert_ne!(
            name_a, name_b,
            "different sandboxes must resolve to distinct mutex hashes"
        );
    }
}
