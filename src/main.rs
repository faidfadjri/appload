//! AppLoad application entry point and CLI mode dispatcher.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use appload::credentials::try_handle_askpass_invocation;
use appload::installer::{
    build_appload_icon_data, check_install_state, default_install_dir, install_or_update_self,
    is_setup_mode, InstallState, SetupWizardApp, APP_VERSION,
};
use appload::migration::{handle_install_migration_and_handoff, run_first_run_data_migration};
use appload::storage::AppStorage;
use appload::ui::AppLoadApp;
use eframe::egui;
use std::env;
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    // 0. Safety sandbox guard: if APPLOAD_TEST is set, APPLOAD_SANDBOX_ROOT MUST be set.
    appload::paths::check_startup_guard();

    // 1. If invoked by OpenSSH (ssh/scp) as SSH_ASKPASS helper, output the secret and exit immediately.
    if try_handle_askpass_invocation() {
        return Ok(());
    }

    // 2. If invoked with `--install` / `--silent` / `install`, install/update binary + shortcuts headlessly and exit.
    let args: Vec<String> = env::args().skip(1).collect();

    // 2b. Dedicated headless migration entry point (--migrate-only [--json])
    if args.iter().any(|a| a == "--migrate-only") {
        let is_json = args.iter().any(|a| a == "--json");
        let outcome = appload::migration::run_first_run_data_migration();
        if is_json {
            let res = serde_json::json!({
                "success": !outcome.is_error,
                "data_migrated": outcome.data_migrated,
                "notice": outcome.notice,
                "is_error": outcome.is_error,
                "project_count": outcome.project_count,
                "source_path": outcome.source_path,
            });
            println!("{res}");
        } else if outcome.is_error {
            eprintln!(
                "[AppLoad Migration] Failed: {}",
                outcome.notice.unwrap_or_default()
            );
        } else {
            println!(
                "[AppLoad Migration] Success (migrated={}): {}",
                outcome.data_migrated,
                outcome
                    .notice
                    .unwrap_or_else(|| "No legacy state to migrate.".to_string())
            );
        }
        std::process::exit(if outcome.is_error { 1 } else { 0 });
    }

    // 2c. Dedicated headless storage check entry point (--check-storage)
    if args.iter().any(|a| a == "--check-storage") {
        let storage_path = appload::storage::AppStorage::default_storage_path();
        let storage = appload::storage::AppStorage::load_or_default(&storage_path);
        let res = serde_json::json!({
            "storage_path": storage_path.display().to_string(),
            "projects_count": storage.projects.len(),
            "deployments_count": storage.deployments.len(),
            "secrets_count": storage.secrets.len(),
        });
        println!("{res}");
        std::process::exit(0);
    }
    if args
        .iter()
        .any(|a| a == "--install" || a == "install" || a == "--silent")
    {
        match install_or_update_self() {
            Ok(msg) => {
                println!("{msg}");
                return Ok(());
            }
            Err(err) => {
                eprintln!("Installation error: {err}");
                std::process::exit(1);
            }
        }
    }

    let app_icon = Arc::new(build_appload_icon_data());

    // 3. If launched as `setup.exe` (or `--setup`), open the graphical Desktop Setup Wizard!
    if is_setup_mode(&args) {
        let setup_options = eframe::NativeOptions {
            centered: true,
            viewport: egui::ViewportBuilder::default()
                .with_title(format!("AppLoad v{APP_VERSION} Setup"))
                .with_icon(app_icon)
                .with_inner_size([520.0, 390.0])
                .with_min_inner_size([500.0, 370.0])
                .with_resizable(false),
            ..Default::default()
        };
        return eframe::run_native(
            "AppLoad Setup",
            setup_options,
            Box::new(|cc| {
                AppLoadApp::configure_workspace_theme(&cc.egui_ctx, false);
                Ok(Box::new(SetupWizardApp::default()))
            }),
        );
    }

    // 4. Install migration and handoff:
    // If running as legacy shiply.exe in Programs\Shiply, install to Programs\AppLoad,
    // swap shortcuts, relaunch the new binary, and exit.
    if handle_install_migration_and_handoff() {
        return Ok(());
    }

    // 5. First-run data migration (copy with rename, leaving legacy intact, marker recorded)
    let migration_outcome = run_first_run_data_migration();

    // 6. Auto-update installed copy when launching a newer .exe version directly
    let mut startup_notice = None;
    if let InstallState::UpdateReady {
        installed_version,
        running_version,
        ..
    } = check_install_state(&default_install_dir(), APP_VERSION)
    {
        if let Ok(msg) = install_or_update_self() {
            startup_notice = Some(format!(
                "Auto-updated installed app from v{installed_version} to v{running_version} ({msg})"
            ));
        }
    }

    let storage_path = AppStorage::default_storage_path();
    let storage = AppStorage::load_or_default(storage_path);

    let mut width = 1280.0;
    let mut height = 800.0;
    if let Ok(w) = env::var("APPLOAD_WINDOW_WIDTH").or_else(|_| env::var("SHIPLY_WINDOW_WIDTH")) {
        if let Ok(val) = w.parse::<f32>() {
            width = val;
        }
    }
    if let Ok(h) = env::var("APPLOAD_WINDOW_HEIGHT").or_else(|_| env::var("SHIPLY_WINDOW_HEIGHT")) {
        if let Ok(val) = h.parse::<f32>() {
            height = val;
        }
    }

    let native_options = eframe::NativeOptions {
        centered: true,
        viewport: egui::ViewportBuilder::default()
            .with_title(format!("AppLoad v{APP_VERSION}"))
            .with_icon(app_icon)
            .with_decorations(false)
            .with_inner_size([width, height])
            .with_min_inner_size([520.0, 420.0]),
        ..Default::default()
    };

    eframe::run_native(
        "AppLoad",
        native_options,
        Box::new(move |cc| {
            AppLoadApp::configure_workspace_theme(&cc.egui_ctx, false);
            let mut app = AppLoadApp::new(storage);
            if let Some(notice) = migration_outcome.notice {
                app.set_status_banner(notice, !migration_outcome.is_error);
            } else if let Some(notice) = startup_notice {
                app.set_status_banner(notice, true);
            }
            Ok(Box::new(app))
        }),
    )
}
