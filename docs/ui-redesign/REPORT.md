# Shiply App-Wide UI Redesign & Architecture Refactor: Final Report

**Date:** November 5, 2026  
**Branch:** `ui/redesign-v2` (in `D:\NTI\Labs\shiply`, based on `main` @ `0467fff`)  
**Status:** Complete — Ready for Review & Merge  
**Target:** Adrian & Orchestrator  

---

## 1. Executive Summary

Briefs 001, 002, and 003 are fully implemented across all 12 phases (Phases 0a through 11). The application has been transformed from its legacy dark-themed desktop layout into a modern Webflow/Linear-inspired light SaaS interface. All oversized UI files (`views.rs`, `sidebar.rs`, `modals.rs`, `tests.rs`) have been split into cohesive modules, strict zero-warning compiler lints (`[lints]` clippy) have been applied, and characterization tests guarantee complete safety of the frozen database schema and process command strings.

### Key Highlights
- **100% Visual Redesign:** Clean Webflow canvas (`#F7F8FA`), solid elevated white cards (`#FFFFFF`), 1px hairline borders (`#E5E7EB`), crisp typography hierarchy (`Typo::H1` through `Typo::Caption`), standardized interactive states, and consistent 36px inputs.
- **DPI Crispness (Phase 0a):** Embedded Windows `PerMonitorV2` application manifest eliminates bitmap stretching and blur at 125%+ display scaling.
- **Font Decision:** Bundled `Inter` as default primary proportional font and `JetBrains Mono` as code font. The `UI_FONT` compile switch is preserved with the `SegoeUi` fallback option for instant toggling if needed.
- **Zero Behaviour Change / Invariants Frozen:** The SQLite storage schema, serialization formats, process executor arguments (`ssh`, `scp`, `powershell.exe`), and OS credential vault keys remain 100% intact and verified by characterization tests.
- **Strict Code Quality:** `cargo check`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` pass with **zero warnings, zero errors**.
- **Test Suite:** 94 unit tests + 3 integration tests (97 total) passing cleanly.

---

## 2. Phase 0a: High-DPI Diagnosis & Manifest Integration

### Problem Statement
Adrian observed that the application "feels low resolution" when running on Windows displays at 125% DPI scaling. The original build did not embed an application manifest, causing Windows DWM to virtualize the process and bitmap-stretch the rendering buffer.

### Investigation & Solution
1. **Manifest Configuration:** Added `shiply.manifest` declaring:
   - `<dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>`
   - `<dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2, PerMonitor</dpiAwareness>`
   - Windows Common Controls v6.0 and OS compatibility GUIDs (Windows 10/11).
2. **Build System Integration:** Updated `build.rs` to compile the manifest into `shiply.rc` as `RT_MANIFEST` (type 24, resource ID 1). Provided a pure-Rust binary resource compiler fallback (`compile_raw_icon_res`) to guarantee embedding even if Windows SDK `rc.exe` is absent.
3. **Verification:**
   - Windows Task Manager `DPI Awareness` column displays: **`Per-Monitor (v2)`**.
   - Text rendering is sharp and unblurred across monitor scale switches.
   - Crop comparisons stored at:
     - `docs/ui-redesign/phase-0a-before-crop.png`
     - `docs/ui-redesign/phase-0a-after-crop.png`

---

## 3. Phase-by-Phase Summary & Commit Log

| Phase | Commit | Description | Key Deliverables |
|---|---|---|---|
| **0a** | `4128dd5` | Declare Per-Monitor DPI Awareness | Embedded `PerMonitorV2` manifest in `build.rs` and `.rc` |
| **0 & 1** | `e49ba6e`<br>`b505eae`<br>`1ea6aae` | Design Tokens, Typography & Checkpoint A | Inter & JetBrains Mono fonts, `Typo` scale, Webflow palette, Checkpoint A review |
| **2** | `848cd23` | Workspace Header, Env Cards & Underline Tabs | 3-column env grid, 40px underline tabs, project workspace header bar |
| **3** | `5863c20` | Pipeline Steps Tab Redesign | Step cards with 3px left color accents, drag reordering, syntax highlighting |
| **4** | `24dd369` | Deployments History & Logs | Deployments table, commit changes, 280px logs console card |
| **5** | `bbffb5f` | Environment Settings Tab | 880px left-aligned form cards, debounced auto-save, Danger zone card |
| **6** | `798bc7e` | All Deployments & Overview | Global history table, quick-start guide, OS credentials card |
| **7** | `6dc9e17` | Uniform Modals & Toast System | Centered responsive modals, 32x32 close button, top-right auto-dismiss toasts |
| **8** | `f065652` | Secret Management Modern SaaS View | 3 stat summary cards, status filter tabs, OS vault cache, guide body |
| **9** | `417a5d2` | Sidebar Navigation & Brand Polish | Dark navy sidebar (`#0B1220`), active states, project drag-sort, title bar |
| **10** | `f39b46f`<br>`15323e8`<br>`6b3737c`<br>`40c2804`<br>`dcba4e0`<br>`4beb0ae`<br>`a4fed82`<br>`75e518c`<br>`3639cac`<br>`270bbce`<br>`b043369` | Rust Best Practices Codebase Refactor | Module splits of all large UI files, [lints] clippy clean, 0 unwraps in prod, doc comments on all modules |
| **11.1** | `98a1d6d` | Pipeline Variable Chips Correctness | Fixed canonical variables (`{remote_dir}`, etc.), added substitution unit test |
| **11.2** | `693718c` | Table Header Left-Alignment | Aligned table headers with cell content in Deployments & Secrets tables |
| **11.3** | `aa5ac5d` | Deployments Table Columns & Responsiveness | Flexible Changes column (message + branch @ sha), duration, hide trigger <850px |
| **11.4** | `7490168` | Row Actions Menu & Rollback Relocation | Moved per-row Rollback into "⋯" menu (Logs button + "⋯" menu) |
| **11.5** | `af201b0` | Step Name Input Affordance | Standard 36px input with hover/focus borders and cursor |
| **11.6** | `ab31e38` | Settings Cards Left-Alignment | Left-aligned 880px settings cards with section header |
| **11.7** | `b5e91f3` | Secrets Page Polish | `Typo::H1` stat tiles, `+N` remainder pills with tooltips, search icon |
| **11.8** | `90a2918` | Pipeline Header Subtitle Wording | Updated to "Runs top to bottom when you deploy to {env_name}." |
| **11.9** | `c172124` | Release Bar Segmented Bump Group | Attached group with shared border and dividers, Sync tooltip |
| **11.10** | `544a6c5` | Collapsible Guide Chevrons | Chevron toggle buttons, height ≤56px when collapsed |
| **11.11** | `593bb00` | Header Path Spacing | 4px gap between folder icon and path text |
| **11.12** | `ef1a929` | Exact Table Header Column Allocation | Allocated exact column widths matching row geometry for Deployments & Secrets |
| **11.13** | `a6469a8` | Commit Message Truncation with Ellipsis | Truncate long commit messages with "..." and full tooltip, added unit test |
| **11.14** | `2857754` | Shorten Status in Narrow Sidebar Footer | Prevent clipping of "Reinstall · Donate" by shortening to "v{version}" when narrow |

---

## 4. Phase 10 Architecture & Refactoring Breakdown

### Module Splitting
All oversized UI view, modal, sidebar, and test files were modularized into focused submodules:

```
src/ui/
├── modals/
│   ├── mod.rs               (462 lines - controller, dispatch & helpers)
│   ├── bulk_deploy.rs       (246 lines - bulk deployment orchestrator dialog)
│   ├── confirm_delete.rs    (226 lines - delete confirmations & danger states)
│   ├── environment.rs       (202 lines - new/edit environment dialogs)
│   ├── project.rs           (248 lines - project creation & import modal)
│   ├── secret.rs            (273 lines - secret creation & edit modal)
│   └── update.rs            (205 lines - in-app update notification modal)
├── sidebar/
│   ├── mod.rs               (573 lines - sidebar container & layout coordinator)
│   ├── context_menu.rs      (752 lines - project & environment right-click menus)
│   ├── footer.rs            (303 lines - update check, open folder, donation)
│   ├── nav.rs               (446 lines - all deployments, secret mgmt, search)
│   └── project_list.rs      (638 lines - project trees, environments, drag-sort)
├── views/
│   ├── mod.rs               (164 lines - views module root & public interface)
│   ├── console.rs           (305 lines - deployment execution log terminal)
│   ├── empty_state.rs       (108 lines - zero projects empty state screen)
│   ├── guides.rs            (421 lines - onboarding & OS credentials guides)
│   ├── history.rs           (475 lines - deployment history table & filters)
│   ├── history_changes.rs   (185 lines - git commit history & diff views)
│   ├── history_desktop.rs   (337 lines - desktop table row layout & actions)
│   ├── history_mobile.rs    (235 lines - responsive stacked row layout)
│   ├── overview.rs          (331 lines - all deployments global overview)
│   ├── pipeline.rs          (689 lines - pipeline step list & editor cards)
│   ├── pipeline_header.rs   (242 lines - pipeline header, variable chips, add menu)
│   ├── project_workspace.rs (655 lines - project header & environment cards)
│   ├── release_bar.rs       (384 lines - release bar, version bump & deploy trigger)
│   ├── secrets.rs           (701 lines - secrets table, vault stats & guidelines)
│   └── settings.rs          (546 lines - environment config forms & danger zone)
└── tests/
    ├── mod.rs               (12 lines - test module aggregator)
    ├── concurrent_deploy_tests.rs (125 lines)
    ├── context_menu_tests.rs (180 lines)
    ├── history_tests.rs     (378 lines)
    ├── modal_tests.rs       (545 lines)
    ├── pipeline_tests.rs    (290 lines)
    ├── secret_tests.rs      (278 lines)
    ├── sidebar_tests.rs     (696 lines)
    ├── theme_tests.rs       (295 lines)
    └── view_tests.rs        (321 lines)
```

### Full Codebase Line Counts (Largest Files)
The table below reflects the actual line counts across `src/` following the refactor:

| File | Line Count | Status | Notes |
|---|---|---|---|
| `src/storage.rs` | 1,899 lines | Kept unified | Frozen SQLite schema, migrations, and transactions |
| `src/ui/app.rs` | 1,828 lines | Kept unified | Root eframe App lifecycle, window events & routing |
| `src/executor.rs` | 1,793 lines | Kept unified | Frozen SSH/SCP/PowerShell command construction |
| `src/ps1_importer.rs` | 949 lines | Unmodified | PowerShell parser & project auto-discovery |
| `src/updater.rs` | 880 lines | Unmodified | GitHub releases & self-update logic |
| `src/ui/icons.rs` | 840 lines | Submodule | Outline & solid vector icon drawing primitives |
| `src/ui/components.rs` | 807 lines | Submodule | Standardized widgets, 36px inputs & search boxes |
| `src/ui/sidebar/context_menu.rs` | 752 lines | Submodule | Context menu viewport flip & actions |
| `src/installer.rs` | 743 lines | Unmodified | OS installer, shortcuts & askpass binary builder |
| `src/domain.rs` | 713 lines | Unmodified | Pure domain models, config structures & interpolation |
| `src/ui/kit.rs` | 704 lines | Submodule | Core design system kit (cards, tags, buttons) |
| `src/ui/views/secrets.rs` | 701 lines | Submodule | Secret management SaaS view & credential guide |

### Refactoring Quality Standards
- **Clippy Invariants:** Zero `#[allow(clippy::too_many_arguments)]` remaining in production code; replaced with `IconButtonVisuals`.
- **Error Handling:** All production `.unwrap()` instances replaced with `Option::insert`, safe matching, or explicit `.expect(...)` documenting the exact invariant.
- **Documentation:** Every crate and submodule contains a top-level module documentation comment (`//!`).

---

## 5. Deviations / Not Done

In accordance with Brief 003 guidance to avoid high-risk semantic churn on frozen subsystems, the following items from the initial Phase 10 wish-list were deliberately not split or restructured:

1. **`src/storage.rs` (1,899 lines) was NOT split into submodules:**
   - *Reason:* Contains the SQLite table migrations, legacy JSON-to-SQLite migration, and ACID transaction boundaries. Splitting across multiple files introduced borrow and module visibility hazards without architectural benefit.
   - *Safety:* Pinned by characterization test `storage::tests::test_characterization_storage_json_schema_frozen`.
2. **`src/executor.rs` (1,793 lines) was NOT split into submodules:**
   - *Reason:* Contains the process execution engines for local PowerShell, SSH sessions, SCP upload, askpass integration, and secret log redaction. Command strings are frozen invariants that must not deviate.
   - *Safety:* Pinned by characterization tests `executor::tests::test_characterization_build_ssh_command_frozen` and `test_characterization_build_scp_command_frozen`.
3. **`src/ui/app.rs` (1,828 lines) was NOT split:**
   - *Reason:* Houses the top-level `eframe::App` implementation (`update`, `setup`), frame event loops, viewport commands (including screenshot automation), and cross-thread channel pollers.
4. **`ShiplyApp` State God-Struct was NOT broken into sub-structs:**
   - *Reason:* Extracting modal and form fields into separate structs (e.g., `SecretFormState`, `BulkDeployState`) would have touched over 1,200 call sites across the codebase, conflicting with the mandate to keep behavior strictly identical and diffs focused on UI quality. Form state cleanup is instead handled cleanly via explicit field resets in the respective modal modules.

---

## 6. Relocated Actions Map

All original capabilities and event handlers are preserved. The table below documents their new SaaS visual locations:

| Action | Old Location | New Location |
|---|---|---|
| **Edit Project** | Small text button in workspace | Workspace header button & "⋯" header context menu |
| **Duplicate Project** | Sidebar right-click menu only | Workspace header "⋯" context menu + Sidebar right-click |
| **Delete Project** | Red text button bottom of page | Workspace header "⋯" context menu (red Destructive item) |
| **Per-Row Rollback** | Orange button taking space on every row | Row actions "⋯" dropdown menu (`Roll back to this version`) |
| **Delete Deployment** | Row trash icon | Row actions "⋯" dropdown menu (`Delete record`) |
| **Add Pipeline Step** | 4 raw colored buttons inline | Single "+ Add step" Primary dropdown menu in section header |
| **Version Bumps** | 3 separate unattached buttons | Segmented attached group `[+Patch | +Minor | +Major]` |
| **Version Sync** | Icon button without tooltip | 36x36 icon button with tooltip `"Sync version across environments"` |
| **Refresh Vault Cache** | Auto-polled on render | Card toolbar refresh icon button (`Refresh OS vault status`) |
| **Secrets Filter** | Dropdown / mixed buttons | Segmented filter control (`All`, `Stored`, `Missing`) |
| **Guide Toggles** | Bare text button (`Show`/`Hide`) | Compact chevron icon buttons (`Show`/`Hide` tooltips) |
| **Copy Path** | Plain unformatted text | Folder icon with 4px gap + Monospace text, click-to-copy tooltip |

---

## 7. Acceptance Verification Results

All automated checks from Brief 002 Section 5 were run directly against the tree:

```powershell
# 1. Legacy font size constants / .strong() check:
grep -rn "FONT_\|FS_\|POPUP_FONT_SIZE\|\.strong()" src/ui
Result: 0 hits (PASS)

# 2. Raw Color32::from_rgb outside theme.rs / tests:
grep -rn "Color32::from_rgb" src/ui | grep -v "theme.rs\|tests.rs"
Result: 0 hits (PASS)

# 3. Typography scale verification:
Only 22, 16, 14, 13, 12, 11 (and 20 MonoLg) appear in typography.rs.
Result: Verified (PASS)

# 4. Strict clippy verification:
cargo clippy --all-targets -- -D warnings
Result: Clean — 0 warnings, 0 errors (PASS)

# 5. Full test suite:
cargo test
Result: 94 unit tests passed, 3 integration tests passed (97/97 PASS)

# 6. Formatting check:
cargo fmt --check
Result: Clean (PASS)
```

---

## 8. Screenshot Deliverables

Fresh screenshots have been generated at standard wide (1920×1080) and narrow responsive (1000×800) resolutions for every view:

| View | Wide (1920×1080) | Narrow (~1000×800) |
|---|---|---|
| **Phase 0a DPI Crops** | `docs/ui-redesign/phase-0a-before-crop.png` | `docs/ui-redesign/phase-0a-after-crop.png` |
| **Workspace (Phase 2)** | `docs/ui-redesign/phase-2-workspace-wide.png` | `docs/ui-redesign/phase-2-workspace-narrow.png` |
| **Pipeline Tab (Phase 11)** | `docs/ui-redesign/phase-11-pipeline-wide.png` | `docs/ui-redesign/phase-11-pipeline-narrow.png` |
| **Deployments Tab (Phase 11)** | `docs/ui-redesign/phase-11-deployments-wide.png` | `docs/ui-redesign/phase-11-deployments-narrow.png` |
| **Settings Tab (Phase 11)** | `docs/ui-redesign/phase-11-settings-wide.png` | `docs/ui-redesign/phase-11-settings-narrow.png` |
| **Secrets Page (Phase 11)** | `docs/ui-redesign/phase-11-secrets-wide.png` | `docs/ui-redesign/phase-11-secrets-narrow.png` |
| **All Deployments (Phase 6)** | `docs/ui-redesign/phase-6-all-deployments-wide.png` | `docs/ui-redesign/phase-6-all-deployments-narrow.png` |
| **Modals (Phase 7 & 8)** | `docs/ui-redesign/phase-7-modal-wide.png` | `docs/ui-redesign/phase-8-secrets-modal.png` |
| **Sidebar (Phase 9)** | `docs/ui-redesign/phase-9-sidebar-wide.png` | `docs/ui-redesign/phase-9-sidebar-narrow.png` |

---

## 9. Summary for Adrian & Orchestrator

The redesign and refactoring is complete on branch `ui/redesign-v2`. `main` remains untouched at `0467fff`, no git tags or releases were created, and no user sessions were interrupted. The branch is ready for your direct review and fast-forward/squash merge whenever convenient.
