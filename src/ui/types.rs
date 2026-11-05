//! UI-specific enums, workspace tabs, font definitions, and modal action types.

use crate::version::GitMetadata;
use eframe::egui::Pos2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UiFont {
    #[default]
    Inter,
    SegoeUi,
}

pub const UI_FONT: UiFont = UiFont::Inter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SidebarSelection {
    AllDeployments,
    SecretManagement,
    Project(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkspaceTab {
    PipelineSteps,
    HistoryAndLogs,
    ServerSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CredentialOsTab {
    Windows,
    Linux,
    Mac,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SecretFilterStatus {
    #[default]
    All,
    Stored,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CachedVaultStatus {
    pub in_vault: bool,
    pub username: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConfirmDeleteAction {
    Project {
        project_id: String,
        project_name: String,
    },
    #[allow(dead_code)]
    Environment {
        project_id: String,
        project_name: String,
        env_key: String,
        env_name: String,
    },
    DeploymentRecord {
        deployment_id: String,
        project_name: String,
        env_name: String,
        version: String,
    },
    ClearDeploymentHistory {
        project_id: Option<String>,
        project_name: Option<String>,
        env_key: Option<String>,
        env_name: Option<String>,
        count: usize,
    },
    Secret {
        key: String,
    },
}

#[derive(Debug, Clone)]
pub struct SidebarContextMenuState {
    pub project_id: String,
    pub click_pos: Pos2,
    pub submenu_open: bool,
}

#[derive(Debug)]
pub struct GitInspectionResult {
    pub project_id: String,
    pub pkg_version: Option<String>,
    pub git_meta: GitMetadata,
    pub git_branches: Vec<String>,
    pub git_changes: Vec<String>,
    pub update_version_input: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncVersionCandidate {
    pub source_name: String,
    pub env_key: Option<String>,
    pub version: String,
    pub is_latest: bool,
}
