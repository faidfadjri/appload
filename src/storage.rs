//! SQLite and JSON storage persistence layer for projects, settings, and history.

use crate::credentials::read_os_vault_credential;
use crate::domain::{DeploymentRecord, DeploymentStatus, Project, SecretEntry, StepExecutionLog};
use crate::ps1_importer::import_project_from_path;
use crate::version::{bump_semver, VersionBump};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppStorage {
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default)]
    pub deployments: Vec<DeploymentRecord>,
    #[serde(default)]
    pub secrets: Vec<SecretEntry>,
    #[serde(skip)]
    storage_path: Option<PathBuf>,
    #[serde(skip)]
    last_state_mtime: Option<SystemTime>,
    #[serde(skip)]
    imported_files_mtime: HashMap<PathBuf, SystemTime>,
    #[serde(skip)]
    pub migration_notice: Option<String>,
}

#[derive(Deserialize)]
struct LegacyJsonState {
    #[serde(default)]
    projects: Vec<Project>,
    #[serde(default)]
    deployments: Vec<DeploymentRecord>,
    #[serde(default)]
    secrets: Vec<SecretEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProjectSortMode {
    #[default]
    Custom,
    NameAsc,
    NameDesc,
    RecentDeploy,
}

/// SQLite database initialization and schema creation.
pub fn init_db(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;

         CREATE TABLE IF NOT EXISTS metadata (
             key TEXT PRIMARY KEY,
             value TEXT NOT NULL
         );

         CREATE TABLE IF NOT EXISTS projects (
             id TEXT PRIMARY KEY,
             name TEXT NOT NULL,
             description TEXT NOT NULL,
             local_path TEXT NOT NULL,
             environments_json TEXT NOT NULL,
             group_name TEXT,
             sort_order INTEGER
         );

         CREATE TABLE IF NOT EXISTS deployments (
             id TEXT PRIMARY KEY,
             project_id TEXT NOT NULL,
             project_name TEXT NOT NULL,
             environment TEXT NOT NULL,
             app_name TEXT NOT NULL,
             version TEXT NOT NULL,
             git_branch TEXT,
             git_commit TEXT,
             git_changes TEXT NOT NULL,
             is_rollback INTEGER NOT NULL,
             servers TEXT NOT NULL,
             status TEXT NOT NULL,
             started_at TEXT NOT NULL,
             finished_at TEXT,
             duration_ms INTEGER,
             step_logs TEXT NOT NULL,
             executed_steps TEXT NOT NULL
         );

         CREATE INDEX IF NOT EXISTS idx_deployments_project ON deployments (project_id);
         CREATE INDEX IF NOT EXISTS idx_deployments_env ON deployments (project_id, environment);
         CREATE INDEX IF NOT EXISTS idx_deployments_started_at ON deployments (started_at DESC);

         CREATE TABLE IF NOT EXISTS secrets (
             key TEXT PRIMARY KEY,
             username TEXT,
             description TEXT NOT NULL,
             updated_at TEXT NOT NULL
         );",
    )?;

    // Backwards-compatible migrations for projects table columns if created in older versions
    let _ = conn.execute("ALTER TABLE projects ADD COLUMN group_name TEXT", []);
    let _ = conn.execute("ALTER TABLE projects ADD COLUMN sort_order INTEGER", []);

    Ok(())
}

pub fn load_projects_from_db(conn: &Connection) -> Result<Vec<Project>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, local_path, environments_json, group_name, sort_order
         FROM projects
         ORDER BY COALESCE(sort_order, 999999) ASC, name ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let description: String = row.get(2)?;
        let local_path: String = row.get(3)?;
        let environments_json: String = row.get(4)?;
        let group: Option<String> = row.get(5)?;
        let sort_order: Option<i32> = row.get(6)?;
        let environments = serde_json::from_str(&environments_json).unwrap_or_default();
        Ok(Project {
            id,
            name,
            description,
            local_path,
            environments,
            group,
            sort_order,
        })
    })?;

    let mut projects = Vec::new();
    for p in rows {
        projects.push(p?);
    }
    Ok(projects)
}

pub fn load_deployments_from_db(
    conn: &Connection,
) -> Result<Vec<DeploymentRecord>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, project_name, environment, app_name, version, git_branch, git_commit, git_changes, is_rollback, servers, status, started_at, finished_at, duration_ms, step_logs, executed_steps
         FROM deployments
         ORDER BY started_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let project_id: String = row.get(1)?;
        let project_name: String = row.get(2)?;
        let environment: String = row.get(3)?;
        let app_name: String = row.get(4)?;
        let version: String = row.get(5)?;
        let git_branch: Option<String> = row.get(6)?;
        let git_commit: Option<String> = row.get(7)?;
        let git_changes_json: String = row.get(8)?;
        let is_rollback_int: i64 = row.get(9)?;
        let servers_json: String = row.get(10)?;
        let status_str: String = row.get(11)?;
        let started_at_str: String = row.get(12)?;
        let finished_at_str: Option<String> = row.get(13)?;
        let duration_ms_int: Option<i64> = row.get(14)?;
        let step_logs_json: String = row.get(15)?;
        let executed_steps_json: String = row.get(16)?;

        let git_changes = serde_json::from_str(&git_changes_json).unwrap_or_default();
        let servers = serde_json::from_str(&servers_json).unwrap_or_default();
        let step_logs = serde_json::from_str(&step_logs_json).unwrap_or_default();
        let executed_steps = serde_json::from_str(&executed_steps_json).unwrap_or_default();
        let is_rollback = is_rollback_int != 0;
        let status = match status_str.as_str() {
            "Running" => DeploymentStatus::Running,
            "Succeeded" => DeploymentStatus::Succeeded,
            "Cancelled" => DeploymentStatus::Cancelled,
            _ => DeploymentStatus::Failed,
        };
        let started_at = chrono::DateTime::parse_from_rfc3339(&started_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        let finished_at = finished_at_str.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        });
        let duration_ms = duration_ms_int.map(|v| v as u64);

        Ok(DeploymentRecord {
            id,
            project_id,
            project_name,
            environment,
            app_name,
            version,
            git_branch,
            git_commit,
            git_changes,
            is_rollback,
            servers,
            status,
            started_at,
            finished_at,
            duration_ms,
            step_logs,
            executed_steps,
        })
    })?;

    let mut deployments = Vec::new();
    for d in rows {
        deployments.push(d?);
    }
    Ok(deployments)
}

pub fn load_secrets_from_db(conn: &Connection) -> Result<Vec<SecretEntry>, rusqlite::Error> {
    init_db(conn)?;
    let mut stmt = conn.prepare(
        "SELECT key, username, description, updated_at
         FROM secrets
         ORDER BY key ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        let key: String = row.get(0)?;
        let username: Option<String> = row.get(1)?;
        let description: String = row.get(2)?;
        let updated_at_str: String = row.get(3)?;
        let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        Ok(SecretEntry {
            key,
            username,
            description,
            updated_at,
        })
    })?;

    let mut secrets = Vec::new();
    for s in rows {
        secrets.push(s?);
    }
    Ok(secrets)
}

pub fn save_secrets_to_db(
    conn: &mut Connection,
    secrets: &[SecretEntry],
) -> Result<(), rusqlite::Error> {
    init_db(conn)?;
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM secrets", [])?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO secrets (key, username, description, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
        )?;
        for s in secrets {
            stmt.execute(params![
                s.key,
                s.username,
                s.description,
                s.updated_at.to_rfc3339()
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn save_all_to_db(
    conn: &mut Connection,
    projects: &[Project],
    deployments: &[DeploymentRecord],
) -> Result<(), rusqlite::Error> {
    init_db(conn)?;
    let tx = conn.transaction()?;

    tx.execute("DELETE FROM projects", [])?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO projects (id, name, description, local_path, environments_json, group_name, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for (idx, proj) in projects.iter().enumerate() {
            let envs_json = serde_json::to_string(&proj.environments).unwrap_or_default();
            let effective_order = proj.sort_order.unwrap_or(idx as i32);
            stmt.execute(params![
                proj.id,
                proj.name,
                proj.description,
                proj.local_path,
                envs_json,
                proj.group,
                effective_order,
            ])?;
        }
    }

    tx.execute("DELETE FROM deployments", [])?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO deployments (id, project_id, project_name, environment, app_name, version, git_branch, git_commit, git_changes, is_rollback, servers, status, started_at, finished_at, duration_ms, step_logs, executed_steps)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        )?;
        for dep in deployments {
            let git_changes_json = serde_json::to_string(&dep.git_changes).unwrap_or_default();
            let servers_json = serde_json::to_string(&dep.servers).unwrap_or_default();
            let step_logs_json = serde_json::to_string(&dep.step_logs).unwrap_or_default();
            let executed_steps_json =
                serde_json::to_string(&dep.executed_steps).unwrap_or_default();
            let status_str = match dep.status {
                DeploymentStatus::Running => "Running",
                DeploymentStatus::Succeeded => "Succeeded",
                DeploymentStatus::Cancelled => "Cancelled",
                DeploymentStatus::Failed => "Failed",
            };
            let started_at_str = dep.started_at.to_rfc3339();
            let finished_at_str = dep.finished_at.as_ref().map(|dt| dt.to_rfc3339());
            let is_rollback_int = if dep.is_rollback { 1i64 } else { 0i64 };
            let duration_ms_int = dep.duration_ms.map(|d| d as i64);

            stmt.execute(params![
                dep.id,
                dep.project_id,
                dep.project_name,
                dep.environment,
                dep.app_name,
                dep.version,
                dep.git_branch,
                dep.git_commit,
                git_changes_json,
                is_rollback_int,
                servers_json,
                status_str,
                started_at_str,
                finished_at_str,
                duration_ms_int,
                step_logs_json,
                executed_steps_json,
            ])?;
        }
    }

    tx.commit()?;
    Ok(())
}

pub fn upsert_deployment_to_db(
    conn: &Connection,
    dep: &DeploymentRecord,
) -> Result<(), rusqlite::Error> {
    init_db(conn)?;
    let git_changes_json = serde_json::to_string(&dep.git_changes).unwrap_or_default();
    let servers_json = serde_json::to_string(&dep.servers).unwrap_or_default();
    let step_logs_json = serde_json::to_string(&dep.step_logs).unwrap_or_default();
    let executed_steps_json = serde_json::to_string(&dep.executed_steps).unwrap_or_default();
    let status_str = match dep.status {
        DeploymentStatus::Running => "Running",
        DeploymentStatus::Succeeded => "Succeeded",
        DeploymentStatus::Cancelled => "Cancelled",
        DeploymentStatus::Failed => "Failed",
    };
    let started_at_str = dep.started_at.to_rfc3339();
    let finished_at_str = dep.finished_at.as_ref().map(|dt| dt.to_rfc3339());
    let is_rollback_int = if dep.is_rollback { 1i64 } else { 0i64 };
    let duration_ms_int = dep.duration_ms.map(|d| d as i64);

    conn.execute(
        "INSERT OR REPLACE INTO deployments (id, project_id, project_name, environment, app_name, version, git_branch, git_commit, git_changes, is_rollback, servers, status, started_at, finished_at, duration_ms, step_logs, executed_steps)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            dep.id,
            dep.project_id,
            dep.project_name,
            dep.environment,
            dep.app_name,
            dep.version,
            dep.git_branch,
            dep.git_commit,
            git_changes_json,
            is_rollback_int,
            servers_json,
            status_str,
            started_at_str,
            finished_at_str,
            duration_ms_int,
            step_logs_json,
            executed_steps_json,
        ],
    )?;
    Ok(())
}

impl AppStorage {
    /// Resolves `appload_state.db` strictly through `crate::paths::data_dir()`.
    /// Portable mode is only active if an explicit `appload.portable` marker file sits next to the exe.
    pub fn default_storage_path() -> PathBuf {
        crate::paths::data_dir().join("appload_state.db")
    }

    /// Returns the directory where `appload_state.db` (and any drop-in source files) live.
    pub fn app_data_dir(&self) -> PathBuf {
        self.storage_path
            .as_ref()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| {
                Self::default_storage_path()
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from("."))
            })
    }

    pub fn storage_file_path(&self) -> Option<&Path> {
        self.storage_path.as_deref()
    }

    pub fn reset_sync_cache(&mut self) {
        self.last_state_mtime = None;
        self.imported_files_mtime.clear();
    }

    /// Loads state from SQLite at `path` if it exists. If the database does not exist or is empty,
    /// seamlessly detects and migrates legacy JSON state files without any data loss.
    /// Also scans the app directory for any pasted `.json` or `.ps1` files.
    pub fn load_or_default(path: impl Into<PathBuf>) -> Self {
        let input_path = path.into();
        let is_json_ext = input_path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("json"));
        let db_path = if is_json_ext {
            input_path.with_extension("db")
        } else {
            input_path.clone()
        };

        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = fs::create_dir_all(parent);
            }
        }

        let mut state = AppStorage {
            projects: Vec::new(),
            deployments: Vec::new(),
            secrets: Vec::new(),
            storage_path: Some(db_path.clone()),
            last_state_mtime: None,
            imported_files_mtime: HashMap::new(),
            migration_notice: None,
        };

        let mut needs_save = false;

        if let Ok(mut conn) = Connection::open(&db_path) {
            let _ = init_db(&conn);
            let mut db_projects = load_projects_from_db(&conn).unwrap_or_default();
            let mut db_deployments = load_deployments_from_db(&conn).unwrap_or_default();
            let mut db_secrets = load_secrets_from_db(&conn).unwrap_or_default();

            // If SQLite is completely empty, check for legacy JSON state file to migrate seamlessly
            if db_projects.is_empty() && db_deployments.is_empty() {
                let mut legacy_candidates = Vec::new();
                if is_json_ext {
                    legacy_candidates.push(input_path);
                } else {
                    legacy_candidates.push(input_path.with_extension("json"));
                }
                if let Some(p) = db_path.parent() {
                    let parent_appload_json = p.join("appload_state.json");
                    let parent_shiply_json = p.join("shiply_state.json");
                    if !legacy_candidates.contains(&parent_appload_json) {
                        legacy_candidates.push(parent_appload_json);
                    }
                    if !legacy_candidates.contains(&parent_shiply_json) {
                        legacy_candidates.push(parent_shiply_json);
                    }
                }
                // If using the default storage path, also check default global locations
                let is_default = db_path == Self::default_storage_path();
                if is_default {
                    let data_appload_json = crate::paths::data_dir().join("appload_state.json");
                    if !legacy_candidates.contains(&data_appload_json) {
                        legacy_candidates.push(data_appload_json);
                    }
                    for cand in crate::paths::legacy_data_candidates() {
                        if cand
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|e| e.eq_ignore_ascii_case("json"))
                            .unwrap_or(false)
                            && !legacy_candidates.contains(&cand)
                        {
                            legacy_candidates.push(cand);
                        }
                    }
                }

                for candidate in legacy_candidates {
                    if candidate.exists() {
                        if let Ok(raw) = fs::read_to_string(&candidate) {
                            if let Ok(parsed) = serde_json::from_str::<LegacyJsonState>(&raw) {
                                if !parsed.projects.is_empty() || !parsed.deployments.is_empty() {
                                    db_projects = parsed.projects;
                                    db_deployments = parsed.deployments;
                                    if !parsed.secrets.is_empty() {
                                        db_secrets = parsed.secrets;
                                    }
                                    needs_save = true;

                                    // Preserve backup file so user data is never lost, but NEVER write into legacy folders
                                    if !crate::paths::is_in_legacy_dir(&candidate) {
                                        let backup_path = candidate.with_extension("json.bak");
                                        let _ = fs::copy(&candidate, &backup_path);
                                    } else {
                                        let backup_name = format!(
                                            "{}.json.bak",
                                            candidate
                                                .file_stem()
                                                .and_then(|s| s.to_str())
                                                .unwrap_or("legacy_state")
                                        );
                                        let backup_path =
                                            crate::paths::data_dir().join(backup_name);
                                        let _ = fs::copy(&candidate, &backup_path);
                                    }

                                    state.migration_notice = Some(format!(
                                        "Seamlessly migrated {} project(s) and {} deployment(s) from '{}' to SQLite database.",
                                        db_projects.len(),
                                        db_deployments.len(),
                                        candidate
                                            .file_name()
                                            .and_then(|s| s.to_str())
                                                .unwrap_or("state.json")
                                    ));
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            state.projects = db_projects;
            state.deployments = db_deployments;
            state.secrets = db_secrets;

            if state.sync_project_vault_targets_to_secrets() {
                needs_save = true;
            }

            if state.reconcile_orphaned_running_deployments(&HashSet::new()) > 0 {
                needs_save = true;
            }

            if needs_save || !db_path.exists() {
                let _ = save_all_to_db(&mut conn, &state.projects, &state.deployments);
                let _ = save_secrets_to_db(&mut conn, &state.secrets);
            }
        }

        state.last_state_mtime = file_mtime(&db_path);
        let _ = state.sync_from_app_directory();
        state
    }

    pub fn save(&mut self) -> Result<(), String> {
        let Some(path) = self.storage_path.clone() else {
            return Ok(());
        };
        self.save_to(&path)?;
        self.last_state_mtime = file_mtime(&path);
        Ok(())
    }

    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        if crate::paths::is_in_legacy_dir(path) {
            return Err(format!(
                "Safety guard: refusing to write storage into legacy directory '{}'",
                path.display()
            ));
        }

        let db_path = if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("json"))
        {
            path.with_extension("db")
        } else {
            path.to_path_buf()
        };

        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create storage directory: {e}"))?;
            }
        }

        let mut conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open SQLite database {}: {e}", db_path.display()))?;
        save_all_to_db(&mut conn, &self.projects, &self.deployments)
            .map_err(|e| format!("Failed to save state to SQLite database: {e}"))?;
        save_secrets_to_db(&mut conn, &self.secrets)
            .map_err(|e| format!("Failed to save secrets metadata to SQLite database: {e}"))?;

        // If path was originally requested as JSON (e.g. drop-in export), also write JSON mirror
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("json"))
        {
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = fs::write(path, json);
            }
        }

        Ok(())
    }

    /// Synchronizes state with the app directory:
    /// 1. If `appload_state.db` was updated externally, reloads projects & history.
    /// 2. If a `.json` file (containing `AppStorage`, `Project`, or `Vec<Project>`) or a `deploy*.ps1`
    ///    script was pasted into the app directory, merges it into `self` automatically and persists to SQLite.
    pub fn sync_from_app_directory(&mut self) -> Option<String> {
        let state_path = self.storage_path.clone()?;
        let mut notifications = Vec::new();

        if let Some(notice) = self.migration_notice.take() {
            notifications.push(notice);
        }

        // 1. Check if primary `appload_state.db` was updated externally on disk
        if let Some(disk_mtime) = file_mtime(&state_path) {
            let is_changed = self.last_state_mtime.is_none_or(|prev| disk_mtime > prev);
            if is_changed {
                if let Ok(conn) = Connection::open(&state_path) {
                    if let (Ok(p), Ok(d)) = (
                        load_projects_from_db(&conn),
                        load_deployments_from_db(&conn),
                    ) {
                        self.projects = p;
                        self.deployments = d;
                        if let Ok(s) = load_secrets_from_db(&conn) {
                            self.secrets = s;
                        }
                        let mut changed_on_sync = self.sync_project_vault_targets_to_secrets();
                        if self.reconcile_orphaned_running_deployments(&HashSet::new()) > 0 {
                            changed_on_sync = true;
                        }
                        if changed_on_sync {
                            let _ = self.save();
                        }
                        self.last_state_mtime = file_mtime(&state_path).or(Some(disk_mtime));
                        notifications.push(format!(
                            "Loaded {} project(s) and {} deployment(s) from SQLite database ({})",
                            self.projects.len(),
                            self.deployments.len(),
                            state_path
                                .file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or("appload_state.db")
                        ));
                    }
                }
            }
        }

        // 2. Scan the app directory for any drop-in `.json` or `.ps1` files
        let Some(app_dir) = state_path.parent().map(|p| p.to_path_buf()) else {
            return if notifications.is_empty() {
                None
            } else {
                Some(notifications.join(" | "))
            };
        };

        let Ok(entries) = fs::read_dir(&app_dir) else {
            return if notifications.is_empty() {
                None
            } else {
                Some(notifications.join(" | "))
            };
        };

        let mut merged_any = false;
        for entry in entries.flatten() {
            let file_path = entry.path();
            if !file_path.is_file() || file_path == state_path {
                continue;
            }
            let Some(ext) = file_path.extension().and_then(|s| s.to_str()) else {
                continue;
            };
            let Some(mtime) = file_mtime(&file_path) else {
                continue;
            };
            if self
                .imported_files_mtime
                .get(&file_path)
                .is_some_and(|prev| *prev >= mtime)
            {
                continue;
            }

            if ext.eq_ignore_ascii_case("json") {
                // Ignore backup files and migration markers
                if file_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| {
                        s.ends_with(".bak")
                            || s.ends_with(".tmp")
                            || s.starts_with("appload_state.pre-migration-")
                            || s.starts_with("migrated_from_shiply")
                    })
                {
                    continue;
                }
                if let Ok(raw) = fs::read_to_string(&file_path) {
                    if let Ok(single_proj) = serde_json::from_str::<Project>(&raw) {
                        if self.merge_projects(vec![single_proj]) > 0 {
                            merged_any = true;
                            notifications.push(format!(
                                "Auto-imported project from '{}'",
                                file_path
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("project.json")
                            ));
                        }
                    } else if let Ok(proj_list) = serde_json::from_str::<Vec<Project>>(&raw) {
                        let added_p = self.merge_projects(proj_list);
                        if added_p > 0 {
                            merged_any = true;
                            notifications.push(format!(
                                "Auto-imported {added_p} project(s) from '{}'",
                                file_path
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("projects.json")
                            ));
                        }
                    } else if let Ok(ext_storage) = serde_json::from_str::<LegacyJsonState>(&raw) {
                        if !ext_storage.projects.is_empty() || !ext_storage.deployments.is_empty() {
                            let added_p = self.merge_projects(ext_storage.projects);
                            let added_d = self.merge_deployments(ext_storage.deployments);
                            if added_p > 0 || added_d > 0 {
                                merged_any = true;
                                notifications.push(format!(
                                    "Auto-imported {added_p} project(s), {added_d} deployment(s) from '{}'",
                                    file_path
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("file.json")
                                ));
                            }
                        }
                    }
                }
                self.imported_files_mtime.insert(file_path, mtime);
            } else if ext.eq_ignore_ascii_case("ps1") {
                let fname = file_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if fname.starts_with("deploy") {
                    if let Ok(imported_proj) = import_project_from_path(&file_path, false) {
                        if self.merge_projects(vec![imported_proj]) > 0 {
                            merged_any = true;
                            notifications.push(format!(
                                "Auto-imported PowerShell script '{}'",
                                file_path
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("deploy.ps1")
                            ));
                        }
                    }
                }
                self.imported_files_mtime.insert(file_path, mtime);
            }
        }

        if merged_any {
            let _ = self.save();
        }

        if notifications.is_empty() {
            None
        } else {
            Some(notifications.join(" | "))
        }
    }

    pub fn merge_projects(&mut self, incoming: Vec<Project>) -> usize {
        let mut count = 0;
        for proj in incoming {
            if let Some(existing) = self.projects.iter_mut().find(|p| p.id == proj.id) {
                for env in proj.environments {
                    if !existing.environments.iter().any(|e| e.key == env.key) {
                        existing.environments.push(env);
                        count += 1;
                    }
                }
            } else {
                self.projects.push(proj);
                count += 1;
            }
        }
        count
    }

    /// Duplicates an existing project, including its environments and pipeline steps,
    /// giving the duplicate a unique display name and project ID, and immediately persists to SQLite.
    pub fn duplicate_project(&mut self, project_id: &str) -> Option<Project> {
        let orig = self.projects.iter().find(|p| p.id == project_id)?.clone();

        let base_name = &orig.name;
        let mut new_name = format!("{base_name} (Copy)");
        let mut counter = 2;
        while self.projects.iter().any(|p| p.name == new_name) {
            new_name = format!("{base_name} (Copy {counter})");
            counter += 1;
        }

        let mut new_id = format!("{}-copy", orig.id);
        let mut id_counter = 2;
        while self.projects.iter().any(|p| p.id == new_id) {
            new_id = format!("{}-copy-{}", orig.id, id_counter);
            id_counter += 1;
        }

        let new_project = orig.create_duplicate(new_id, new_name);
        self.projects.push(new_project.clone());
        let _ = self.save();
        Some(new_project)
    }

    /// Reorders a project from `from_idx` to `to_idx` and updates manual sort order.
    pub fn reorder_project(&mut self, from_idx: usize, to_idx: usize) -> bool {
        if from_idx != to_idx && from_idx < self.projects.len() && to_idx < self.projects.len() {
            let proj = self.projects.remove(from_idx);
            self.projects.insert(to_idx, proj);
            for (i, p) in self.projects.iter_mut().enumerate() {
                p.sort_order = Some(i as i32);
            }
            let _ = self.save();
            true
        } else {
            false
        }
    }

    /// Moves a project up by 1 position in manual sort order.
    pub fn move_project_up(&mut self, idx: usize) -> bool {
        if idx > 0 && idx < self.projects.len() {
            self.projects.swap(idx - 1, idx);
            for (i, p) in self.projects.iter_mut().enumerate() {
                p.sort_order = Some(i as i32);
            }
            let _ = self.save();
            true
        } else {
            false
        }
    }

    /// Moves a project down by 1 position in manual sort order.
    pub fn move_project_down(&mut self, idx: usize) -> bool {
        if idx + 1 < self.projects.len() {
            self.projects.swap(idx, idx + 1);
            for (i, p) in self.projects.iter_mut().enumerate() {
                p.sort_order = Some(i as i32);
            }
            let _ = self.save();
            true
        } else {
            false
        }
    }

    /// Sets or clears the group for a given project ID.
    pub fn set_project_group(&mut self, project_id: &str, group: Option<String>) -> bool {
        if let Some(proj) = self.projects.iter_mut().find(|p| p.id == project_id) {
            let normalized = group
                .map(|g| g.trim().to_string())
                .filter(|g| !g.is_empty());
            proj.group = normalized;
            let _ = self.save();
            true
        } else {
            false
        }
    }

    /// Returns a deduplicated, sorted list of all existing non-empty project group names.
    pub fn existing_project_groups(&self) -> Vec<String> {
        let mut groups = Vec::new();
        for proj in &self.projects {
            if let Some(g) = &proj.group {
                let trimmed = g.trim();
                if !trimmed.is_empty()
                    && !groups
                        .iter()
                        .any(|existing: &String| existing.eq_ignore_ascii_case(trimmed))
                {
                    groups.push(trimmed.to_string());
                }
            }
        }
        groups.sort();
        groups
    }

    /// Sorts projects according to the requested sort mode and persists the order.
    pub fn sort_projects(&mut self, mode: ProjectSortMode) {
        match mode {
            ProjectSortMode::Custom => {
                // Preserves current custom order
            }
            ProjectSortMode::NameAsc => {
                self.projects.sort_by_key(|a| a.name.to_lowercase());
                for (i, p) in self.projects.iter_mut().enumerate() {
                    p.sort_order = Some(i as i32);
                }
                let _ = self.save();
            }
            ProjectSortMode::NameDesc => {
                self.projects
                    .sort_by_key(|a| std::cmp::Reverse(a.name.to_lowercase()));
                for (i, p) in self.projects.iter_mut().enumerate() {
                    p.sort_order = Some(i as i32);
                }
                let _ = self.save();
            }
            ProjectSortMode::RecentDeploy => {
                let deployments = &self.deployments;
                self.projects.sort_by(|a, b| {
                    let a_latest = deployments
                        .iter()
                        .find(|d| d.project_id == a.id)
                        .map(|d| d.started_at);
                    let b_latest = deployments
                        .iter()
                        .find(|d| d.project_id == b.id)
                        .map(|d| d.started_at);
                    match (a_latest, b_latest) {
                        (Some(ta), Some(tb)) => tb.cmp(&ta),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                    }
                });
                for (i, p) in self.projects.iter_mut().enumerate() {
                    p.sort_order = Some(i as i32);
                }
                let _ = self.save();
            }
        }
    }

    /// Reconciles any deployment record marked `Running` that does not belong to `active_ids`
    /// (e.g. when the application or desktop was closed/restarted while a deployment was in progress).
    /// Marks orphaned records and their running step logs as `Failed`, records `finished_at` and `duration_ms`,
    /// and returns the number of reconciled records.
    pub fn reconcile_orphaned_running_deployments(
        &mut self,
        active_ids: &HashSet<String>,
    ) -> usize {
        let now = chrono::Utc::now();
        let mut reconciled = 0;
        for dep in &mut self.deployments {
            if dep.status == DeploymentStatus::Running && !active_ids.contains(&dep.id) {
                dep.status = DeploymentStatus::Failed;
                dep.finished_at = Some(now);
                let elapsed = (now - dep.started_at).num_milliseconds().max(0) as u64;
                dep.duration_ms = Some(elapsed);

                let mut marked_step = false;
                for step_log in &mut dep.step_logs {
                    if step_log.status == DeploymentStatus::Running {
                        step_log.status = DeploymentStatus::Failed;
                        if !step_log.output.is_empty() && !step_log.output.ends_with('\n') {
                            step_log.output.push('\n');
                        }
                        step_log.output.push_str(
                            "[FAILED] Deployment was interrupted because the application or desktop restarted before completion.",
                        );
                        marked_step = true;
                    }
                }

                if !marked_step {
                    dep.step_logs.push(StepExecutionLog {
                        step_name: "Interrupted Session".to_string(),
                        command_summary: "session interrupted".to_string(),
                        status: DeploymentStatus::Failed,
                        output: "[FAILED] Deployment was interrupted because the application or desktop restarted before completion.".to_string(),
                        duration_ms: 0,
                    });
                }
                reconciled += 1;
            }
        }
        reconciled
    }

    pub fn merge_deployments(&mut self, incoming: Vec<DeploymentRecord>) -> usize {
        let mut count = 0;
        for dep in incoming {
            if !self.deployments.iter().any(|d| d.id == dep.id) {
                self.deployments.push(dep);
                count += 1;
            }
        }
        self.deployments
            .sort_by_key(|a| std::cmp::Reverse(a.started_at));
        count
    }

    /// Inserts a new deployment record at the top (newest-first) or updates an existing record with the same ID,
    /// atomically persisting directly to SQLite.
    pub fn upsert_deployment(&mut self, record: DeploymentRecord) {
        if let Some(path) = &self.storage_path {
            if let Ok(conn) = Connection::open(path) {
                let _ = upsert_deployment_to_db(&conn, &record);
                self.last_state_mtime = file_mtime(path);
            }
        }

        if let Some(existing) = self.deployments.iter_mut().find(|d| d.id == record.id) {
            *existing = record;
        } else {
            self.deployments.insert(0, record);
        }
    }

    /// Removes a single deployment record by ID (if it is not currently Running) and persists to SQLite.
    pub fn remove_deployment(&mut self, deployment_id: &str) -> Option<DeploymentRecord> {
        let pos = self
            .deployments
            .iter()
            .position(|d| d.id == deployment_id && d.status != DeploymentStatus::Running)?;
        let removed = self.deployments.remove(pos);

        if let Some(path) = &self.storage_path {
            if let Ok(conn) = Connection::open(path) {
                let _ = conn.execute(
                    "DELETE FROM deployments WHERE id = ?1",
                    params![deployment_id],
                );
                self.last_state_mtime = file_mtime(path);
            }
        }
        Some(removed)
    }

    /// Clears non-running deployment history matching the optional `project_id` and `env_key` filters,
    /// persisting to SQLite and returning the number of deleted records.
    pub fn clear_deployment_history(
        &mut self,
        project_id: Option<&str>,
        env_key: Option<&str>,
    ) -> usize {
        let before = self.deployments.len();
        self.deployments.retain(|d| {
            if d.status == DeploymentStatus::Running {
                return true;
            }
            let matches_proj = project_id.is_none_or(|pid| d.project_id == pid);
            let matches_env = env_key.is_none_or(|ek| d.environment == ek);
            !(matches_proj && matches_env)
        });
        let removed_count = before.saturating_sub(self.deployments.len());
        if removed_count > 0 {
            let _ = self.save();
        }
        removed_count
    }

    /// Inserts or updates a secret metadata entry in `self.secrets` and persists to SQLite.
    pub fn upsert_secret_metadata(&mut self, mut entry: SecretEntry) {
        entry.key = entry.key.trim().to_string();
        if entry.key.is_empty() {
            return;
        }
        if let Some(existing) = self
            .secrets
            .iter_mut()
            .find(|s| s.key.eq_ignore_ascii_case(&entry.key))
        {
            *existing = entry;
        } else {
            self.secrets.push(entry);
            self.secrets.sort_by_key(|a| a.key.to_lowercase());
        }
        let _ = self.save();
    }

    /// Removes a secret metadata entry by key and persists to SQLite.
    pub fn remove_secret_metadata(&mut self, key: &str) -> Option<SecretEntry> {
        let trimmed = key.trim();
        let pos = self
            .secrets
            .iter()
            .position(|s| s.key.eq_ignore_ascii_case(trimmed))?;
        let removed = self.secrets.remove(pos);
        let _ = self.save();
        Some(removed)
    }

    /// Ensures any non-empty `vault_target` configured across project environments is registered
    /// in `self.secrets` so users can view and manage all project credentials in Secret Management.
    pub fn sync_project_vault_targets_to_secrets(&mut self) -> bool {
        let mut added = false;
        let now = chrono::Utc::now();
        for proj in &self.projects {
            for env in &proj.environments {
                let target = env.vault_target.trim();
                if target.is_empty() {
                    continue;
                }
                if !self
                    .secrets
                    .iter()
                    .any(|s| s.key.eq_ignore_ascii_case(target))
                {
                    let username = read_os_vault_credential(target)
                        .and_then(|c| c.username)
                        .or_else(|| {
                            let u = env.remote_user.trim();
                            if u.is_empty() {
                                None
                            } else {
                                Some(u.to_string())
                            }
                        });
                    self.secrets.push(SecretEntry {
                        key: target.to_string(),
                        username,
                        description: format!("SSH Vault target for {} ({})", proj.name, env.name),
                        updated_at: now,
                    });
                    added = true;
                }
            }
        }
        if added {
            self.secrets.sort_by_key(|a| a.key.to_lowercase());
        }
        added
    }

    /// Returns a list of `"Project Name (ENV)"` labels for all environments referencing `secret_key`.
    pub fn environments_using_secret(&self, secret_key: &str) -> Vec<String> {
        let trimmed = secret_key.trim();
        let mut usages = Vec::new();
        for proj in &self.projects {
            let mut matching_envs = Vec::new();
            for env in &proj.environments {
                if env.vault_target.trim().eq_ignore_ascii_case(trimmed) {
                    matching_envs.push(crate::domain::format_env_badge(&env.key));
                }
            }
            if !matching_envs.is_empty() {
                usages.push(format!("{} ({})", proj.name, matching_envs.join(", ")));
            }
        }
        usages
    }

    /// Returns the most recent successful deployment for a given project and environment key.
    pub fn last_succeeded_deployment(
        &self,
        project_id: &str,
        env_key: &str,
    ) -> Option<&DeploymentRecord> {
        self.deployments.iter().find(|d| {
            d.project_id == project_id
                && d.environment == env_key
                && d.status == DeploymentStatus::Succeeded
        })
    }

    /// Returns the previous successful deployment (prior to the currently active one) for Rollback.
    pub fn previous_rollback_candidate(
        &self,
        project_id: &str,
        env_key: &str,
    ) -> Option<&DeploymentRecord> {
        let mut succeeded_iter = self.deployments.iter().filter(|d| {
            d.project_id == project_id
                && d.environment == env_key
                && d.status == DeploymentStatus::Succeeded
        });
        let _current = succeeded_iter.next()?;
        succeeded_iter.next()
    }

    /// Returns the most recent deployment (regardless of status) for a given project and environment key.
    pub fn latest_deployment(&self, project_id: &str, env_key: &str) -> Option<&DeploymentRecord> {
        self.deployments
            .iter()
            .find(|d| d.project_id == project_id && d.environment == env_key)
    }

    /// Filters deployments by optional project ID, optional environment key, and optional status.
    pub fn filter_deployments(
        &self,
        project_id: Option<&str>,
        env_filter: Option<&str>,
        status_filter: Option<DeploymentStatus>,
    ) -> Vec<&DeploymentRecord> {
        self.deployments
            .iter()
            .filter(|d| {
                project_id.is_none_or(|pid| d.project_id == pid)
                    && env_filter.is_none_or(|env| d.environment == env)
                    && status_filter.is_none_or(|st| d.status == st)
            })
            .collect()
    }

    /// Returns the environment filter tabs `(display_name, Some(env_key))` representing
    /// the existing environments for either a specific project or across all projects.
    pub fn existing_environment_tabs(
        &self,
        project_filter: Option<&str>,
    ) -> Vec<(String, Option<String>)> {
        let mut tabs = vec![("All Environments".to_string(), None)];
        if let Some(pid) = project_filter {
            if let Some(proj) = self.projects.iter().find(|p| p.id == pid) {
                for env in &proj.environments {
                    if !tabs
                        .iter()
                        .any(|(_, k)| k.as_deref() == Some(env.key.as_str()))
                    {
                        tabs.push((env.name.clone(), Some(env.key.clone())));
                    }
                }
            }
        } else {
            for proj in &self.projects {
                for env in &proj.environments {
                    if !tabs
                        .iter()
                        .any(|(_, k)| k.as_deref() == Some(env.key.as_str()))
                    {
                        tabs.push((env.name.clone(), Some(env.key.clone())));
                    }
                }
            }
        }
        tabs
    }

    /// Computes the next auto-bumped version for the Version input field based on the
    /// last succeeded deployment history for `(project_id, env_key)` (or any environment in `project_id`),
    /// falling back to `package.json` or `"1.0.0"` if no succeeded deployment history exists yet.
    /// Failed deployments do NOT bump the version.
    pub fn next_auto_bumped_version(
        &self,
        project_id: &str,
        env_key: &str,
        fallback_pkg_version: Option<&str>,
    ) -> String {
        if let Some(dep) = self.last_succeeded_deployment(project_id, env_key) {
            return bump_semver(&dep.version, VersionBump::Patch);
        }

        if let Some(dep) = self
            .deployments
            .iter()
            .find(|d| d.project_id == project_id && d.status == DeploymentStatus::Succeeded)
        {
            return bump_semver(&dep.version, VersionBump::Patch);
        }

        if let Some(pkg_ver) = fallback_pkg_version {
            let trimmed = pkg_ver.trim();
            if !trimmed.is_empty() && trimmed != "0.0.0" {
                return trimmed.to_string();
            }
        }

        "1.0.0".to_string()
    }
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    fn dummy_record(
        id: &str,
        project_id: &str,
        env_key: &str,
        version: &str,
        status: DeploymentStatus,
    ) -> DeploymentRecord {
        DeploymentRecord {
            id: id.to_string(),
            project_id: project_id.to_string(),
            project_name: project_id.to_string(),
            environment: env_key.to_string(),
            app_name: format!("{project_id}-{env_key}"),
            version: version.to_string(),
            git_branch: Some("main".to_string()),
            git_commit: Some("a1b2c3d".to_string()),
            git_changes: vec!["a1b2c3d feat: initial release".to_string()],
            is_rollback: false,
            servers: vec!["10.62.70.80".to_string()],
            status,
            started_at: Utc::now(),
            finished_at: Some(Utc::now()),
            duration_ms: Some(1200),
            step_logs: Vec::new(),
            executed_steps: Vec::new(),
        }
    }

    #[test]
    fn test_first_time_install_is_empty_without_project_or_environment() {
        let tmp = TempDir::new().expect("tempdir");
        let file_path = tmp.path().join("appload_state.db");

        let storage = AppStorage::load_or_default(&file_path);
        assert!(storage.projects.is_empty());
        assert!(storage.deployments.is_empty());
        assert!(file_path.exists());
    }

    #[test]
    fn test_seamless_migration_from_legacy_json_to_sqlite() {
        let tmp = TempDir::new().expect("tempdir");
        let json_path = tmp.path().join("shiply_state.json");
        let db_path = tmp.path().join("shiply_state.db");

        // 1. Create a legacy shiply_state.json with projects and deployments
        let sample_proj = Project {
            id: "legacy-proj".to_string(),
            name: "Legacy Project".to_string(),
            description: "From JSON".to_string(),
            local_path: r"C:\Legacy\Proj".to_string(),
            environments: Vec::new(),
            group: None,
            sort_order: None,
        };
        let sample_dep = dummy_record(
            "dep-legacy-1",
            "legacy-proj",
            "prod",
            "1.2.3",
            DeploymentStatus::Succeeded,
        );

        let legacy_data = serde_json::json!({
            "projects": [sample_proj],
            "deployments": [sample_dep]
        });
        fs::write(
            &json_path,
            serde_json::to_string_pretty(&legacy_data).unwrap(),
        )
        .unwrap();

        // 2. Call load_or_default with the SQLite db path
        let storage = AppStorage::load_or_default(&db_path);

        // 3. Verify SQLite was created and populated seamlessly
        assert!(db_path.exists());
        assert_eq!(storage.projects.len(), 1);
        assert_eq!(storage.projects[0].id, "legacy-proj");
        assert_eq!(storage.projects[0].name, "Legacy Project");
        assert_eq!(storage.deployments.len(), 1);
        assert_eq!(storage.deployments[0].id, "dep-legacy-1");
        assert_eq!(storage.deployments[0].version, "1.2.3");

        // 4. Verify backup was preserved
        assert!(tmp.path().join("shiply_state.json.bak").exists());

        // 5. Subsequent load reads directly from SQLite
        let reloaded = AppStorage::load_or_default(&db_path);
        assert_eq!(reloaded.projects.len(), 1);
        assert_eq!(reloaded.deployments.len(), 1);
    }

    #[test]
    fn test_storage_persistence_and_auto_recognize_pasted_files() {
        let tmp = TempDir::new().expect("tempdir");
        let file_path = tmp.path().join("appload_state.db");

        let mut storage = AppStorage::load_or_default(&file_path);
        storage.upsert_deployment(dummy_record(
            "dep-1",
            "ndm-dq-subscription",
            "dev",
            "1.0.0",
            DeploymentStatus::Succeeded,
        ));
        storage.upsert_deployment(dummy_record(
            "dep-2",
            "ndm-dq-subscription",
            "dev",
            "1.0.1",
            DeploymentStatus::Succeeded,
        ));

        let last_ok = storage
            .last_succeeded_deployment("ndm-dq-subscription", "dev")
            .expect("last succeeded");
        assert_eq!(last_ok.version, "1.0.1");

        let rollback_cand = storage
            .previous_rollback_candidate("ndm-dq-subscription", "dev")
            .expect("rollback candidate");
        assert_eq!(rollback_cand.version, "1.0.0");

        // Simulate user pasting a project JSON file into the app directory
        let drop_json = tmp.path().join("my_project.json");
        let sample_proj = Project {
            id: "ndm-dq-subscription".to_string(),
            name: "ndm-dq-subscription".to_string(),
            description: "Backend Service".to_string(),
            local_path: r"D:\NTI\Projects\backend\ndm-dq-subscription".to_string(),
            environments: Vec::new(),
            group: None,
            sort_order: None,
        };
        fs::write(
            &drop_json,
            serde_json::to_string_pretty(&sample_proj).unwrap(),
        )
        .expect("write drop_json");

        let notice = storage.sync_from_app_directory();
        assert!(notice.is_some());
        assert_eq!(storage.projects.len(), 1);
        assert_eq!(storage.projects[0].id, "ndm-dq-subscription");
    }

    #[test]
    fn test_existing_environment_tabs_and_auto_bumped_version() {
        let mut storage = AppStorage::default();
        let mut proj = Project {
            id: "web-api".to_string(),
            name: "web-api".to_string(),
            description: "API Service".to_string(),
            local_path: r"C:\Projects\web-api".to_string(),
            environments: Vec::new(),
            group: None,
            sort_order: None,
        };
        proj.add_environment("Development", None).unwrap();
        proj.add_environment("QA", None).unwrap();
        storage.projects.push(proj);

        // Tabs on All Deployments page (project_filter = None) reflect actual existing environments
        let all_tabs = storage.existing_environment_tabs(None);
        assert_eq!(
            all_tabs,
            vec![
                ("All Environments".to_string(), None),
                ("Development".to_string(), Some("development".to_string())),
                ("QA".to_string(), Some("qa".to_string())),
            ]
        );

        // Without history, falls back to package.json or 1.0.0
        assert_eq!(
            storage.next_auto_bumped_version("web-api", "development", Some("1.4.0")),
            "1.4.0"
        );
        assert_eq!(
            storage.next_auto_bumped_version("web-api", "development", None),
            "1.0.0"
        );

        // With deployment history, auto-bumps the latest succeeded deployment version
        storage.upsert_deployment(dummy_record(
            "d1",
            "web-api",
            "development",
            "1.4.2",
            DeploymentStatus::Succeeded,
        ));
        assert_eq!(
            storage.next_auto_bumped_version("web-api", "development", Some("1.4.2")),
            "1.4.3"
        );

        // A failed deployment MUST NOT bump the auto-suggested version
        storage.upsert_deployment(dummy_record(
            "d2",
            "web-api",
            "development",
            "1.4.3",
            DeploymentStatus::Failed,
        ));
        assert_eq!(
            storage.next_auto_bumped_version("web-api", "development", Some("1.4.2")),
            "1.4.3"
        );
    }

    #[test]
    fn test_storage_duplicate_project() {
        let tmp = TempDir::new().expect("tempdir");
        let db_path = tmp.path().join("appload_state.db");
        let mut storage = AppStorage::load_or_default(&db_path);

        let proj = Project {
            id: "api-service".to_string(),
            name: "API Service".to_string(),
            description: "Main backend".to_string(),
            local_path: r"C:\apps\api".to_string(),
            environments: vec![],
            group: Some("Services".to_string()),
            sort_order: Some(1),
        };
        storage.projects.push(proj);
        let _ = storage.save();

        // 1. First duplication
        let dup1 = storage
            .duplicate_project("api-service")
            .expect("duplicate 1");
        assert_eq!(dup1.id, "api-service-copy");
        assert_eq!(dup1.name, "API Service (Copy)");
        assert_eq!(dup1.group, Some("Services".to_string()));
        assert_eq!(storage.projects.len(), 2);

        // 2. Second duplication of original
        let dup2 = storage
            .duplicate_project("api-service")
            .expect("duplicate 2");
        assert_eq!(dup2.id, "api-service-copy-2");
        assert_eq!(dup2.name, "API Service (Copy 2)");
        assert_eq!(dup2.group, Some("Services".to_string()));
        assert_eq!(storage.projects.len(), 3);

        // 3. Reload from SQLite and verify persistence
        let reloaded = AppStorage::load_or_default(&db_path);
        assert_eq!(reloaded.projects.len(), 3);
        assert!(reloaded.projects.iter().any(|p| p.id == "api-service-copy"));
        assert!(reloaded
            .projects
            .iter()
            .any(|p| p.id == "api-service-copy-2"));
    }

    #[test]
    fn test_storage_project_sort_and_reorder() {
        let tmp = TempDir::new().expect("tempdir");
        let db_path = tmp.path().join("appload_state.db");
        let mut storage = AppStorage::load_or_default(&db_path);

        let p1 = Project {
            id: "p1".to_string(),
            name: "Alpha".to_string(),
            description: "".to_string(),
            local_path: "".to_string(),
            environments: vec![],
            group: None,
            sort_order: Some(0),
        };
        let p2 = Project {
            id: "p2".to_string(),
            name: "Beta".to_string(),
            description: "".to_string(),
            local_path: "".to_string(),
            environments: vec![],
            group: None,
            sort_order: Some(1),
        };
        let p3 = Project {
            id: "p3".to_string(),
            name: "Gamma".to_string(),
            description: "".to_string(),
            local_path: "".to_string(),
            environments: vec![],
            group: None,
            sort_order: Some(2),
        };

        storage.projects = vec![p1, p2, p3];
        let _ = storage.save();

        // Test move down
        assert!(storage.move_project_down(0));
        assert_eq!(storage.projects[0].id, "p2");
        assert_eq!(storage.projects[1].id, "p1");
        assert_eq!(storage.projects[0].sort_order, Some(0));
        assert_eq!(storage.projects[1].sort_order, Some(1));

        // Test move up
        assert!(storage.move_project_up(2));
        assert_eq!(storage.projects[1].id, "p3");
        assert_eq!(storage.projects[2].id, "p1");

        // Test reorder_project
        assert!(storage.reorder_project(2, 0));
        assert_eq!(storage.projects[0].id, "p1");
        assert_eq!(storage.projects[1].id, "p2");
        assert_eq!(storage.projects[2].id, "p3");

        // Reload from DB and verify sort_order is preserved
        let reloaded = AppStorage::load_or_default(&db_path);
        assert_eq!(reloaded.projects[0].id, "p1");
        assert_eq!(reloaded.projects[1].id, "p2");
        assert_eq!(reloaded.projects[2].id, "p3");

        // Test sort by NameDesc
        storage.sort_projects(ProjectSortMode::NameDesc);
        assert_eq!(storage.projects[0].name, "Gamma");
        assert_eq!(storage.projects[1].name, "Beta");
        assert_eq!(storage.projects[2].name, "Alpha");
    }

    #[test]
    fn test_storage_project_grouping_and_persistence() {
        let tmp = TempDir::new().expect("tempdir");
        let db_path = tmp.path().join("appload_state.db");
        let mut storage = AppStorage::load_or_default(&db_path);

        let p1 = Project {
            id: "p1".to_string(),
            name: "P1".to_string(),
            description: "".to_string(),
            local_path: "".to_string(),
            environments: vec![],
            group: Some("Backend".to_string()),
            sort_order: Some(0),
        };
        let p2 = Project {
            id: "p2".to_string(),
            name: "P2".to_string(),
            description: "".to_string(),
            local_path: "".to_string(),
            environments: vec![],
            group: Some("Frontend".to_string()),
            sort_order: Some(1),
        };
        let p3 = Project {
            id: "p3".to_string(),
            name: "P3".to_string(),
            description: "".to_string(),
            local_path: "".to_string(),
            environments: vec![],
            group: None,
            sort_order: Some(2),
        };
        storage.projects = vec![p1, p2, p3];
        let _ = storage.save();

        let groups = storage.existing_project_groups();
        assert_eq!(groups, vec!["Backend", "Frontend"]);

        // Change group of p3
        assert!(storage.set_project_group("p3", Some("Backend".to_string())));
        assert_eq!(storage.projects[2].group.as_deref(), Some("Backend"));

        // Ungroup p1
        assert!(storage.set_project_group("p1", None));
        assert_eq!(storage.projects[0].group, None);

        // Reload and verify
        let reloaded = AppStorage::load_or_default(&db_path);
        assert_eq!(
            reloaded
                .projects
                .iter()
                .find(|p| p.id == "p3")
                .unwrap()
                .group
                .as_deref(),
            Some("Backend")
        );
        assert_eq!(
            reloaded
                .projects
                .iter()
                .find(|p| p.id == "p1")
                .unwrap()
                .group,
            None
        );
    }

    #[test]
    fn test_reconcile_orphaned_running_deployments_on_load() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("appload_state.db");

        // 1. Create a deployment that is left in Running state (simulating crash or desktop restart)
        let mut storage = AppStorage::load_or_default(&db_path);
        let started_at = chrono::Utc::now() - chrono::Duration::seconds(45);
        let running_dep = DeploymentRecord {
            id: "dep-orphaned-1".to_string(),
            project_id: "backend-cdo".to_string(),
            project_name: "Backend Customer Data Ops".to_string(),
            environment: "production".to_string(),
            app_name: "backend-cdo".to_string(),
            version: "1.1.22".to_string(),
            git_branch: Some("main".to_string()),
            git_commit: Some("70c0d68".to_string()),
            git_changes: vec![],
            is_rollback: false,
            servers: vec!["10.0.0.1".to_string()],
            status: DeploymentStatus::Running,
            started_at,
            finished_at: None,
            duration_ms: None,
            step_logs: vec![StepExecutionLog {
                step_name: "Upload Artifacts via SCP".to_string(),
                command_summary: "[SCP Upload] ./dist -> {remote_dir}".to_string(),
                status: DeploymentStatus::Running,
                output: "Uploading files...".to_string(),
                duration_ms: 0,
            }],
            executed_steps: vec![],
        };
        storage.upsert_deployment(running_dep);

        // 2. Reload storage from disk (as happens on app startup after desktop restart)
        let reloaded = AppStorage::load_or_default(&db_path);
        assert_eq!(reloaded.deployments.len(), 1);
        let rec = &reloaded.deployments[0];
        assert_eq!(rec.id, "dep-orphaned-1");
        assert_eq!(
            rec.status,
            DeploymentStatus::Failed,
            "Orphaned Running deployment must be reconciled to Failed on startup"
        );
        assert!(rec.finished_at.is_some());
        assert!(rec.duration_ms.is_some());
        assert_eq!(rec.step_logs[0].status, DeploymentStatus::Failed);
        assert!(rec.step_logs[0].output.contains("interrupted"));
    }

    #[test]
    fn test_remove_deployment_and_clear_deployment_history() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("appload_state.db");

        let mut storage = AppStorage::load_or_default(&db_path);
        let now = chrono::Utc::now();

        let make_dep =
            |id: &str, proj: &str, env: &str, status: DeploymentStatus| DeploymentRecord {
                id: id.to_string(),
                project_id: proj.to_string(),
                project_name: format!("Project {proj}"),
                environment: env.to_string(),
                app_name: format!("{proj}-{env}"),
                version: "1.0.0".to_string(),
                git_branch: Some("main".to_string()),
                git_commit: Some("abc1234".to_string()),
                git_changes: vec![],
                is_rollback: false,
                servers: vec!["10.0.0.1".to_string()],
                status,
                started_at: now,
                finished_at: if status == DeploymentStatus::Running {
                    None
                } else {
                    Some(now)
                },
                duration_ms: if status == DeploymentStatus::Running {
                    None
                } else {
                    Some(1200)
                },
                step_logs: vec![],
                executed_steps: vec![],
            };

        storage.upsert_deployment(make_dep("dep-1", "p1", "dev", DeploymentStatus::Succeeded));
        storage.upsert_deployment(make_dep("dep-2", "p1", "dev", DeploymentStatus::Failed));
        storage.upsert_deployment(make_dep("dep-3", "p1", "prod", DeploymentStatus::Succeeded));
        storage.upsert_deployment(make_dep(
            "dep-running",
            "p1",
            "dev",
            DeploymentStatus::Running,
        ));
        storage.upsert_deployment(make_dep("dep-4", "p2", "prod", DeploymentStatus::Cancelled));

        // 1. Running deployments cannot be removed while running
        assert!(storage.remove_deployment("dep-running").is_none());

        // 2. Remove a single completed deployment record and verify SQLite persistence
        let removed = storage.remove_deployment("dep-1");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, "dep-1");
        assert_eq!(storage.deployments.len(), 4);

        // 3. Clear history for p1 + dev (should remove dep-2, keep dep-running, dep-3, dep-4)
        let cleared_dev = storage.clear_deployment_history(Some("p1"), Some("dev"));
        assert_eq!(cleared_dev, 1);
        assert!(storage.deployments.iter().any(|d| d.id == "dep-running"));
        assert!(!storage.deployments.iter().any(|d| d.id == "dep-2"));
        assert!(storage.deployments.iter().any(|d| d.id == "dep-3"));
        assert!(storage.deployments.iter().any(|d| d.id == "dep-4"));

        // 4. Clear all remaining non-running history across all projects/environments
        let cleared_all = storage.clear_deployment_history(None, None);
        assert_eq!(cleared_all, 2);
        assert_eq!(storage.deployments.len(), 1);
        assert_eq!(storage.deployments[0].id, "dep-running");
    }

    #[test]
    fn test_secrets_metadata_persistence_and_project_vault_sync() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("appload_state.db");

        let mut storage = AppStorage::load_or_default(&db_path);
        storage.upsert_secret_metadata(SecretEntry::new(
            "GITHUB_TOKEN",
            Some("octocat".to_string()),
            "GitHub API Personal Access Token",
        ));
        storage.upsert_secret_metadata(SecretEntry::new(
            "prod-ssh-server",
            Some("deploy".to_string()),
            "Production SSH Server Credential",
        ));
        assert_eq!(storage.secrets.len(), 2);

        // Reload from SQLite and verify secrets metadata persisted
        let mut reloaded = AppStorage::load_or_default(&db_path);
        assert_eq!(reloaded.secrets.len(), 2);
        assert_eq!(reloaded.secrets[0].key, "GITHUB_TOKEN");
        assert_eq!(reloaded.secrets[0].username.as_deref(), Some("octocat"));
        assert_eq!(reloaded.secrets[1].key, "prod-ssh-server");

        // Remove a secret metadata entry and verify
        let removed = reloaded.remove_secret_metadata("GITHUB_TOKEN");
        assert!(removed.is_some());
        assert_eq!(reloaded.secrets.len(), 1);
    }

    #[test]
    fn test_characterization_storage_json_schema_frozen() {
        let json_data = r#"{
            "projects": [
                {
                    "id": "proj-1",
                    "name": "Project Alpha",
                    "description": "Alpha service",
                    "local_path": "/alpha",
                    "group": "Ops",
                    "environments": [
                        {
                            "key": "prod",
                            "name": "Production",
                            "app_name": "alpha-prod",
                            "servers": ["srv1.internal"],
                            "remote_dir": "/var/www/alpha",
                            "remote_user": "ubuntu",
                            "vault_target": "alpha-vault",
                            "steps": []
                        }
                    ]
                }
            ],
            "deployments": [],
            "secrets": [
                {
                    "key": "ALPHA_KEY",
                    "username": "admin",
                    "description": "API Key",
                    "updated_at": "2026-11-04T00:00:00Z"
                }
            ]
        }"#;
        let storage: AppStorage = serde_json::from_str(json_data).expect("deserialize legacy json");
        assert_eq!(storage.projects.len(), 1);
        assert_eq!(storage.projects[0].name, "Project Alpha");
        assert_eq!(storage.projects[0].description, "Alpha service");
        assert_eq!(storage.projects[0].local_path, "/alpha");
        assert_eq!(storage.projects[0].group.as_deref(), Some("Ops"));
        assert_eq!(storage.projects[0].environments.len(), 1);
        assert_eq!(storage.projects[0].environments[0].key, "prod");
        assert_eq!(storage.secrets.len(), 1);
        assert_eq!(storage.secrets[0].key, "ALPHA_KEY");
    }
}
