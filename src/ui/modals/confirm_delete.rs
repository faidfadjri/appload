//! Destructive confirmation dialogs for projects and environments.

use super::*;

impl AppLoadApp {
    /// Renders a confirmation dialog before deleting a project or an environment.
    pub(crate) fn render_delete_confirmation_modal(&mut self, ctx: &egui::Context) {
        let mut is_open = self.pending_delete_confirm.is_some();
        let Some(anim_t) = prepare_animated_modal(ctx, "delete_confirm", &mut is_open) else {
            if !is_open {
                self.pending_delete_confirm = None;
            }
            return;
        };
        if !is_open {
            self.pending_delete_confirm = None;
            return;
        }

        let Some(action) = self.pending_delete_confirm.clone() else {
            return;
        };

        let mut execute_delete = false;
        let mut dismiss = false;

        let frame = modal_dialog_frame_for(ctx).stroke(Stroke::new(1.0_f32, DANGER_SOFT));
        let (window, inner_w) = animated_modal_window(ctx, "Confirm Deletion", anim_t, Some(frame));
        window.show(ctx, |ui| {
            ui.set_width(inner_w);

                let (modal_title, modal_subtitle, item_name, warning_text): (
                    &str,
                    &str,
                    String,
                    String,
                ) = match &action {
                    ConfirmDeleteAction::Project { project_name, .. } => (
                        "Delete Project",
                        "This will permanently delete the project from AppLoad.",
                        project_name.clone(),
                        "Are you sure you want to delete this project? All associated environments, pipeline configurations, and settings for this project will be removed.".to_string(),
                    ),
                    ConfirmDeleteAction::Environment { env_name, .. } => (
                        "Delete Environment",
                        "This will permanently remove this environment from the project.",
                        env_name.clone(),
                        "Are you sure you want to delete this environment? Its pipeline steps and SSH credentials mapping will be deleted.".to_string(),
                    ),
                    ConfirmDeleteAction::DeploymentRecord {
                        project_name,
                        env_name,
                        version,
                        ..
                    } => (
                        "Delete Deployment Record",
                        "This will permanently remove this deployment entry and its console logs.",
                        format!("{project_name} ({env_name} v{version})"),
                        "Are you sure you want to delete this deployment history record? Its execution logs and timestamp record will be permanently removed.".to_string(),
                    ),
                    ConfirmDeleteAction::ClearDeploymentHistory {
                        project_name,
                        env_name,
                        count,
                        ..
                    } => {
                        let scope_desc = match (project_name.as_deref(), env_name.as_deref()) {
                            (Some(p), Some(e)) => format!("{p} — {e} ({count} records)"),
                            (Some(p), None) => format!("{p} — All Environments ({count} records)"),
                            (None, Some(e)) => format!("All Projects — {e} ({count} records)"),
                            (None, None) => format!("All Deployments ({count} records)"),
                        };
                        (
                            "Clear Deployment History",
                            "This will permanently delete completed and failed deployment history records.",
                            scope_desc,
                            format!(
                                "Are you sure you want to clear {count} deployment history record(s)? Active running deployments will not be affected, but all completed/failed logs in this view will be permanently removed."
                            ),
                        )
                    }
                    ConfirmDeleteAction::Secret { key } => (
                        "Delete Secret from OS Vault",
                        "This will permanently remove this secret from your OS Credential Manager and AppLoad.",
                        key.clone(),
                        format!(
                            "Are you sure you want to delete secret '{key}'? Its stored password/token will be removed from your OS Credential Manager."
                        ),
                    ),
                };

                if render_modal_header(
                    ui,
                    "close_delete_confirm_modal",
                    modal_title,
                    modal_subtitle,
                ) {
                    dismiss = true;
                }

                ui.add_space(SP_16);

                egui::Frame::new()
                    .fill(FAIL_PILL_BG)
                    .stroke(Stroke::new(1.0_f32, DANGER_SOFT))
                    .corner_radius(R_LG)
                    .inner_margin(Margin::same(16))
                    .show(ui, |ui| {
                        horizontal_row(ui, 24.0, |ui| {
                            render_pill(
                                ui,
                                "Warning",
                                DANGER,
                                Color32::WHITE,
                            );
                            ui.label(
                                RichText::new(format!("Target: {item_name}"))
                                    .typo(Typo::Label).color(FAIL_PILL_TEXT),
                            );
                        });
                        ui.add_space(SP_8);
                        ui.label(
                            RichText::new(warning_text)
                                .typo(Typo::Small)
                                .color(DANGER),
                        );
                    });

                // Footer (top border 1px MD_BORDER, padding-top 16px, margin-top 24px)
                ui.add_space(SP_24);
                draw_divider(ui, MD_BORDER);
                ui.add_space(SP_16);

                horizontal_row(ui, 40.0, |ui| {
                    ui.label(
                        RichText::new("Warning: This action cannot be undone.")
                            .typo(Typo::Caption)
                            .color(DANGER),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let enter_submitted = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                        if kit::button(
                            ui,
                            ButtonVariant::DangerSolid,
                            ButtonSize::Md,
                            "Delete permanently",
                            Some(OutlineIcon::Trash),
                        )
                        .clicked()
                            || enter_submitted
                        {
                            execute_delete = true;
                        }

                        ui.add_space(SP_8);
                        if kit::button(
                            ui,
                            ButtonVariant::Secondary,
                            ButtonSize::Md,
                            "Cancel",
                            None,
                        )
                        .clicked()
                        {
                            dismiss = true;
                        }
                    });
                });
            });

        if execute_delete {
            match action {
                ConfirmDeleteAction::Project {
                    project_id,
                    project_name,
                } => {
                    if let Some(idx) = self
                        .storage
                        .projects
                        .iter()
                        .position(|p| p.id == project_id)
                    {
                        self.storage.projects.remove(idx);
                        let _ = self.storage.save();
                        self.selection = self
                            .storage
                            .projects
                            .first()
                            .map(|p| SidebarSelection::Project(p.id.clone()))
                            .unwrap_or(SidebarSelection::AllDeployments);
                        self.refresh_selected_project_metadata(true);
                        self.status_banner =
                            Some((format!("Removed project '{project_name}'."), true));
                    }
                }
                ConfirmDeleteAction::Environment {
                    project_id,
                    env_key,
                    env_name,
                    ..
                } => {
                    let mut remove_res: Option<Result<String, String>> = None;
                    if let Some(proj) = self
                        .storage
                        .projects
                        .iter_mut()
                        .find(|p| p.id == project_id)
                    {
                        if let Err(err) = proj.remove_environment(&env_key) {
                            remove_res = Some(Err(err));
                        } else {
                            let next_key = proj
                                .environments
                                .first()
                                .map(|e| e.key.clone())
                                .unwrap_or_default();
                            remove_res = Some(Ok(next_key));
                        }
                    }

                    match remove_res {
                        Some(Err(err)) => {
                            self.status_banner = Some((err, false));
                        }
                        Some(Ok(next_key)) => {
                            let _ = self.storage.save();
                            self.selected_env_key = next_key;
                            self.refresh_selected_project_metadata(true);
                            self.status_banner =
                                Some((format!("Removed environment '{env_name}'."), true));
                        }
                        None => {}
                    }
                }
                ConfirmDeleteAction::DeploymentRecord {
                    deployment_id,
                    project_name,
                    env_name,
                    version,
                } => {
                    if self.storage.remove_deployment(&deployment_id).is_some() {
                        if self.selected_deployment_id.as_deref() == Some(deployment_id.as_str()) {
                            self.selected_deployment_id =
                                self.storage.deployments.first().map(|d| d.id.clone());
                        }
                        self.refresh_selected_project_metadata(false);
                        self.status_banner = Some((
                            format!(
                                "Deleted deployment record v{version} ({project_name} - {env_name})."
                            ),
                            true,
                        ));
                    }
                }
                ConfirmDeleteAction::ClearDeploymentHistory {
                    project_id,
                    env_key,
                    ..
                } => {
                    let removed = self
                        .storage
                        .clear_deployment_history(project_id.as_deref(), env_key.as_deref());
                    if let Some(sel_id) = &self.selected_deployment_id {
                        if !self.storage.deployments.iter().any(|d| &d.id == sel_id) {
                            self.selected_deployment_id =
                                self.storage.deployments.first().map(|d| d.id.clone());
                        }
                    }
                    self.refresh_selected_project_metadata(false);
                    self.status_banner = Some((
                        format!("Cleared {removed} deployment history record(s)."),
                        true,
                    ));
                }
                ConfirmDeleteAction::Secret { key } => {
                    let _ = self.delete_secret_from_os_vault(&key);
                }
            }
            self.pending_delete_confirm = None;
        } else if dismiss {
            self.pending_delete_confirm = None;
        }
    }
}
