//! Main ShiplyApp state container and top-level eframe/egui render loop.

use super::*;

pub struct ShiplyApp {
    pub(crate) storage: AppStorage,
    pub(crate) selection: SidebarSelection,
    pub(crate) selected_env_key: String,
    pub(crate) active_tab: WorkspaceTab,
    pub(crate) history_env_filter: Option<String>,
    pub(crate) history_visible_limit: usize,
    pub(crate) selected_deployment_id: Option<String>,

    // Debounced disk save & field editing buffers
    pub(crate) dirty_save_pending: bool,
    pub(crate) last_edit_time: Option<Instant>,
    pub(crate) servers_edit_buffer: Option<(String, String)>,

    // Collapsible Console Output & Code Changes state
    pub(crate) console_collapsed: bool,
    pub(crate) console_expanded_height: bool,
    pub(crate) changes_collapsed: bool,
    pub(crate) collapsed_step_indices: HashSet<usize>,
    pub(crate) dragged_step_idx: Option<usize>,
    pub(crate) history_search_query: String,

    // Project Sorting & Grouping state
    pub(crate) project_sort_mode: ProjectSortMode,
    pub(crate) collapsed_project_groups: HashSet<String>,
    pub(crate) dragged_project_idx: Option<usize>,
    pub(crate) sidebar_sort_dropdown_open: bool,
    pub project_search_query: String,
    pub(crate) sidebar_context_menu: Option<SidebarContextMenuState>,
    pub(crate) sync_version_dropdown_open: bool,

    // Deployment version, branch & release controls
    pub(crate) deploy_version_input: String,
    pub(crate) selected_git_branch: String,
    pub(crate) bump_package_json: bool,
    pub(crate) create_git_tag: bool,
    pub(crate) manual_ssh_password: String,

    // Cached project metadata (Git + branches + package.json + OS Vault check)
    pub(crate) cached_project_id: Option<String>,
    pub(crate) cached_pkg_version: Option<String>,
    pub(crate) cached_git_meta: GitMetadata,
    pub(crate) cached_git_branches: Vec<String>,
    pub(crate) cached_vault_found: bool,
    pub(crate) cached_git_changes: Vec<String>,

    // Background execution channel
    pub(crate) event_tx: Sender<ExecutorEvent>,
    pub(crate) event_rx: Receiver<ExecutorEvent>,
    pub active_deployments: HashMap<(String, String), String>,
    pub active_deployment_id: Option<String>,

    // Async Git inspection channel (instant project switching without UI freezing)
    pub(crate) git_inspect_tx: Sender<GitInspectionResult>,
    pub(crate) git_inspect_rx: Receiver<GitInspectionResult>,

    // Add Project & Auto-Import .ps1 Modal state
    pub(crate) show_new_project_modal: bool,
    pub(crate) new_project_ps1_path: String,
    pub(crate) new_project_import_siblings: bool,
    pub(crate) new_project_name: String,
    pub(crate) new_project_path: String,
    pub(crate) new_project_desc: String,
    pub(crate) new_project_group: String,
    pub(crate) new_project_imported_envs: Vec<EnvironmentConfig>,
    pub(crate) new_project_import_notice: Option<String>,

    // Edit Project Modal state
    pub(crate) show_edit_project_modal: bool,
    pub(crate) edit_project_id: String,
    pub(crate) edit_project_name: String,
    pub(crate) edit_project_path: String,
    pub(crate) edit_project_desc: String,
    pub(crate) edit_project_group: String,
    pub(crate) edit_project_ps1_path: String,
    pub(crate) edit_project_import_notice: Option<String>,

    // Add Environment Modal state
    pub(crate) show_new_env_modal: bool,
    pub(crate) new_env_name: String,
    pub(crate) new_env_clone_from: String,
    pub(crate) new_env_ps1_path: String,

    // Bulk Deployment Modal & Queue state
    pub(crate) show_bulk_deploy_modal: bool,
    pub(crate) bulk_selected_projects: HashSet<String>,
    pub(crate) bulk_target_env_key: String,
    pub(crate) bulk_project_versions: BTreeMap<String, String>,
    pub(crate) bulk_bump_pkg: bool,
    pub(crate) bulk_create_tag: bool,
    pub(crate) bulk_deploy_queue: VecDeque<DeploymentRequest>,
    pub(crate) bulk_total_count: usize,

    // Onboarding & Multi-Platform OS Credential Setup Guide state
    pub(crate) show_onboarding_guide: bool,
    pub(crate) show_credential_guide: bool,
    pub(crate) cred_guide_os_tab: CredentialOsTab,
    pub(crate) cred_guide_target_input: String,
    pub(crate) cred_guide_user_input: String,

    // Secret Management (OS Credential Manager) state
    pub(crate) show_secret_modal: bool,
    pub(crate) secret_form_show_password: bool,
    pub(crate) secret_help_expanded: bool,
    pub(crate) secret_vault_cache: HashMap<String, CachedVaultStatus>,
    pub(crate) secret_vault_cache_dirty: bool,
    pub(crate) secret_vault_last_refresh: Option<Instant>,
    pub(crate) secret_filter_status: SecretFilterStatus,
    pub(crate) secret_form_key: String,
    pub(crate) secret_form_username: String,
    pub(crate) secret_form_value: String,
    pub(crate) secret_form_description: String,
    pub(crate) secret_form_editing_existing: bool,
    pub(crate) secret_search_query: String,

    // In-App GitHub Releases Auto-Updater state
    pub(crate) updater_tx: Sender<UpdaterEvent>,
    pub(crate) updater_rx: Receiver<UpdaterEvent>,
    pub(crate) show_update_modal: bool,
    pub(crate) available_update: Option<ReleaseInfo>,
    pub(crate) update_checking: bool,
    pub(crate) update_downloading: bool,
    pub(crate) update_progress: f32,
    pub(crate) update_status_text: String,
    pub(crate) update_error: Option<String>,
    pub pending_update_restart: Option<(String, PathBuf)>,

    // Throttled external file sync timer (Requirement #8 & #9)
    pub(crate) last_sync_check: Instant,

    // Status banner & Window/Sidebar state
    pub(crate) status_banner: Option<(String, bool)>,
    pub(crate) status_banner_time: Option<Instant>,
    pub(crate) sidebar_collapsed: bool,
    pub(crate) window_maximized: bool,
    pub(crate) pending_delete_confirm: Option<ConfirmDeleteAction>,
    pub(crate) fonts_initialized: bool,
    pub(crate) logo_texture: Option<egui::TextureHandle>,
    pub(crate) project_switch_loading_id: Option<String>,
    pub(crate) project_switch_loading_start: Option<Instant>,
    pub(crate) sidebar_refreshing_until: Option<Instant>,
    pub(crate) dark_mode: bool,
    pub(crate) sidebar_drawer_open: bool,
    pub(crate) is_version_input_focused: bool,
    pub(crate) step_scp_sources_buffer: Option<(String, String)>,
    pub(crate) server_env_source_buffer: Option<(String, String)>,
    pub(crate) server_http_proxy_buffer: Option<(String, String)>,
    pub refresh_anim_start: Option<Instant>,
    #[cfg(debug_assertions)]
    pub(crate) dpi_diagnostics_logged: bool,
    pub(crate) screenshot_frames: usize,
}
impl ShiplyApp {
    pub fn new(storage: AppStorage) -> Self {
        let (event_tx, event_rx) = mpsc::channel();
        let (updater_tx, updater_rx) = mpsc::channel();
        let (git_inspect_tx, git_inspect_rx) = mpsc::channel();
        let migration_notice = storage.migration_notice.clone();
        // Run a non-blocking background check against GitHub Releases on startup
        spawn_update_check(updater_tx.clone(), false);

        let initial_project_id = storage.projects.first().map(|p| p.id.clone());
        let initial_env_key = storage
            .projects
            .first()
            .and_then(|p| p.environments.first())
            .map(|e| e.key.clone())
            .unwrap_or_default();
        let mut selection = match &initial_project_id {
            Some(id) => SidebarSelection::Project(id.clone()),
            None => SidebarSelection::AllDeployments,
        };
        if let Ok(val) = std::env::var("SHIPLY_INITIAL_VIEW") {
            if val.eq_ignore_ascii_case("all_deployments") {
                selection = SidebarSelection::AllDeployments;
            } else if val.eq_ignore_ascii_case("secret_management") {
                selection = SidebarSelection::SecretManagement;
            }
        }
        let modal_env = std::env::var("SHIPLY_OPEN_MODAL").unwrap_or_default();
        let show_modal = modal_env.eq_ignore_ascii_case("new_project");
        let show_secret_modal_init = modal_env.eq_ignore_ascii_case("secret")
            || modal_env.eq_ignore_ascii_case("new_secret");
        let initial_selected_dep = storage.deployments.first().map(|d| d.id.clone());

        let mut active_tab = WorkspaceTab::HistoryAndLogs;
        if let Ok(val) = std::env::var("SHIPLY_INITIAL_TAB") {
            if val.eq_ignore_ascii_case("pipeline_steps") || val.eq_ignore_ascii_case("steps") {
                active_tab = WorkspaceTab::PipelineSteps;
            } else if val.eq_ignore_ascii_case("deployments") || val.eq_ignore_ascii_case("history")
            {
                active_tab = WorkspaceTab::HistoryAndLogs;
            } else if val.eq_ignore_ascii_case("settings") {
                active_tab = WorkspaceTab::ServerSettings;
            }
        }

        let has_no_projects = storage.projects.is_empty();

        let mut app = Self {
            storage,
            selection,
            selected_env_key: initial_env_key.clone(),
            active_tab,
            history_env_filter: None,
            history_visible_limit: 5,
            selected_deployment_id: initial_selected_dep,
            dirty_save_pending: false,
            last_edit_time: None,
            servers_edit_buffer: None,
            console_collapsed: false,
            console_expanded_height: std::env::var("SHIPLY_CONSOLE_EXPANDED").is_ok(),
            changes_collapsed: false,
            collapsed_step_indices: HashSet::new(),
            dragged_step_idx: None,
            history_search_query: String::new(),
            project_sort_mode: ProjectSortMode::Custom,
            collapsed_project_groups: HashSet::new(),
            dragged_project_idx: None,
            sidebar_sort_dropdown_open: false,
            project_search_query: String::new(),
            sidebar_context_menu: None,
            sync_version_dropdown_open: false,
            deploy_version_input: "1.0.0".to_string(),
            selected_git_branch: String::new(),
            bump_package_json: true,
            create_git_tag: false,
            manual_ssh_password: String::new(),
            cached_project_id: None,
            cached_pkg_version: None,
            cached_git_meta: GitMetadata::default(),
            cached_git_branches: Vec::new(),
            cached_vault_found: false,
            cached_git_changes: Vec::new(),
            event_tx,
            event_rx,
            active_deployments: HashMap::new(),
            active_deployment_id: None,
            show_new_project_modal: show_modal,
            new_project_ps1_path: String::new(),
            new_project_import_siblings: true,
            new_project_name: String::new(),
            new_project_path: String::new(),
            new_project_desc: String::new(),
            new_project_group: String::new(),
            new_project_imported_envs: Vec::new(),
            new_project_import_notice: None,
            show_edit_project_modal: false,
            edit_project_id: String::new(),
            edit_project_name: String::new(),
            edit_project_path: String::new(),
            edit_project_desc: String::new(),
            edit_project_group: String::new(),
            edit_project_ps1_path: String::new(),
            edit_project_import_notice: None,
            show_new_env_modal: false,
            new_env_name: String::new(),
            new_env_clone_from: initial_env_key.clone(),
            new_env_ps1_path: String::new(),
            show_bulk_deploy_modal: false,
            bulk_selected_projects: HashSet::new(),
            bulk_target_env_key: initial_env_key,
            bulk_project_versions: BTreeMap::new(),
            bulk_bump_pkg: true,
            bulk_create_tag: false,
            bulk_deploy_queue: VecDeque::new(),
            bulk_total_count: 0,
            show_onboarding_guide: has_no_projects,
            show_credential_guide: false,
            cred_guide_os_tab: if cfg!(windows) {
                CredentialOsTab::Windows
            } else if cfg!(target_os = "macos") {
                CredentialOsTab::Mac
            } else {
                CredentialOsTab::Linux
            },
            cred_guide_target_input: "prod-ssh-server".to_string(),
            cred_guide_user_input: "deploy".to_string(),
            show_secret_modal: show_secret_modal_init,
            secret_form_show_password: false,
            secret_help_expanded: false,
            secret_vault_cache: HashMap::new(),
            secret_vault_cache_dirty: true,
            secret_vault_last_refresh: None,
            secret_filter_status: SecretFilterStatus::All,
            secret_form_key: String::new(),
            secret_form_username: String::new(),
            secret_form_value: String::new(),
            secret_form_description: String::new(),
            secret_form_editing_existing: false,
            secret_search_query: String::new(),
            updater_tx,
            updater_rx,
            show_update_modal: false,
            available_update: None,
            update_checking: false,
            update_downloading: false,
            update_progress: 0.0,
            update_status_text: String::new(),
            update_error: None,
            pending_update_restart: None,
            last_sync_check: Instant::now(),
            status_banner: None,
            status_banner_time: None,
            sidebar_collapsed: std::env::var("SHIPLY_SIDEBAR_COLLAPSED").is_ok(),
            window_maximized: false,
            pending_delete_confirm: None,
            fonts_initialized: false,
            logo_texture: None,
            project_switch_loading_id: None,
            project_switch_loading_start: None,
            sidebar_refreshing_until: None,
            dark_mode: false,
            sidebar_drawer_open: false,
            is_version_input_focused: false,
            step_scp_sources_buffer: None,
            server_env_source_buffer: None,
            server_http_proxy_buffer: None,
            refresh_anim_start: None,
            #[cfg(debug_assertions)]
            dpi_diagnostics_logged: false,
            screenshot_frames: 0,
            git_inspect_tx,
            git_inspect_rx,
        };
        if let Some(notice) = migration_notice {
            app.status_banner = Some((notice, true));
        }
        app.storage.sync_project_vault_targets_to_secrets();
        app.refresh_selected_project_metadata(true);
        app
    }

    /// Saves the secret currently in the Secret Management form directly to the OS Credential Manager
    /// and updates the non-sensitive metadata in `AppStorage`.
    pub fn save_secret_to_os_vault(&mut self) -> Result<String, String> {
        let key = self.secret_form_key.trim().to_string();
        if key.is_empty() {
            return Err("Secret Name / Target Key cannot be empty.".to_string());
        }
        if self.secret_form_value.is_empty() {
            return Err("Secret Value / Password cannot be empty.".to_string());
        }

        let username_opt = {
            let u = self.secret_form_username.trim();
            if u.is_empty() {
                None
            } else {
                Some(u.to_string())
            }
        };

        write_os_vault_credential(&key, username_opt.as_deref(), &self.secret_form_value)?;

        let entry = SecretEntry::new(
            key.clone(),
            username_opt,
            self.secret_form_description.trim(),
        );
        self.storage.upsert_secret_metadata(entry);

        // Wipe plaintext secret value from memory immediately after storing in OS Vault
        self.secret_form_value.clear();
        self.secret_form_key.clear();
        self.secret_form_username.clear();
        self.secret_form_description.clear();
        self.secret_form_editing_existing = false;

        self.refresh_selected_project_metadata(false);
        self.refresh_secret_vault_cache();
        let msg = format!("Saved secret '{key}' to OS Credential Manager.");
        self.status_banner = Some((msg.clone(), true));
        self.status_banner_time = Some(Instant::now());
        Ok(msg)
    }

    /// Removes a secret from both the OS Credential Manager and `AppStorage` metadata.
    pub fn delete_secret_from_os_vault(&mut self, key: &str) -> Result<(), String> {
        let trimmed = key.trim();
        let res = delete_os_vault_credential(trimmed);
        self.storage.remove_secret_metadata(trimmed);
        self.refresh_selected_project_metadata(false);
        self.refresh_secret_vault_cache();
        match res {
            Ok(()) => {
                self.status_banner = Some((
                    format!("Deleted secret '{trimmed}' from OS Credential Manager."),
                    true,
                ));
                self.status_banner_time = Some(Instant::now());
                Ok(())
            }
            Err(err) => {
                self.status_banner = Some((err.clone(), false));
                self.status_banner_time = Some(Instant::now());
                Err(err)
            }
        }
    }

    /// Opens the Secret Management page with the form pre-filled for creating or updating a specific secret key.
    pub fn open_secret_management_for_target(
        &mut self,
        key: &str,
        username: &str,
        description: &str,
    ) {
        self.secret_form_key = key.trim().to_string();
        self.secret_form_username = username.trim().to_string();
        self.secret_form_description = description.trim().to_string();
        self.secret_form_value.clear();
        self.secret_form_editing_existing = !self.secret_form_key.is_empty();
        self.secret_form_show_password = false;
        self.selection = SidebarSelection::SecretManagement;
        self.show_secret_modal = true;
        self.secret_vault_cache_dirty = true;
    }

    /// Clears form state and opens the modal to create a new secret.
    pub fn open_new_secret_modal(&mut self) {
        self.secret_form_key.clear();
        self.secret_form_username.clear();
        self.secret_form_value.clear();
        self.secret_form_description.clear();
        self.secret_form_editing_existing = false;
        self.secret_form_show_password = false;
        self.show_secret_modal = true;
    }

    /// Refreshes the cached OS vault status for all configured secrets.
    pub(crate) fn refresh_secret_vault_cache(&mut self) {
        if self.storage.sync_project_vault_targets_to_secrets() {
            let _ = self.storage.save();
        }
        let mut cache = HashMap::new();
        for sec in &self.storage.secrets {
            if let Some(cred) = read_os_vault_credential(&sec.key) {
                cache.insert(
                    sec.key.clone(),
                    CachedVaultStatus {
                        in_vault: true,
                        username: cred.username.or_else(|| sec.username.clone()),
                    },
                );
            } else {
                cache.insert(
                    sec.key.clone(),
                    CachedVaultStatus {
                        in_vault: false,
                        username: sec.username.clone(),
                    },
                );
            }
        }
        self.secret_vault_cache = cache;
        self.secret_vault_cache_dirty = false;
        self.secret_vault_last_refresh = Some(Instant::now());
    }

    pub fn theme(&self) -> Theme {
        if self.dark_mode {
            Theme::dark()
        } else {
            Theme::light()
        }
    }

    pub fn set_status_banner(&mut self, message: String, is_success: bool) {
        self.status_banner = Some((message, is_success));
        self.status_banner_time = Some(Instant::now());
    }

    pub fn is_env_deploying(&self, project_id: &str, env_key: &str) -> bool {
        self.active_deployments
            .keys()
            .any(|(p, e)| p == project_id && e == env_key)
            || self.storage.deployments.iter().any(|d| {
                d.project_id == project_id
                    && d.environment == env_key
                    && d.status == DeploymentStatus::Running
            })
    }

    pub fn is_project_deploying(&self, project_id: &str) -> bool {
        self.active_deployments.keys().any(|(p, _)| p == project_id)
            || self.active_deployment_id.as_ref().is_some_and(|dep_id| {
                self.storage
                    .deployments
                    .iter()
                    .any(|d| &d.id == dep_id && d.project_id == project_id)
            })
            || self
                .storage
                .deployments
                .iter()
                .any(|d| d.project_id == project_id && d.status == DeploymentStatus::Running)
    }

    pub fn active_deployment_for_env(&self, project_id: &str, env_key: &str) -> Option<String> {
        self.active_deployments
            .iter()
            .find(|((p, e), _)| p == project_id && e == env_key)
            .map(|(_, id)| id.clone())
            .or_else(|| {
                self.storage
                    .deployments
                    .iter()
                    .find(|d| {
                        d.project_id == project_id
                            && d.environment == env_key
                            && d.status == DeploymentStatus::Running
                    })
                    .map(|d| d.id.clone())
            })
    }

    pub fn active_deployment_for_project(&self, project_id: &str) -> Option<String> {
        self.active_deployments
            .iter()
            .find(|((p, _), _)| p == project_id)
            .map(|(_, id)| id.clone())
            .or_else(|| {
                self.storage
                    .deployments
                    .iter()
                    .find(|d| d.project_id == project_id && d.status == DeploymentStatus::Running)
                    .map(|d| d.id.clone())
            })
    }

    pub fn is_deployment_active(&self, deployment_id: &str) -> bool {
        self.active_deployments
            .values()
            .any(|id| id == deployment_id)
            || self.active_deployment_id.as_deref() == Some(deployment_id)
            || self
                .storage
                .deployments
                .iter()
                .any(|d| d.id == deployment_id && d.status == DeploymentStatus::Running)
    }

    pub fn has_active_deployments(&self) -> bool {
        !self.active_deployments.is_empty()
            || self.active_deployment_id.is_some()
            || self
                .storage
                .deployments
                .iter()
                .any(|d| d.status == DeploymentStatus::Running)
    }

    pub fn cancel_deployment(&mut self, dep_id: &str) {
        self.active_deployments.retain(|_, id| id != dep_id);
        if self.active_deployment_id.as_deref() == Some(dep_id) {
            self.active_deployment_id = self.active_deployments.values().next().cloned();
        }

        let now = chrono::Utc::now();
        let mut updated_record = None;
        if let Some(dep) = self.storage.deployments.iter_mut().find(|d| d.id == dep_id) {
            dep.status = DeploymentStatus::Cancelled;
            dep.finished_at = Some(now);
            dep.duration_ms = Some((now - dep.started_at).num_milliseconds().max(0) as u64);
            for step_log in &mut dep.step_logs {
                if step_log.status == DeploymentStatus::Running {
                    step_log.status = DeploymentStatus::Cancelled;
                    if !step_log.output.is_empty() && !step_log.output.ends_with('\n') {
                        step_log.output.push('\n');
                    }
                    step_log
                        .output
                        .push_str("[CANCELLED] Deployment was cancelled by user.");
                }
            }
            updated_record = Some(dep.clone());
        }

        if let Some(rec) = updated_record {
            let proj_name = rec.project_name.clone();
            let env_badge = format_env_badge(&rec.environment);
            self.storage.upsert_deployment(rec);
            self.refresh_selected_project_metadata(true);
            self.status_banner = Some((
                format!("Cancelled deployment of {proj_name} ({env_badge})."),
                false,
            ));
        }
    }

    /// Configures typography fonts: bundles Inter (Regular, Medium, SemiBold) and JetBrains Mono
    /// with system Segoe UI fallback on Windows. Supports runtime Font A/B testing via `SHIPLY_UI_FONT`.
    pub(crate) fn install_fonts(ctx: &egui::Context) {
        #[allow(unused_mut)]
        let mut fonts = FontDefinitions::default();

        let active_font = if let Ok(val) = std::env::var("SHIPLY_UI_FONT") {
            if val.eq_ignore_ascii_case("segoeui") || val.eq_ignore_ascii_case("segoe") {
                UiFont::SegoeUi
            } else {
                UiFont::Inter
            }
        } else {
            UI_FONT
        };

        // 1. Embedded Inter fonts (SIL OFL)
        fonts.font_data.insert(
            "inter-regular".to_owned(),
            Arc::new(FontData::from_static(include_bytes!(
                "../../assets/fonts/Inter-Regular.ttf"
            ))),
        );
        fonts.font_data.insert(
            "inter-medium".to_owned(),
            Arc::new(FontData::from_static(include_bytes!(
                "../../assets/fonts/Inter-Medium.ttf"
            ))),
        );
        fonts.font_data.insert(
            "inter-semibold".to_owned(),
            Arc::new(FontData::from_static(include_bytes!(
                "../../assets/fonts/Inter-SemiBold.ttf"
            ))),
        );

        // 2. Embedded JetBrains Mono fonts (SIL OFL)
        fonts.font_data.insert(
            "jbmono-regular".to_owned(),
            Arc::new(FontData::from_static(include_bytes!(
                "../../assets/fonts/JetBrainsMono-Regular.ttf"
            ))),
        );
        fonts.font_data.insert(
            "jbmono-medium".to_owned(),
            Arc::new(FontData::from_static(include_bytes!(
                "../../assets/fonts/JetBrainsMono-Medium.ttf"
            ))),
        );

        // 3. System fallback on Windows (Segoe UI family + Consolas)
        #[cfg(windows)]
        {
            if let Ok(win_dir) = std::env::var("SystemRoot") {
                let fonts_dir = PathBuf::from(win_dir).join("Fonts");
                if let Ok(bytes) = fs::read(fonts_dir.join("segoeui.ttf")) {
                    fonts
                        .font_data
                        .insert("segoeui".to_owned(), Arc::new(FontData::from_owned(bytes)));
                }
                if let Ok(bytes) = fs::read(fonts_dir.join("seguisb.ttf")) {
                    fonts
                        .font_data
                        .insert("seguisb".to_owned(), Arc::new(FontData::from_owned(bytes)));
                }
                if let Ok(bytes) = fs::read(fonts_dir.join("segoeuib.ttf")) {
                    fonts
                        .font_data
                        .insert("segoeuib".to_owned(), Arc::new(FontData::from_owned(bytes)));
                }
                if let Ok(bytes) = fs::read(fonts_dir.join("consola.ttf")) {
                    fonts
                        .font_data
                        .insert("consola".to_owned(), Arc::new(FontData::from_owned(bytes)));
                }
            }
        }

        let fallback_prop: Vec<String> = fonts
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default();
        let fallback_mono: Vec<String> = fonts
            .families
            .get(&FontFamily::Monospace)
            .cloned()
            .unwrap_or_default();

        match active_font {
            UiFont::Inter => {
                let mut prop = vec!["inter-regular".to_owned()];
                #[cfg(windows)]
                prop.push("segoeui".to_owned());
                prop.extend(fallback_prop.clone());
                fonts.families.insert(FontFamily::Proportional, prop);

                let mut mono = vec!["jbmono-regular".to_owned()];
                #[cfg(windows)]
                mono.push("consola".to_owned());
                mono.extend(fallback_mono.clone());
                fonts.families.insert(FontFamily::Monospace, mono);

                let mut med = vec!["inter-medium".to_owned()];
                #[cfg(windows)]
                med.push("seguisb".to_owned());
                med.extend(fallback_prop.clone());
                fonts
                    .families
                    .insert(FontFamily::Name("medium".into()), med);

                let mut sbold = vec!["inter-semibold".to_owned()];
                #[cfg(windows)]
                sbold.push("segoeuib".to_owned());
                sbold.extend(fallback_prop);
                fonts
                    .families
                    .insert(FontFamily::Name("semibold".into()), sbold);

                let mut mono_med = vec!["jbmono-medium".to_owned()];
                #[cfg(windows)]
                mono_med.push("consola".to_owned());
                mono_med.extend(fallback_mono);
                fonts
                    .families
                    .insert(FontFamily::Name("mono-medium".into()), mono_med);
            }
            UiFont::SegoeUi => {
                let mut prop = Vec::new();
                #[cfg(windows)]
                prop.push("segoeui".to_owned());
                prop.push("inter-regular".to_owned());
                prop.extend(fallback_prop.clone());
                fonts.families.insert(FontFamily::Proportional, prop);

                let mut mono = Vec::new();
                #[cfg(windows)]
                mono.push("consola".to_owned());
                mono.push("jbmono-regular".to_owned());
                mono.extend(fallback_mono.clone());
                fonts.families.insert(FontFamily::Monospace, mono);

                let mut med = Vec::new();
                #[cfg(windows)]
                med.push("seguisb".to_owned());
                med.push("inter-medium".to_owned());
                med.extend(fallback_prop.clone());
                fonts
                    .families
                    .insert(FontFamily::Name("medium".into()), med);

                let mut sbold = Vec::new();
                #[cfg(windows)]
                sbold.push("segoeuib".to_owned());
                sbold.push("inter-semibold".to_owned());
                sbold.extend(fallback_prop);
                fonts
                    .families
                    .insert(FontFamily::Name("semibold".into()), sbold);

                let mut mono_med = Vec::new();
                #[cfg(windows)]
                mono_med.push("consola".to_owned());
                mono_med.push("jbmono-medium".to_owned());
                mono_med.extend(fallback_mono);
                fonts
                    .families
                    .insert(FontFamily::Name("mono-medium".into()), mono_med);
            }
        }

        ctx.set_fonts(fonts);
    }

    pub(crate) fn install_system_fonts(ctx: &egui::Context) {
        Self::install_fonts(ctx);
    }

    pub fn mark_config_dirty(&mut self) {
        self.dirty_save_pending = true;
        self.last_edit_time = Some(Instant::now());
    }

    pub fn flush_pending_save(&mut self) {
        if self.dirty_save_pending {
            let _ = self.storage.save();
            self.dirty_save_pending = false;
            self.last_edit_time = None;
        }
    }

    pub fn process_pending_git_inspect(&mut self) -> bool {
        let mut received = false;
        while let Ok(res) = self.git_inspect_rx.try_recv() {
            if let SidebarSelection::Project(cur_id) = &self.selection {
                if cur_id == &res.project_id {
                    self.cached_pkg_version = res.pkg_version;
                    self.cached_git_meta = res.git_meta;
                    self.cached_git_branches = res.git_branches;
                    self.cached_git_changes = res.git_changes;

                    if let Some(current_branch) = &self.cached_git_meta.branch {
                        self.selected_git_branch = current_branch.clone();
                        if !self.cached_git_branches.contains(current_branch) {
                            self.cached_git_branches.insert(0, current_branch.clone());
                        }
                    } else if let Some(first_b) = self.cached_git_branches.first() {
                        self.selected_git_branch = first_b.clone();
                    } else {
                        self.selected_git_branch.clear();
                    }

                    if res.update_version_input {
                        self.deploy_version_input = self.storage.next_auto_bumped_version(
                            cur_id,
                            &self.selected_env_key,
                            self.cached_pkg_version.as_deref(),
                        );
                    }

                    if self.project_switch_loading_id.as_deref() == Some(&res.project_id) {
                        self.project_switch_loading_id = None;
                        self.project_switch_loading_start = None;
                    }
                    received = true;
                }
            }
        }
        received
    }

    pub(crate) fn refresh_selected_project_metadata(&mut self, update_version_input: bool) {
        self.refresh_anim_start = Some(Instant::now());
        let SidebarSelection::Project(proj_id) = &self.selection else {
            return;
        };
        let Some(proj) = self.storage.projects.iter().find(|p| &p.id == proj_id) else {
            return;
        };

        if proj.get_env(&self.selected_env_key).is_none() {
            self.selected_env_key = proj
                .environments
                .first()
                .map(|e| e.key.clone())
                .unwrap_or_default();
        }

        let is_different_proj = self.cached_project_id.as_deref() != Some(&proj.id);
        if is_different_proj {
            self.history_visible_limit = 5;
            self.servers_edit_buffer = None;
            self.cached_pkg_version = None;
            self.cached_git_changes.clear();
            self.project_switch_loading_id = Some(proj.id.clone());
            self.project_switch_loading_start = Some(Instant::now());
        }
        self.cached_project_id = Some(proj.id.clone());

        if let Some(env_cfg) = proj.get_env(&self.selected_env_key) {
            self.cached_vault_found = read_os_vault_credential(&env_cfg.vault_target).is_some();
        } else {
            self.cached_vault_found = false;
        }

        // Ensure selected_deployment_id belongs strictly to this active project,
        // prioritizing any active/running deployment for this project & selected environment
        if let Some(active_dep_id) = self
            .active_deployment_for_env(proj_id, &self.selected_env_key)
            .or_else(|| self.active_deployment_for_project(proj_id))
        {
            self.selected_deployment_id = Some(active_dep_id);
        } else {
            let belongs_to_current_project = self
                .selected_deployment_id
                .as_ref()
                .and_then(|id| self.storage.deployments.iter().find(|d| &d.id == id))
                .is_some_and(|d| &d.project_id == proj_id);

            if !belongs_to_current_project {
                self.selected_deployment_id = self
                    .storage
                    .deployments
                    .iter()
                    .find(|d| &d.project_id == proj_id && d.environment == self.selected_env_key)
                    .or_else(|| {
                        self.storage
                            .deployments
                            .iter()
                            .find(|d| &d.project_id == proj_id)
                    })
                    .map(|d| d.id.clone());
            }
        }

        if update_version_input {
            self.deploy_version_input = self.storage.next_auto_bumped_version(
                proj_id,
                &self.selected_env_key,
                self.cached_pkg_version.as_deref(),
            );
        }

        // Dispatch heavy Git inspection to a background worker so project switching is instant (0ms latency, zero UI freeze)
        let tx = self.git_inspect_tx.clone();
        let proj_id_clone = proj.id.clone();
        let path = PathBuf::from(&proj.local_path);
        let last_deploy_commit = self
            .storage
            .last_succeeded_deployment(proj_id, &self.selected_env_key)
            .and_then(|d| d.git_commit.clone());

        std::thread::spawn(move || {
            let pkg_version = read_package_json_version(&path);
            let git_meta = read_git_metadata(&path);
            let git_branches = list_git_branches(&path);
            let git_changes = read_git_code_changes(&path, last_deploy_commit.as_deref());

            let _ = tx.send(GitInspectionResult {
                project_id: proj_id_clone,
                pkg_version,
                git_meta,
                git_branches,
                git_changes,
                update_version_input,
            });
        });
    }

    pub(crate) fn poll_executor_events(&mut self, ctx: &egui::Context) {
        let mut received = false;
        while let Ok(event) = self.event_rx.try_recv() {
            received = true;
            match event {
                ExecutorEvent::Updated(record) => {
                    if self
                        .storage
                        .deployments
                        .iter()
                        .any(|d| d.id == record.id && d.status == DeploymentStatus::Cancelled)
                    {
                        continue;
                    }
                    let is_current = match &self.selection {
                        SidebarSelection::Project(pid) => {
                            if pid != &record.project_id {
                                false
                            } else if let Some(sel_id) = &self.selected_deployment_id {
                                sel_id == &record.id
                            } else {
                                self.selected_env_key == record.environment
                            }
                        }
                        SidebarSelection::AllDeployments | SidebarSelection::SecretManagement => {
                            if let Some(sel_id) = &self.selected_deployment_id {
                                sel_id == &record.id
                            } else {
                                true
                            }
                        }
                    };
                    if is_current {
                        self.selected_deployment_id = Some(record.id.clone());
                    }
                    self.storage.upsert_deployment(record);
                }
                ExecutorEvent::Finished(record) => {
                    if self
                        .storage
                        .deployments
                        .iter()
                        .any(|d| d.id == record.id && d.status == DeploymentStatus::Cancelled)
                    {
                        continue;
                    }
                    let ok = record.status == DeploymentStatus::Succeeded;
                    let action_prefix = if record.is_rollback {
                        "Rollback"
                    } else {
                        "Deployment"
                    };
                    let msg = format!(
                        "{action_prefix} of {} ({}) v{} {}",
                        record.project_name,
                        format_env_badge(&record.environment),
                        record.version,
                        record.status.label()
                    );
                    self.status_banner = Some((msg.clone(), ok));
                    let is_current = match &self.selection {
                        SidebarSelection::Project(pid) => {
                            if pid != &record.project_id {
                                false
                            } else if let Some(sel_id) = &self.selected_deployment_id {
                                sel_id == &record.id
                            } else {
                                self.selected_env_key == record.environment
                            }
                        }
                        SidebarSelection::AllDeployments | SidebarSelection::SecretManagement => {
                            if let Some(sel_id) = &self.selected_deployment_id {
                                sel_id == &record.id
                            } else {
                                true
                            }
                        }
                    };
                    if is_current {
                        self.selected_deployment_id = Some(record.id.clone());
                    }
                    self.active_deployments.retain(|_, id| id != &record.id);
                    if self.active_deployment_id.as_deref() == Some(&record.id) {
                        self.active_deployment_id =
                            self.active_deployments.values().next().cloned();
                    }
                    self.storage.upsert_deployment(record);
                    if is_current {
                        self.refresh_selected_project_metadata(true);
                    }

                    if let Some(next_req) = self.bulk_deploy_queue.pop_front() {
                        let done = self
                            .bulk_total_count
                            .saturating_sub(self.bulk_deploy_queue.len() + 1);
                        let step_idx = done + 1;
                        let total = self.bulk_total_count.max(step_idx);
                        let next_msg = format!(
                            "Bulk Deploy [{step_idx}/{total}]: Deploying {} ({}) v{}... (Previous: {msg})",
                            next_req.project_name,
                            format_env_badge(&next_req.env_config.key),
                            next_req.version
                        );
                        self.status_banner = Some((next_msg, ok));
                        self.start_deployment(next_req);
                    } else if self.bulk_total_count > 0 {
                        let total = self.bulk_total_count;
                        self.bulk_total_count = 0;
                        self.status_banner = Some((
                            format!("Bulk Deployment finished ({total} project(s) deployed). Last: {msg}"),
                            ok,
                        ));
                    }

                    // If all active deployments and queued bulk items are finished, apply any deferred update restart
                    if !self.has_active_deployments() && self.bulk_deploy_queue.is_empty() {
                        if let Some((new_ver, exe_path)) = self.pending_update_restart.take() {
                            if let Err(err) = restart_application(&exe_path) {
                                self.update_error = Some(err);
                            } else {
                                self.status_banner = Some((
                                    format!("All deployments completed! Updated to Shiply v{new_ver}. Restarting application..."),
                                    true,
                                ));
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        }
                    }
                }
            }
        }

        // Poll GitHub Releases Auto-Updater events
        while let Ok(upd_event) = self.updater_rx.try_recv() {
            received = true;
            match upd_event {
                UpdaterEvent::Checking => {
                    self.update_checking = true;
                }
                UpdaterEvent::UpdateAvailable { release, manual: _ } => {
                    self.update_checking = false;
                    self.update_error = None;
                    self.available_update = Some(release);
                    self.show_update_modal = true;
                }
                UpdaterEvent::UpToDate {
                    current_version,
                    manual,
                } => {
                    self.update_checking = false;
                    if manual {
                        self.status_banner = Some((
                            format!(
                                "Shiply v{current_version} is up to date with GitHub Releases ({GITHUB_REPO})."
                            ),
                            true,
                        ));
                    }
                }
                UpdaterEvent::CheckFailed { error, manual } => {
                    self.update_checking = false;
                    if manual {
                        self.status_banner = Some((error, false));
                    }
                }
                UpdaterEvent::DownloadProgress {
                    fraction,
                    status_text,
                    ..
                } => {
                    self.update_downloading = true;
                    self.update_progress = fraction;
                    self.update_status_text = status_text;
                }
                UpdaterEvent::ReadyToRestart {
                    new_version,
                    exe_path,
                } => {
                    self.update_downloading = false;
                    self.update_progress = 1.0;
                    if self.has_active_deployments() {
                        self.pending_update_restart = Some((new_version.clone(), exe_path));
                        self.show_update_modal = false;
                        self.status_banner = Some((
                            format!("Shiply v{new_version} update is downloaded! Active deployments will continue running uninterrupted before restart."),
                            true,
                        ));
                    } else if let Err(err) = restart_application(&exe_path) {
                        self.update_error = Some(err);
                    } else {
                        self.status_banner = Some((
                            format!("Updated to Shiply v{new_version}! Restarting application..."),
                            true,
                        ));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                UpdaterEvent::UpdateFailed { error } => {
                    self.update_downloading = false;
                    self.update_error = Some(error);
                }
            }
        }

        // Throttled check (once every 2s) for files pasted into the app directory
        if !self.has_active_deployments()
            && self.last_sync_check.elapsed() >= Duration::from_secs(2)
        {
            self.last_sync_check = Instant::now();
            if let Some(sync_msg) = self.storage.sync_from_app_directory() {
                if matches!(self.selection, SidebarSelection::AllDeployments) {
                    if let Some(first_proj) = self.storage.projects.first() {
                        self.selection = SidebarSelection::Project(first_proj.id.clone());
                    }
                }
                self.refresh_selected_project_metadata(true);
                self.status_banner = Some((sync_msg, true));
            }
        }

        if self.process_pending_git_inspect() {
            received = true;
        }

        if received
            || self.has_active_deployments()
            || self.update_checking
            || self.update_downloading
            || self.project_switch_loading_id.is_some()
        {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }

    pub(crate) fn start_deployment(&mut self, req: DeploymentRequest) {
        self.flush_pending_save();
        if self.is_env_deploying(&req.project_id, &req.env_config.key) {
            self.status_banner = Some((
                format!(
                    "A deployment for '{}' ({}) is already running. Please wait for it to finish.",
                    req.project_name,
                    format_env_badge(&req.env_config.key)
                ),
                false,
            ));
            return;
        }

        let dep_id = req.deployment_id.clone();
        let key = (req.project_id.clone(), req.env_config.key.clone());
        self.active_deployments.insert(key, dep_id.clone());
        self.active_deployment_id = Some(dep_id.clone());
        self.selected_deployment_id = Some(dep_id);
        self.active_tab = WorkspaceTab::HistoryAndLogs;
        self.console_collapsed = false;
        self.collapsed_step_indices.clear();
        let tx = self.event_tx.clone();

        thread::spawn(move || {
            run_deployment(&req, Some(&tx));
        });
    }

    pub(crate) fn trigger_current_environment_deploy(&mut self) {
        let SidebarSelection::Project(proj_id) = &self.selection else {
            return;
        };
        let Some(proj) = self.storage.projects.iter().find(|p| &p.id == proj_id) else {
            return;
        };
        let Some(env_cfg) = proj.get_env(&self.selected_env_key).cloned() else {
            return;
        };

        let version = if self.deploy_version_input.trim().is_empty() {
            "1.0.0".to_string()
        } else {
            self.deploy_version_input.trim().to_string()
        };

        let manual_pw = if self.manual_ssh_password.is_empty() {
            None
        } else {
            Some(self.manual_ssh_password.clone())
        };

        let prev_commit = self
            .storage
            .last_succeeded_deployment(&proj.id, &env_cfg.key)
            .and_then(|d| d.git_commit.clone());

        let target_branch = if self.selected_git_branch.trim().is_empty() {
            None
        } else {
            Some(self.selected_git_branch.trim().to_string())
        };

        let req = DeploymentRequest {
            deployment_id: Uuid::new_v4().to_string(),
            project_id: proj.id.clone(),
            project_name: proj.name.clone(),
            local_path: PathBuf::from(&proj.local_path),
            env_config: env_cfg,
            version,
            bump_package_json: self.bump_package_json,
            create_git_tag_on_success: self.create_git_tag,
            manual_password: manual_pw,
            skip_ssh_preflight: false,
            target_git_branch: target_branch,
            rollback_commit: None,
            previous_commit: prev_commit,
            is_rollback: false,
        };

        let _ = self.storage.save();
        self.start_deployment(req);
    }

    pub(crate) fn open_bulk_deploy_modal(&mut self) {
        if self.bulk_target_env_key.is_empty()
            || !self
                .storage
                .projects
                .iter()
                .any(|p| p.get_env(&self.bulk_target_env_key).is_some())
        {
            self.bulk_target_env_key = self
                .storage
                .projects
                .iter()
                .flat_map(|p| p.environments.first())
                .map(|e| e.key.clone())
                .next()
                .unwrap_or_else(|| "dev".to_string());
        }

        // Clean up any deleted project IDs from selection
        let valid_ids: HashSet<String> =
            self.storage.projects.iter().map(|p| p.id.clone()).collect();
        self.bulk_selected_projects
            .retain(|id| valid_ids.contains(id));

        if self.bulk_selected_projects.is_empty() {
            for proj in &self.storage.projects {
                if !proj.environments.is_empty() {
                    self.bulk_selected_projects.insert(proj.id.clone());
                }
            }
        }

        self.sync_bulk_project_versions();
        self.show_bulk_deploy_modal = true;
    }

    pub(crate) fn sync_bulk_project_versions(&mut self) {
        for proj in &self.storage.projects {
            let env_key = proj
                .get_env(&self.bulk_target_env_key)
                .or_else(|| proj.environments.first())
                .map(|e| e.key.clone())
                .unwrap_or_else(|| self.bulk_target_env_key.clone());
            let pkg_ver = read_package_json_version(&PathBuf::from(&proj.local_path));
            let auto_ver =
                self.storage
                    .next_auto_bumped_version(&proj.id, &env_key, pkg_ver.as_deref());
            self.bulk_project_versions.insert(proj.id.clone(), auto_ver);
        }
    }

    pub(crate) fn trigger_bulk_deployment(&mut self) {
        if self.has_active_deployments() {
            self.status_banner = Some((
                "A deployment is already running. Please wait for it to finish.".to_string(),
                false,
            ));
            return;
        }

        let manual_pw = if self.manual_ssh_password.is_empty() {
            None
        } else {
            Some(self.manual_ssh_password.clone())
        };

        let mut requests = VecDeque::new();
        for proj in &self.storage.projects {
            if !self.bulk_selected_projects.contains(&proj.id) {
                continue;
            }
            let Some(env_cfg) = proj
                .get_env(&self.bulk_target_env_key)
                .or_else(|| proj.environments.first())
                .cloned()
            else {
                continue;
            };

            let version = self
                .bulk_project_versions
                .get(&proj.id)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| {
                    let pkg_ver = read_package_json_version(&PathBuf::from(&proj.local_path));
                    self.storage.next_auto_bumped_version(
                        &proj.id,
                        &env_cfg.key,
                        pkg_ver.as_deref(),
                    )
                });

            let prev_commit = self
                .storage
                .last_succeeded_deployment(&proj.id, &env_cfg.key)
                .and_then(|d| d.git_commit.clone());

            requests.push_back(DeploymentRequest {
                deployment_id: Uuid::new_v4().to_string(),
                project_id: proj.id.clone(),
                project_name: proj.name.clone(),
                local_path: PathBuf::from(&proj.local_path),
                env_config: env_cfg,
                version,
                bump_package_json: self.bulk_bump_pkg,
                create_git_tag_on_success: self.bulk_create_tag,
                manual_password: manual_pw.clone(),
                skip_ssh_preflight: false,
                target_git_branch: None,
                rollback_commit: None,
                previous_commit: prev_commit,
                is_rollback: false,
            });
        }

        if requests.is_empty() {
            self.status_banner = Some((
                "Select at least one project with a configured environment to run Bulk Deployment."
                    .to_string(),
                false,
            ));
            return;
        }

        let total = requests.len();
        self.bulk_total_count = total;
        self.bulk_deploy_queue = requests;
        self.show_bulk_deploy_modal = false;

        if let Some(first_req) = self.bulk_deploy_queue.pop_front() {
            self.status_banner = Some((
                format!(
                    "Bulk Deploy [1/{total}]: Starting {} ({}) v{}...",
                    first_req.project_name,
                    format_env_badge(&first_req.env_config.key),
                    first_req.version
                ),
                true,
            ));
            self.start_deployment(first_req);
        }
    }

    pub(crate) fn trigger_redeploy_from_record(
        &mut self,
        record: &DeploymentRecord,
        is_rollback: bool,
    ) {
        let Some(proj) = self
            .storage
            .projects
            .iter()
            .find(|p| p.id == record.project_id)
        else {
            self.status_banner = Some((
                format!("Project '{}' not found.", record.project_name),
                false,
            ));
            return;
        };
        let Some(mut env_cfg) = proj
            .get_env(&record.environment)
            .or_else(|| proj.environments.first())
            .cloned()
        else {
            self.status_banner = Some((
                format!("Environment '{}' not found.", record.environment),
                false,
            ));
            return;
        };

        if !record.executed_steps.is_empty() {
            env_cfg.steps = record.executed_steps.clone();
        }

        let manual_pw = if self.manual_ssh_password.is_empty() {
            None
        } else {
            Some(self.manual_ssh_password.clone())
        };

        let prev_commit = self
            .storage
            .last_succeeded_deployment(&proj.id, &env_cfg.key)
            .and_then(|d| d.git_commit.clone());

        let req = DeploymentRequest {
            deployment_id: Uuid::new_v4().to_string(),
            project_id: proj.id.clone(),
            project_name: proj.name.clone(),
            local_path: PathBuf::from(&proj.local_path),
            env_config: env_cfg,
            version: record.version.clone(),
            bump_package_json: true,
            create_git_tag_on_success: false,
            manual_password: manual_pw,
            skip_ssh_preflight: false,
            target_git_branch: record.git_branch.clone(),
            rollback_commit: if is_rollback {
                record.git_commit.clone()
            } else {
                None
            },
            previous_commit: prev_commit,
            is_rollback,
        };

        self.deploy_version_input = record.version.clone();
        self.start_deployment(req);
    }

    pub(crate) fn apply_ps1_auto_import_to_modal(&mut self, raw_path: &str) {
        let path = PathBuf::from(raw_path.trim().trim_matches('"'));
        match import_project_from_path(&path, self.new_project_import_siblings) {
            Ok(imported) => {
                self.new_project_name = imported.name;
                self.new_project_path = imported.local_path;
                self.new_project_desc = imported.description;
                let env_names: Vec<String> = imported
                    .environments
                    .iter()
                    .map(|e| format!("{} ({})", e.name, e.app_name))
                    .collect();
                self.new_project_imported_envs = imported.environments;
                self.new_project_import_notice = Some(format!(
                    "Auto-imported {} environment(s): {}",
                    env_names.len(),
                    env_names.join(", ")
                ));
            }
            Err(err) => {
                self.new_project_import_notice = Some(format!("Import error: {err}"));
            }
        }
    }

    pub fn configure_workspace_theme(ctx: &egui::Context, dark: bool) {
        Self::install_fonts(ctx);
        let theme = if dark { Theme::dark() } else { Theme::light() };
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(10.0, 8.0);
        style.spacing.button_padding = Vec2::new(12.0, 6.0);
        style.spacing.indent = 16.0;
        style.spacing.scroll.bar_outer_margin = 0.0;
        style.spacing.scroll.bar_inner_margin = 4.0;
        style.spacing.scroll.bar_width = 8.0;

        // Structured Typography Hierarchy (Brief 002 Section 2.2)
        let mut text_styles = BTreeMap::new();
        text_styles.insert(TextStyle::Heading, Typo::H2.font());
        text_styles.insert(TextStyle::Body, Typo::Body.font());
        text_styles.insert(TextStyle::Button, Typo::Label.font());
        text_styles.insert(TextStyle::Small, Typo::Small.font());
        text_styles.insert(TextStyle::Monospace, Typo::Code.font());
        style.text_styles = text_styles;

        let mut visuals = if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.panel_fill = theme.bg;
        visuals.window_fill = theme.surface;
        visuals.faint_bg_color = theme.surface_2;
        visuals.extreme_bg_color = theme.surface;
        visuals.override_text_color = Some(theme.text);

        visuals.selection.bg_fill = theme.primary_soft;
        visuals.selection.stroke = Stroke::new(1.5_f32, theme.primary);

        visuals.widgets.noninteractive.bg_fill = theme.surface;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, theme.border);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, theme.text_muted);

        visuals.widgets.inactive.bg_fill = theme.surface;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, theme.border_strong);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, theme.text);
        visuals.widgets.inactive.corner_radius = R_MD.into();

        visuals.widgets.hovered.bg_fill = theme.surface_2;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, theme.primary);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, theme.text);
        visuals.widgets.hovered.corner_radius = R_MD.into();

        visuals.widgets.active.bg_fill = theme.surface_2;
        visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, theme.primary_active);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, theme.primary_active);
        visuals.widgets.active.corner_radius = R_MD.into();

        visuals.widgets.open.bg_fill = theme.surface;
        visuals.widgets.open.bg_stroke = Stroke::new(1.5_f32, theme.primary);
        visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, theme.text);
        visuals.widgets.open.corner_radius = R_MD.into();

        visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);

        style.visuals = visuals;
        ctx.set_style(style);
    }

    pub fn configure_light_workspace_style(ctx: &egui::Context) {
        Self::configure_workspace_theme(ctx, false);
    }
}

fn save_color_image_as_bmp(
    image: &egui::ColorImage,
    path: &std::path::Path,
) -> std::io::Result<()> {
    use std::io::Write;
    let [w, h] = image.size;
    let pixel_bytes_len = (w * h * 4) as u32;
    let file_size = 54 + pixel_bytes_len;
    let mut file = std::fs::File::create(path)?;

    // BITMAPFILEHEADER (14 bytes)
    file.write_all(b"BM")?;
    file.write_all(&file_size.to_le_bytes())?;
    file.write_all(&0u32.to_le_bytes())?;
    file.write_all(&54u32.to_le_bytes())?;

    // BITMAPINFOHEADER (40 bytes)
    file.write_all(&40u32.to_le_bytes())?;
    file.write_all(&(w as i32).to_le_bytes())?;
    file.write_all(&(-(h as i32)).to_le_bytes())?; // negative for top-down bitmap
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&32u16.to_le_bytes())?;
    file.write_all(&0u32.to_le_bytes())?;
    file.write_all(&pixel_bytes_len.to_le_bytes())?;
    file.write_all(&0i32.to_le_bytes())?;
    file.write_all(&0i32.to_le_bytes())?;
    file.write_all(&0u32.to_le_bytes())?;
    file.write_all(&0u32.to_le_bytes())?;

    // Pixels (BGRA)
    let mut raw_pixels = Vec::with_capacity(w * h * 4);
    for p in &image.pixels {
        raw_pixels.push(p.b());
        raw_pixels.push(p.g());
        raw_pixels.push(p.r());
        raw_pixels.push(p.a());
    }
    file.write_all(&raw_pixels)?;
    file.flush()?;
    Ok(())
}

impl eframe::App for ShiplyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.fonts_initialized {
            Self::install_system_fonts(ctx);
            self.fonts_initialized = true;
        }
        if self.logo_texture.is_none() {
            self.logo_texture = Some(load_shiply_logo_texture(ctx));
        }

        if let Ok(scale_str) = std::env::var("SHIPLY_ZOOM_FACTOR") {
            if let Ok(scale) = scale_str.parse::<f32>() {
                ctx.set_zoom_factor(scale);
            }
        }

        if let Ok(path) = std::env::var("SHIPLY_SCREENSHOT_PATH") {
            ctx.input_mut(|i| i.focused = true);
            self.screenshot_frames += 1;
            if self.screenshot_frames == 5 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            }
            for event in &ctx.input(|i| i.raw.events.clone()) {
                if let egui::Event::Screenshot { image, .. } = event {
                    let bmp_path = std::path::PathBuf::from(&path);
                    if let Err(err) = save_color_image_as_bmp(image, &bmp_path) {
                        eprintln!("[Screenshot Error] Failed to save BMP: {err}");
                    } else {
                        eprintln!("[Screenshot Success] Saved frame to {}", bmp_path.display());
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            ctx.request_repaint();
        }

        self.poll_executor_events(ctx);
        Self::configure_light_workspace_style(ctx);

        if self.project_switch_loading_id.is_some() {
            ctx.request_repaint();
        }

        let refresh_angle = if let Some(start) = self.refresh_anim_start {
            let elapsed = start.elapsed().as_secs_f32();
            if elapsed < 0.65 {
                ctx.request_repaint();
                (elapsed / 0.65) * std::f32::consts::TAU
            } else {
                self.refresh_anim_start = None;
                0.0
            }
        } else {
            0.0
        };
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("refresh_anim_angle"), refresh_angle));

        if let Some(m) = ctx.input(|i| i.viewport().maximized) {
            self.window_maximized = m;
        }

        #[cfg(debug_assertions)]
        if !self.dpi_diagnostics_logged {
            let ppp = ctx.pixels_per_point();
            let native_ppp = ctx
                .input(|i| i.viewport().native_pixels_per_point)
                .unwrap_or(ppp);
            let zoom = ctx.zoom_factor();
            let screen_rect = ctx.screen_rect();
            let phys_w = screen_rect.width() * ppp;
            let phys_h = screen_rect.height() * ppp;
            eprintln!(
                "[DPI Diagnostics] pixels_per_point: {:.2}, native_pixels_per_point: {:.2}, zoom_factor: {:.2}, inner_size: [{:.1}, {:.1}] pt ([{:.0}, {:.0}] px)",
                ppp, native_ppp, zoom, screen_rect.width(), screen_rect.height(), phys_w, phys_h
            );
            self.dpi_diagnostics_logged = true;
        }

        self.render_termius_topbar(ctx);
        let screen_width = ctx.screen_rect().width();
        let is_narrow = screen_width < 760.0;

        if !is_narrow {
            self.sidebar_drawer_open = false;
            let sidebar_anim_t = ctx.animate_bool(
                egui::Id::new("sidebar_smooth_anim"),
                !self.sidebar_collapsed,
            );
            self.render_sidebar(
                ctx,
                if self.sidebar_collapsed {
                    0.25
                } else {
                    sidebar_anim_t.max(0.25)
                },
            );
        } else if self.sidebar_drawer_open {
            self.render_sidebar_drawer(ctx);
        }

        self.render_new_project_modal(ctx);
        self.render_edit_project_modal(ctx);
        self.render_new_environment_modal(ctx);
        self.render_bulk_deploy_modal(ctx);
        self.render_github_update_modal(ctx);
        self.render_delete_confirmation_modal(ctx);
        self.render_secret_modal(ctx);
        self.render_window_resize_handles(ctx);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(CANVAS_BG)
                    .inner_margin(Margin::ZERO),
            )
            .show(ctx, |ui| {
                let mut main_scroll = ScrollArea::vertical()
                    .id_salt("main_workspace_vertical_scroll")
                    .auto_shrink([false, false]);
                #[cfg(debug_assertions)]
                if std::env::var("SHIPLY_SCROLL_TO_BOTTOM").is_ok() {
                    main_scroll = main_scroll.vertical_scroll_offset(99999.0);
                }
                main_scroll.show(ui, |ui| {
                        let avail_w = ui.available_width();
                        let h_pad = 32.0_f32;
                        let col_w = (avail_w - h_pad * 2.0).clamp(280.0, MAX_CONTENT_WIDTH);
                        let left_offset = ((avail_w - col_w) / 2.0).max(h_pad);

                        ui.add_space(SP_24);
                        ui.horizontal(|ui| {
                            ui.add_space(left_offset);
                            ui.vertical(|ui| {
                                ui.set_width(col_w);
                                if let Some((msg, is_ok)) = &self.status_banner.clone() {
                                    let now = Instant::now();
                                    let banner_time = *self.status_banner_time.get_or_insert(now);
                                    if *is_ok && now.duration_since(banner_time).as_secs_f32() > 4.0 {
                                        self.status_banner = None;
                                        self.status_banner_time = None;
                                    } else {
                                        let bar_color = if *is_ok { SUCCESS } else { DANGER };
                                        let icon = if *is_ok { OutlineIcon::Check } else { OutlineIcon::Info };
                                        let icon_color = if *is_ok { SUCCESS } else { DANGER };

                                        egui::Area::new(egui::Id::new("floating_toast_area"))
                                            .order(egui::Order::Foreground)
                                            .anchor(Align2::RIGHT_TOP, Vec2::new(-24.0, 24.0))
                                            .show(ctx, |ui| {
                                                egui::Frame::new()
                                                    .fill(CARD_BG)
                                                    .stroke(Stroke::new(1.0_f32, CARD_BORDER))
                                                    .corner_radius(CornerRadius::same(R_MD as u8))
                                                    .shadow(Shadow {
                                                        offset: [0, 8],
                                                        blur: 24,
                                                        spread: 0,
                                                        color: SHADOW_MODAL,
                                                    })
                                                    .inner_margin(Margin::symmetric(14, 10))
                                                    .show(ui, |ui| {
                                                        ui.set_max_width(420.0);
                                                        ui.horizontal(|ui| {
                                                            let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(4.0, 24.0), Sense::hover());
                                                            ui.painter().rect_filled(bar_rect, CornerRadius::same(2), bar_color);
                                                            ui.add_space(SP_8);

                                                            let (ic_rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                                                            draw_outline_icon(ui.painter(), ic_rect.center(), icon, icon_color);
                                                            ui.add_space(SP_8);

                                                            ui.label(Typo::Body.rich(msg).color(TEXT_PRIMARY));

                                                            if let Some((_, exe_path)) = self.pending_update_restart.clone() {
                                                                ui.add_space(SP_8);
                                                                if kit::button(
                                                                    ui,
                                                                    ButtonVariant::Primary,
                                                                    ButtonSize::Sm,
                                                                    "Restart now",
                                                                    None,
                                                                )
                                                                .clicked()
                                                                {
                                                                    if let Err(err) = restart_application(&exe_path) {
                                                                        self.update_error = Some(err);
                                                                    } else {
                                                                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                                                    }
                                                                }
                                                            }

                                                            ui.add_space(SP_8);
                                                            let (close_rect, close_resp) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::click());
                                                            let close_resp = close_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                                                            let close_color = if close_resp.hovered() { TEXT_PRIMARY } else { TEXT_MUTED };
                                                            let c = close_rect.center();
                                                            let s = Stroke::new(1.5_f32, close_color);
                                                            ui.painter().line_segment([Pos2::new(c.x - 3.5, c.y - 3.5), Pos2::new(c.x + 3.5, c.y + 3.5)], s);
                                                            ui.painter().line_segment([Pos2::new(c.x + 3.5, c.y - 3.5), Pos2::new(c.x - 3.5, c.y + 3.5)], s);
                                                            if close_resp.clicked() {
                                                                self.status_banner = None;
                                                                self.status_banner_time = None;
                                                            }
                                                        });
                                                    });
                                            });
                                    }
                                }

                                if self.storage.projects.is_empty()
                                    && !matches!(self.selection, SidebarSelection::SecretManagement)
                                {
                                    self.render_first_time_empty_state(ui);
                                } else {
                                    match self.selection.clone() {
                                        SidebarSelection::AllDeployments => {
                                            self.render_all_deployments_overview(ui);
                                        }
                                        SidebarSelection::SecretManagement => {
                                            self.render_secret_management_view(ui);
                                        }
                                        SidebarSelection::Project(proj_id) => {
                                            self.render_project_workspace(ui, &proj_id);
                                        }
                                    }
                                }
                            });
                        });
                        ui.add_space(SP_32);
                    });
            });

        // Debounced state saving: writes to disk 400ms after user finishes typing
        if self.dirty_save_pending {
            if let Some(last) = self.last_edit_time {
                if last.elapsed() >= Duration::from_millis(400) {
                    let _ = self.storage.save();
                    self.dirty_save_pending = false;
                    self.last_edit_time = None;
                } else {
                    ctx.request_repaint_after(Duration::from_millis(100));
                }
            } else {
                let _ = self.storage.save();
                self.dirty_save_pending = false;
            }
        }
    }
}
