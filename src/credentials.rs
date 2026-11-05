#![allow(unsafe_code)] // Windows Credential Manager Win32 FFI requires unsafe blocks for CredReadW/CredWriteW/CredDeleteW

use std::env;
use std::fs;
use std::io::{self, Write};

pub const ENV_ASKPASS_MODE: &str = "SHIPLY_ASKPASS_MODE";
pub const ENV_ASKPASS_FILE: &str = "SHIPLY_ASKPASS_FILE";
pub const ENV_SSH_PASSWORD: &str = "SHIPLY_SSH_PASSWORD";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCredentials {
    pub username: String,
    pub password: Option<String>,
    pub source_description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultCredential {
    pub username: Option<String>,
    pub password: String,
}

/// If the current process was launched by `ssh`/`scp` as `SSH_ASKPASS`,
/// prints the password to stdout and returns `true`.
pub fn try_handle_askpass_invocation() -> bool {
    if env::var_os(ENV_ASKPASS_MODE).is_none() {
        return false;
    }

    if let Some(secret) = read_askpass_secret_from_env() {
        let mut out = io::stdout().lock();
        let _ = out.write_all(secret.as_bytes());
        let _ = out.flush();
    }
    true
}

pub fn read_askpass_secret_from_env() -> Option<String> {
    if let Ok(path) = env::var(ENV_ASKPASS_FILE) {
        if !path.is_empty() {
            if let Ok(contents) = fs::read_to_string(&path) {
                return Some(contents);
            }
        }
    }
    if let Ok(pw) = env::var(ENV_SSH_PASSWORD) {
        if !pw.is_empty() {
            return Some(pw);
        }
    }
    None
}

/// Decodes a Windows Credential Manager `CredentialBlob` which may be stored
/// either as UTF-16LE (standard Windows Generic Credentials) or UTF-8.
pub fn decode_credential_blob(blob: &[u8]) -> String {
    if blob.is_empty() {
        return String::new();
    }

    if blob.len().is_multiple_of(2) && (blob.contains(&0) || std::str::from_utf8(blob).is_err()) {
        let units: Vec<u16> = blob
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&chunk| u16::from_le_bytes(chunk))
            .collect();
        return String::from_utf16_lossy(&units)
            .trim_end_matches('\0')
            .to_string();
    }

    String::from_utf8_lossy(blob)
        .trim_end_matches('\0')
        .to_string()
}

/// Reads a stored credential from the OS native credential store:
/// - Windows: Windows Credential Manager (`CredReadW` Generic Credential, e.g., `indyserver`)
/// - macOS: Keychain (`security find-generic-password`)
/// - Linux: Secret Service (`secret-tool lookup`)
pub fn read_os_vault_credential(target: &str) -> Option<VaultCredential> {
    let target = target.trim();
    if target.is_empty() {
        return None;
    }

    #[cfg(windows)]
    {
        read_windows_generic_credential(target)
    }

    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("security")
            .args(["find-generic-password", "-s", target, "-w"])
            .output()
            .ok()?;
        if output.status.success() {
            let pw = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !pw.is_empty() {
                return Some(VaultCredential {
                    username: None,
                    password: pw,
                });
            }
        }
        None
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        let output = std::process::Command::new("secret-tool")
            .args(["lookup", "service", target])
            .output()
            .ok()?;
        if output.status.success() {
            let pw = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !pw.is_empty() {
                return Some(VaultCredential {
                    username: None,
                    password: pw,
                });
            }
        }
        None
    }
}

#[cfg(windows)]
fn read_windows_generic_credential(target: &str) -> Option<VaultCredential> {
    use std::ptr;
    use windows_sys::Win32::Security::Credentials::{
        CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
    };

    let mut wide_target: Vec<u16> = target.encode_utf16().collect();
    wide_target.push(0);

    let mut cred_ptr: *mut CREDENTIALW = ptr::null_mut();
    // SAFETY: `wide_target` is a valid null-terminated UTF-16 string and `cred_ptr` receives the allocated CREDENTIALW pointer.
    let ok = unsafe { CredReadW(wide_target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut cred_ptr) };

    if ok == 0 || cred_ptr.is_null() {
        return None;
    }

    // SAFETY: `cred_ptr` was allocated by `CredReadW` and is freed via `CredFree` before returning.
    unsafe {
        let cred = &*cred_ptr;
        let username = if !cred.UserName.is_null() {
            let mut len = 0usize;
            while *cred.UserName.add(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(cred.UserName, len);
            let u = String::from_utf16_lossy(slice).trim().to_string();
            if u.is_empty() {
                None
            } else {
                Some(u)
            }
        } else {
            None
        };

        let password = if !cred.CredentialBlob.is_null() && cred.CredentialBlobSize > 0 {
            let blob =
                std::slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize);
            decode_credential_blob(blob)
        } else {
            String::new()
        };

        CredFree(cred_ptr as *const _);

        if password.is_empty() {
            None
        } else {
            Some(VaultCredential { username, password })
        }
    }
}

/// Writes or updates a secret credential directly in the OS native credential store:
/// - Windows: Windows Credential Manager (`CredWriteW` Generic Credential, `CRED_PERSIST_LOCAL_MACHINE`)
/// - macOS: Keychain (`security add-generic-password -U`)
/// - Linux: Secret Service (`secret-tool store`)
pub fn write_os_vault_credential(
    target: &str,
    username: Option<&str>,
    secret_value: &str,
) -> Result<(), String> {
    let target = target.trim();
    if target.is_empty() {
        return Err("Secret name / target key cannot be empty.".to_string());
    }
    if secret_value.is_empty() {
        return Err("Secret value / password cannot be empty.".to_string());
    }

    #[cfg(windows)]
    {
        write_windows_generic_credential(target, username, secret_value)
    }

    #[cfg(target_os = "macos")]
    {
        let account = username
            .map(str::trim)
            .filter(|u| !u.is_empty())
            .unwrap_or("shiply");
        let output = std::process::Command::new("security")
            .args([
                "add-generic-password",
                "-U",
                "-s",
                target,
                "-a",
                account,
                "-w",
                secret_value,
            ])
            .output()
            .map_err(|e| format!("Failed to invoke macOS security tool: {e}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "macOS Keychain write failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        use std::process::Stdio;
        let account = username
            .map(str::trim)
            .filter(|u| !u.is_empty())
            .unwrap_or("shiply");
        let mut child = std::process::Command::new("secret-tool")
            .args([
                "store",
                &format!("--label=Shiply ({target})"),
                "service",
                target,
                "username",
                account,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to invoke secret-tool: {e}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(secret_value.as_bytes());
        }
        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed waiting for secret-tool: {e}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "Linux Secret Service write failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }
}

#[cfg(windows)]
fn write_windows_generic_credential(
    target: &str,
    username: Option<&str>,
    secret_value: &str,
) -> Result<(), String> {
    use std::ptr;
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Security::Credentials::{
        CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
    };

    let mut wide_target: Vec<u16> = target.encode_utf16().collect();
    wide_target.push(0);

    let clean_user = username.map(str::trim).filter(|u| !u.is_empty());
    let mut wide_user: Vec<u16> = clean_user
        .map(|u| {
            let mut v: Vec<u16> = u.encode_utf16().collect();
            v.push(0);
            v
        })
        .unwrap_or_default();

    let mut utf16_secret_blob: Vec<u8> = secret_value
        .encode_utf16()
        .flat_map(|u| u.to_le_bytes())
        .collect();

    let cred = CREDENTIALW {
        Flags: 0,
        Type: CRED_TYPE_GENERIC,
        TargetName: wide_target.as_mut_ptr(),
        Comment: ptr::null_mut(),
        LastWritten: unsafe { std::mem::zeroed() },
        CredentialBlobSize: utf16_secret_blob.len() as u32,
        CredentialBlob: utf16_secret_blob.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        AttributeCount: 0,
        Attributes: ptr::null_mut(),
        TargetAlias: ptr::null_mut(),
        UserName: if wide_user.is_empty() {
            ptr::null_mut()
        } else {
            wide_user.as_mut_ptr()
        },
    };

    // SAFETY: `cred` pointers (`wide_target`, `wide_user`, `utf16_secret_blob`) remain valid for the duration of `CredWriteW`.
    let ok = unsafe { CredWriteW(&cred, 0) };
    if ok == 0 {
        let err_code = unsafe { GetLastError() };
        return Err(format!(
            "Windows Credential Manager failed to write '{target}' (Win32 error {err_code})."
        ));
    }
    Ok(())
}

/// Deletes a stored credential from the OS native credential store.
pub fn delete_os_vault_credential(target: &str) -> Result<(), String> {
    let target = target.trim();
    if target.is_empty() {
        return Err("Secret name / target key cannot be empty.".to_string());
    }

    #[cfg(windows)]
    {
        delete_windows_generic_credential(target)
    }

    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("security")
            .args(["delete-generic-password", "-s", target])
            .output();
        Ok(())
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("secret-tool")
            .args(["clear", "service", target])
            .output();
        Ok(())
    }
}

#[cfg(windows)]
fn delete_windows_generic_credential(target: &str) -> Result<(), String> {
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_NOT_FOUND};
    use windows_sys::Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC};

    let mut wide_target: Vec<u16> = target.encode_utf16().collect();
    wide_target.push(0);

    // SAFETY: `wide_target` is a valid null-terminated UTF-16 string.
    let ok = unsafe { CredDeleteW(wide_target.as_ptr(), CRED_TYPE_GENERIC, 0) };
    if ok == 0 {
        let err_code = unsafe { GetLastError() };
        if err_code == ERROR_NOT_FOUND {
            return Ok(());
        }
        return Err(format!(
            "Windows Credential Manager failed to delete '{target}' (Win32 error {err_code})."
        ));
    }
    Ok(())
}

/// Interpolates GitHub Actions-style secret references (`${{ secrets.KEY }}`, `${{ secrets.KEY.username }}`,
/// `{secret:KEY}`, `{secret_user:KEY}`) from the OS Credential Manager and returns both the interpolated
/// string and the list of sensitive secret values that must be redacted from logs.
pub fn interpolate_os_secrets(input: &str) -> (String, Vec<String>) {
    let mut output = input.to_string();
    let mut resolved_secrets = Vec::new();

    // 1. Handle `${{ secrets.KEY }}` and `${{ secrets.KEY.username }}`
    while let Some(start) = output.find("${{") {
        let Some(rel_end) = output[start..].find("}}") else {
            break;
        };
        let end = start + rel_end + 2;
        let inner = output[start + 3..start + rel_end].trim();
        if let Some(rest) = inner.strip_prefix("secrets.") {
            let rest = rest.trim();
            let (key, want_username) = if let Some(k) = rest.strip_suffix(".username") {
                (k.trim(), true)
            } else if let Some(k) = rest.strip_suffix(".user") {
                (k.trim(), true)
            } else if let Some(k) = rest.strip_suffix(".password") {
                (k.trim(), false)
            } else {
                (rest, false)
            };

            let replacement = if let Some(cred) = read_os_vault_credential(key) {
                if !cred.password.is_empty() && !resolved_secrets.contains(&cred.password) {
                    resolved_secrets.push(cred.password.clone());
                }
                if want_username {
                    cred.username.unwrap_or_default()
                } else {
                    cred.password
                }
            } else {
                String::new()
            };
            output.replace_range(start..end, &replacement);
        } else {
            break;
        }
    }

    // 2. Handle `{secret:KEY}` and `{secret_user:KEY}`
    for (prefix, want_username) in [("{secret:", false), ("{secret_user:", true)] {
        while let Some(start) = output.find(prefix) {
            let Some(rel_end) = output[start..].find('}') else {
                break;
            };
            let end = start + rel_end + 1;
            let key = output[start + prefix.len()..start + rel_end].trim();
            let replacement = if let Some(cred) = read_os_vault_credential(key) {
                if !cred.password.is_empty() && !resolved_secrets.contains(&cred.password) {
                    resolved_secrets.push(cred.password.clone());
                }
                if want_username {
                    cred.username.unwrap_or_default()
                } else {
                    cred.password
                }
            } else {
                String::new()
            };
            output.replace_range(start..end, &replacement);
        }
    }

    (output, resolved_secrets)
}

/// Resolves the SSH username and password by checking:
/// 1. Explicit UI password override (if entered by user)
/// 2. OS Vault / Credential Manager (`vault_target`, e.g., `indyserver`)
/// 3. Fallback to configured username with SSH Key authentication (`password: None`)
pub fn resolve_credentials(
    configured_user: &str,
    vault_target: &str,
    manual_password_override: Option<&str>,
) -> ResolvedCredentials {
    let default_user = if configured_user.trim().is_empty() {
        "appndq".to_string()
    } else {
        configured_user.trim().to_string()
    };

    if let Some(pw) = manual_password_override {
        if !pw.is_empty() {
            return ResolvedCredentials {
                username: default_user,
                password: Some(pw.to_string()),
                source_description: "Manual UI Password".to_string(),
            };
        }
    }

    if let Some(vault_cred) = read_os_vault_credential(vault_target) {
        let username = vault_cred.username.unwrap_or(default_user);
        return ResolvedCredentials {
            username: username.clone(),
            password: Some(vault_cred.password),
            source_description: format!("OS Credential Store ('{vault_target}' as '{username}')"),
        };
    }

    ResolvedCredentials {
        username: default_user,
        password: None,
        source_description: "SSH Key / Agent (No stored password found)".to_string(),
    }
}

/// Redacts all occurrences of sensitive secret strings from a given text string.
/// Replaces every occurrence with `[REDACTED]`.
pub fn redact_secrets(text: &str, secrets: &[&str]) -> String {
    let mut sanitized = text.to_string();
    for secret in secrets {
        let trimmed = secret.trim();
        // Ignore single-character or trivial substrings to avoid over-redaction
        if trimmed.len() >= 2 {
            sanitized = sanitized.replace(trimmed, "[REDACTED]");
        }
    }
    sanitized
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_decode_utf16le_credential_blob() {
        let original = "MySecretP@ss123!";
        let utf16_bytes: Vec<u8> = original
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        assert_eq!(decode_credential_blob(&utf16_bytes), original);
    }

    #[test]
    fn test_decode_utf8_credential_blob() {
        let original = "Utf8SecretPass99";
        assert_eq!(decode_credential_blob(original.as_bytes()), original);
    }

    #[test]
    fn test_resolve_credentials_with_manual_override() {
        let resolved = resolve_credentials("appndq", "nonexistent_vault_xyz", Some("secret123"));
        assert_eq!(resolved.username, "appndq");
        assert_eq!(resolved.password.as_deref(), Some("secret123"));
        assert_eq!(resolved.source_description, "Manual UI Password");
    }

    #[test]
    fn test_read_askpass_secret_from_file() {
        let mut tmp = NamedTempFile::new().expect("temp file");
        write!(tmp, "s3cr3t_pw").expect("write pw");
        env::set_var(ENV_ASKPASS_FILE, tmp.path().as_os_str());
        let read_pw = read_askpass_secret_from_env();
        env::remove_var(ENV_ASKPASS_FILE);
        assert_eq!(read_pw.as_deref(), Some("s3cr3t_pw"));
    }

    #[test]
    fn test_redact_secrets_from_command_output() {
        let text = "Connecting to user@10.0.0.1 with password SuperSecretPassword123! in /var/app";
        let redacted = redact_secrets(text, &["SuperSecretPassword123!"]);
        assert_eq!(
            redacted,
            "Connecting to user@10.0.0.1 with password [REDACTED] in /var/app"
        );
        assert!(!redacted.contains("SuperSecretPassword123!"));
    }

    #[test]
    fn test_write_read_delete_os_vault_credential_and_secret_interpolation() {
        let target = format!("shiply_test_secret_{}", uuid::Uuid::new_v4().simple());
        write_os_vault_credential(&target, Some("deploy_admin"), "MyVaultS3cret!99")
            .expect("write credential to OS vault");

        let loaded = read_os_vault_credential(&target).expect("read credential from OS vault");
        assert_eq!(loaded.username.as_deref(), Some("deploy_admin"));
        assert_eq!(loaded.password, "MyVaultS3cret!99");

        let cmd_template = format!(
            "docker login -u ${{{{ secrets.{target}.username }}}} -p ${{{{ secrets.{target} }}}}"
        );
        let (interpolated, redact_list) = interpolate_os_secrets(&cmd_template);
        assert_eq!(
            interpolated,
            "docker login -u deploy_admin -p MyVaultS3cret!99"
        );
        assert!(redact_list.contains(&"MyVaultS3cret!99".to_string()));

        let refs: Vec<&str> = redact_list.iter().map(String::as_str).collect();
        let sanitized = redact_secrets(&interpolated, &refs);
        assert_eq!(sanitized, "docker login -u deploy_admin -p [REDACTED]");

        delete_os_vault_credential(&target).expect("delete credential from OS vault");
        assert!(read_os_vault_credential(&target).is_none());
    }
}
