//! Tests for secret management page and OS vault roundtrip integration.

use super::*;

#[test]
fn test_secret_management_page_and_os_vault_roundtrip() {
    let _sandbox = crate::paths::TestSandbox::new();
    let ctx = egui::Context::default();
    AppLoadApp::configure_light_workspace_style(&ctx);

    let db_path = _sandbox.path().join("appload_state.db");
    let storage = crate::storage::AppStorage::load_or_default(&db_path);
    let mut app = AppLoadApp::new(storage);

    let test_key = format!("appload-test-{}", uuid::Uuid::new_v4());
    let _vault_guard = crate::credentials::TestVaultCredentialGuard::new(test_key.clone());
    app.open_secret_management_for_target(
        &test_key,
        "deploy_user",
        "Test SSH Credential for Secret Management UI",
    );
    assert_eq!(app.selection, SidebarSelection::SecretManagement);
    assert!(app.show_secret_modal);

    app.secret_form_value = "TopS3cretVaultPass!2026".to_string();
    let save_msg = app
        .save_secret_to_os_vault()
        .expect("save secret to OS vault");
    assert!(save_msg.contains(&test_key));
    // Plaintext secret value must be wiped from UI memory immediately after saving
    assert!(app.secret_form_value.is_empty());
    // Cache must be populated and marked stored in vault
    assert!(app
        .secret_vault_cache
        .get(&test_key)
        .is_some_and(|c| c.in_vault));

    // Verify OS Credential Manager has the secret and username
    let vault_cred = read_os_vault_credential(&test_key).expect("secret in OS vault");
    assert_eq!(vault_cred.username.as_deref(), Some("deploy_user"));
    assert_eq!(vault_cred.password, "TopS3cretVaultPass!2026");

    // Render both sidebar, Secret Management view, and secret modal at desktop and compact widths
    for width in [1280.0_f32, 640.0_f32] {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(width, 800.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            app.render_sidebar(ctx, 1.0);
            app.render_secret_modal(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                app.render_secret_management_view(ui);
            });
        });
    }

    // Verify open_new_secret_modal clears form and opens modal
    app.open_new_secret_modal();
    assert!(app.show_secret_modal);
    assert!(app.secret_form_key.is_empty());
    assert!(!app.secret_form_editing_existing);

    // Delete secret from OS Vault and verify cleanup
    app.delete_secret_from_os_vault(&test_key)
        .expect("delete secret from OS vault");
    assert!(read_os_vault_credential(&test_key).is_none());
    assert!(!app.storage.secrets.iter().any(|s| s.key == test_key));
    assert!(!app.secret_vault_cache.contains_key(&test_key));
}
