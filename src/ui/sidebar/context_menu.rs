//! Sidebar project sort dropdown and right-click context menu.

use super::*;

impl AppLoadApp {
    pub(crate) fn render_sidebar_sort_dropdown(&mut self, ctx: &egui::Context, trigger_rect: Rect) {
        if !self.sidebar_sort_dropdown_open {
            return;
        }

        let menu_w = 192.0_f32;
        let menu_h = 4.0 + (32.0 * 4.0) + (3.0 * 2.0) + 4.0; // 142px
        let screen = ctx.screen_rect();

        // Right-align with the trigger so it never clips off the sidebar
        let mut menu_pos = Pos2::new(trigger_rect.right() - menu_w, trigger_rect.bottom() + 4.0);
        // Guard if below screen
        if menu_pos.y + menu_h > screen.bottom() - 8.0 {
            menu_pos.y = (trigger_rect.top() - menu_h - 4.0).max(screen.top() + 8.0);
        }
        let menu_rect = Rect::from_min_size(menu_pos, Vec2::new(menu_w, menu_h));

        // Close on outside click
        if ctx.input(|i| i.pointer.any_pressed()) {
            if let Some(ptr) = ctx.input(|i| i.pointer.latest_pos()) {
                if !menu_rect.contains(ptr) && !trigger_rect.contains(ptr) {
                    self.sidebar_sort_dropdown_open = false;
                    return;
                }
            }
        }

        // Close on Escape or Tab
        if ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Tab)) {
            self.sidebar_sort_dropdown_open = false;
            return;
        }

        // Keyboard navigation: ArrowUp, ArrowDown, Enter
        let modes = [
            ProjectSortMode::Custom,
            ProjectSortMode::NameAsc,
            ProjectSortMode::NameDesc,
            ProjectSortMode::RecentDeploy,
        ];
        let current_idx = modes
            .iter()
            .position(|m| *m == self.project_sort_mode)
            .unwrap_or(0);
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            let next_idx = (current_idx + 1) % modes.len();
            self.project_sort_mode = modes[next_idx];
            self.storage.sort_projects(modes[next_idx]);
        } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            let prev_idx = if current_idx == 0 {
                modes.len() - 1
            } else {
                current_idx - 1
            };
            self.project_sort_mode = modes[prev_idx];
            self.storage.sort_projects(modes[prev_idx]);
        } else if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.sidebar_sort_dropdown_open = false;
            return;
        }

        egui::Area::new(egui::Id::new("sidebar_sort_dropdown_area"))
            .order(egui::Order::Foreground)
            .fixed_pos(menu_pos)
            .show(ctx, |ui| {
                // Drop shadow 0 12px 32px rgba(0,0,0,0.45)
                let shadow_rect = menu_rect.expand(2.0).translate(Vec2::new(0.0, 4.0));
                ui.painter().rect_filled(
                    shadow_rect,
                    CornerRadius::same(R_MD as u8),
                    SB_MENU_SHADOW,
                );

                // Container: min-width 192px, bg --sb-surface, border 1px --sb-border, radius --r-md, padding 4px
                ui.painter()
                    .rect_filled(menu_rect, CornerRadius::same(R_MD as u8), SB_SURFACE);
                ui.painter().rect_stroke(
                    menu_rect,
                    CornerRadius::same(R_MD as u8),
                    Stroke::new(1.0_f32, SB_BORDER),
                    StrokeKind::Inside,
                );

                let options = [
                    (ProjectSortMode::Custom, "Custom / Drag"),
                    (ProjectSortMode::NameAsc, "Name (A-Z)"),
                    (ProjectSortMode::NameDesc, "Name (Z-A)"),
                    (ProjectSortMode::RecentDeploy, "Recently Deployed"),
                ];

                for (idx, (mode, label)) in options.iter().enumerate() {
                    let item_y = menu_pos.y + 4.0 + idx as f32 * 34.0;
                    let item_rect = Rect::from_min_size(
                        Pos2::new(menu_pos.x + 4.0, item_y),
                        Vec2::new(menu_w - 8.0, 32.0),
                    );
                    let item_resp = ui
                        .interact(
                            item_rect,
                            egui::Id::new(("sidebar_sort_opt", idx)),
                            Sense::click(),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand);

                    let is_selected = self.project_sort_mode == *mode;
                    let (bg, text_color, check_color) = if is_selected {
                        (PRIMARY_BLUE, Color32::WHITE, Color32::WHITE)
                    } else if item_resp.hovered() {
                        (SB_SURFACE_HOVER, SB_TEXT, SB_TEXT_MUTED)
                    } else {
                        (Color32::TRANSPARENT, SB_TEXT_MUTED, Color32::TRANSPARENT)
                    };

                    ui.painter()
                        .rect_filled(item_rect, CornerRadius::same(R_SM as u8), bg);

                    // Text: --fs-small
                    ui.painter().text(
                        Pos2::new(item_rect.left() + 10.0, item_rect.center().y),
                        Align2::LEFT_CENTER,
                        *label,
                        Typo::Small.font(),
                        text_color,
                    );

                    // Selected item: 14px check icon (SVG) on the right
                    if is_selected {
                        let check_pos = Pos2::new(item_rect.right() - 14.0, item_rect.center().y);
                        draw_outline_icon(ui.painter(), check_pos, OutlineIcon::Check, check_color);
                    }

                    if item_resp.clicked() {
                        self.project_sort_mode = *mode;
                        self.storage.sort_projects(*mode);
                        self.sidebar_sort_dropdown_open = false;
                    }
                }
            });
    }

    pub(crate) fn render_sidebar_custom_context_menu(&mut self, ctx: &egui::Context) {
        let state = match &self.sidebar_context_menu {
            Some(s) => s.clone(),
            None => return,
        };

        let project = match self
            .storage
            .projects
            .iter()
            .find(|p| p.id == state.project_id)
        {
            Some(p) => p.clone(),
            None => {
                self.sidebar_context_menu = None;
                return;
            }
        };

        let total_projects = self.storage.projects.len();
        let proj_idx = self
            .storage
            .projects
            .iter()
            .position(|p| p.id == state.project_id)
            .unwrap_or(0);

        let can_move_up = proj_idx > 0;
        let can_move_down = proj_idx + 1 < total_projects;
        let has_move_sep = can_move_up || can_move_down;

        let existing_groups = self.storage.existing_project_groups();
        let is_set_group_disabled = existing_groups.is_empty() && project.group.is_none();

        // Calculate menu height:
        let mut items_count = 4; // Edit Project, Set Group..., Duplicate Project, Refresh Metadata
        if can_move_up {
            items_count += 1;
        }
        if can_move_down {
            items_count += 1;
        }
        items_count += 1; // Delete Project

        let seps_count = if has_move_sep { 2 } else { 1 };
        let menu_w = 220.0_f32;
        let menu_h = 8.0 + (items_count as f32 * 32.0) + (seps_count as f32 * 9.0);

        let screen = ctx.screen_rect();
        let mut pos = state.click_pos;

        // Viewport collision flip:
        // If bottom overflow: flip upward!
        if pos.y + menu_h > screen.bottom() - 8.0 {
            pos.y = (pos.y - menu_h).max(screen.top() + 8.0);
        } else {
            pos.y = pos.y.max(screen.top() + 8.0);
        }

        // If right overflow: flip left!
        if pos.x + menu_w > screen.right() - 8.0 {
            pos.x = (pos.x - menu_w).max(screen.left() + 8.0);
        } else {
            pos.x = pos.x.max(screen.left() + 8.0);
        }

        let menu_rect = Rect::from_min_size(pos, Vec2::new(menu_w, menu_h));

        // Submenu geometry (if open):
        let submenu_w = 190.0_f32;
        let submenu_items_count =
            existing_groups.len() + if project.group.is_some() { 1 } else { 0 };
        let submenu_seps = if project.group.is_some() && !existing_groups.is_empty() {
            1
        } else {
            0
        };
        let submenu_h = 8.0 + (submenu_items_count as f32 * 32.0) + (submenu_seps as f32 * 9.0);

        let sub_x = if menu_rect.right() + 4.0 + submenu_w <= screen.right() - 8.0 {
            menu_rect.right() + 4.0
        } else {
            (menu_rect.left() - 4.0 - submenu_w).max(screen.left() + 8.0)
        };
        // Align submenu y to Set Group item
        let set_group_y_offset = 4.0
            + (if can_move_up { 32.0 } else { 0.0 })
            + (if can_move_down { 32.0 } else { 0.0 })
            + (if has_move_sep { 9.0 } else { 0.0 })
            + 32.0; // after Edit Project
        let mut sub_y = pos.y + set_group_y_offset;
        if sub_y + submenu_h > screen.bottom() - 8.0 {
            sub_y = (screen.bottom() - 8.0 - submenu_h).max(screen.top() + 8.0);
        }
        let submenu_rect =
            Rect::from_min_size(Pos2::new(sub_x, sub_y), Vec2::new(submenu_w, submenu_h));

        // Outside click handling:
        if ctx.input(|i| i.pointer.any_pressed()) {
            if let Some(ptr) = ctx.input(|i| i.pointer.latest_pos()) {
                let in_main = menu_rect.contains(ptr);
                let in_sub = state.submenu_open && submenu_rect.contains(ptr);
                if !in_main && !in_sub {
                    self.sidebar_context_menu = None;
                    return;
                }
            }
        }

        // Close on Escape
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.sidebar_context_menu = None;
            return;
        }

        // Close on scroll
        if ctx.input(|i| i.raw_scroll_delta.length_sq() > 0.0) {
            self.sidebar_context_menu = None;
            return;
        }

        let mut next_submenu_open = state.submenu_open;
        let mut action_move_up = false;
        let mut action_move_down = false;
        let mut action_edit = false;
        let mut action_duplicate = false;
        let mut action_refresh = false;
        let mut action_delete = false;
        let mut action_set_group = None;

        egui::Area::new(egui::Id::new("sidebar_custom_context_menu_area"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                // Drop shadow
                let shadow_rect = menu_rect.expand(2.0).translate(Vec2::new(0.0, 4.0));
                ui.painter().rect_filled(
                    shadow_rect,
                    CornerRadius::same(R_MD as u8),
                    SB_MENU_SHADOW,
                );

                // Container
                ui.painter()
                    .rect_filled(menu_rect, CornerRadius::same(R_MD as u8), SB_SURFACE);
                ui.painter().rect_stroke(
                    menu_rect,
                    CornerRadius::same(R_MD as u8),
                    Stroke::new(1.0_f32, SB_BORDER),
                    StrokeKind::Inside,
                );

                let mut cur_y = menu_rect.top() + 4.0;

                // Move Up
                if can_move_up {
                    let r = Rect::from_min_size(
                        Pos2::new(menu_rect.left() + 4.0, cur_y),
                        Vec2::new(menu_w - 8.0, 32.0),
                    );
                    let resp = ui
                        .interact(r, egui::Id::new("ctx_move_up"), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    let bg = if resp.hovered() {
                        SB_SURFACE_HOVER
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter()
                        .rect_filled(r, CornerRadius::same(R_SM as u8), bg);
                    draw_outline_icon(
                        ui.painter(),
                        Pos2::new(r.left() + 16.0, r.center().y),
                        OutlineIcon::ArrowUp,
                        SB_TEXT,
                    );
                    ui.painter().text(
                        Pos2::new(r.left() + 32.0, r.center().y),
                        Align2::LEFT_CENTER,
                        "Move Up",
                        Typo::Label.font(),
                        SB_TEXT,
                    );
                    if resp.clicked() {
                        action_move_up = true;
                    }
                    cur_y += 32.0;
                }

                // Move Down
                if can_move_down {
                    let r = Rect::from_min_size(
                        Pos2::new(menu_rect.left() + 4.0, cur_y),
                        Vec2::new(menu_w - 8.0, 32.0),
                    );
                    let resp = ui
                        .interact(r, egui::Id::new("ctx_move_down"), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    let bg = if resp.hovered() {
                        SB_SURFACE_HOVER
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter()
                        .rect_filled(r, CornerRadius::same(R_SM as u8), bg);
                    draw_outline_icon(
                        ui.painter(),
                        Pos2::new(r.left() + 16.0, r.center().y),
                        OutlineIcon::ArrowDown,
                        SB_TEXT,
                    );
                    ui.painter().text(
                        Pos2::new(r.left() + 32.0, r.center().y),
                        Align2::LEFT_CENTER,
                        "Move Down",
                        Typo::Label.font(),
                        SB_TEXT,
                    );
                    if resp.clicked() {
                        action_move_down = true;
                    }
                    cur_y += 32.0;
                }

                if has_move_sep {
                    cur_y += 4.0;
                    ui.painter().line_segment(
                        [
                            Pos2::new(menu_rect.left() + 8.0, cur_y),
                            Pos2::new(menu_rect.right() - 8.0, cur_y),
                        ],
                        Stroke::new(1.0_f32, SB_BORDER),
                    );
                    cur_y += 5.0;
                }

                // Edit Project
                {
                    let r = Rect::from_min_size(
                        Pos2::new(menu_rect.left() + 4.0, cur_y),
                        Vec2::new(menu_w - 8.0, 32.0),
                    );
                    let resp = ui
                        .interact(r, egui::Id::new("ctx_edit"), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    let bg = if resp.hovered() {
                        SB_SURFACE_HOVER
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter()
                        .rect_filled(r, CornerRadius::same(R_SM as u8), bg);
                    draw_outline_icon(
                        ui.painter(),
                        Pos2::new(r.left() + 16.0, r.center().y),
                        OutlineIcon::Edit,
                        SB_TEXT,
                    );
                    ui.painter().text(
                        Pos2::new(r.left() + 32.0, r.center().y),
                        Align2::LEFT_CENTER,
                        "Edit Project",
                        Typo::Label.font(),
                        SB_TEXT,
                    );
                    if resp.clicked() {
                        action_edit = true;
                    }
                    cur_y += 32.0;
                }

                // Set Group...
                {
                    let r = Rect::from_min_size(
                        Pos2::new(menu_rect.left() + 4.0, cur_y),
                        Vec2::new(menu_w - 8.0, 32.0),
                    );
                    let resp = if is_set_group_disabled {
                        ui.interact(r, egui::Id::new("ctx_set_group"), Sense::hover())
                            .on_hover_cursor(egui::CursorIcon::NotAllowed)
                            .on_hover_text("No existing groups. Assign groups in Edit Project.")
                    } else {
                        ui.interact(r, egui::Id::new("ctx_set_group"), Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                    };

                    let bg = if !is_set_group_disabled && (resp.hovered() || state.submenu_open) {
                        SB_SURFACE_HOVER
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter()
                        .rect_filled(r, CornerRadius::same(R_SM as u8), bg);

                    let fg = if is_set_group_disabled {
                        SB_DIVIDER_MUTED
                    } else {
                        SB_TEXT
                    };

                    draw_outline_icon(
                        ui.painter(),
                        Pos2::new(r.left() + 16.0, r.center().y),
                        OutlineIcon::Folder,
                        fg,
                    );
                    ui.painter().text(
                        Pos2::new(r.left() + 32.0, r.center().y),
                        Align2::LEFT_CENTER,
                        "Set Group...",
                        Typo::Label.font(),
                        fg,
                    );
                    draw_outline_icon(
                        ui.painter(),
                        Pos2::new(r.right() - 14.0, r.center().y),
                        OutlineIcon::ChevronRight,
                        fg,
                    );

                    if !is_set_group_disabled && (resp.hovered() || resp.clicked()) {
                        next_submenu_open = true;
                    }
                    cur_y += 32.0;
                }

                // Duplicate Project
                {
                    let r = Rect::from_min_size(
                        Pos2::new(menu_rect.left() + 4.0, cur_y),
                        Vec2::new(menu_w - 8.0, 32.0),
                    );
                    let resp = ui
                        .interact(r, egui::Id::new("ctx_dup"), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    let bg = if resp.hovered() {
                        SB_SURFACE_HOVER
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter()
                        .rect_filled(r, CornerRadius::same(R_SM as u8), bg);
                    draw_outline_icon(
                        ui.painter(),
                        Pos2::new(r.left() + 16.0, r.center().y),
                        OutlineIcon::Copy,
                        SB_TEXT,
                    );
                    ui.painter().text(
                        Pos2::new(r.left() + 32.0, r.center().y),
                        Align2::LEFT_CENTER,
                        "Duplicate Project",
                        Typo::Label.font(),
                        SB_TEXT,
                    );
                    if resp.clicked() {
                        action_duplicate = true;
                    }
                    cur_y += 32.0;
                }

                // Refresh Metadata
                {
                    let r = Rect::from_min_size(
                        Pos2::new(menu_rect.left() + 4.0, cur_y),
                        Vec2::new(menu_w - 8.0, 32.0),
                    );
                    let resp = ui
                        .interact(r, egui::Id::new("ctx_refresh"), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    let bg = if resp.hovered() {
                        SB_SURFACE_HOVER
                    } else {
                        Color32::TRANSPARENT
                    };
                    let fg = if resp.hovered() {
                        SB_TEXT
                    } else {
                        SB_TEXT_MUTED
                    };
                    ui.painter()
                        .rect_filled(r, CornerRadius::same(R_SM as u8), bg);
                    draw_outline_icon(
                        ui.painter(),
                        Pos2::new(r.left() + 16.0, r.center().y),
                        OutlineIcon::Refresh,
                        fg,
                    );
                    ui.painter().text(
                        Pos2::new(r.left() + 32.0, r.center().y),
                        Align2::LEFT_CENTER,
                        "Refresh Metadata",
                        Typo::Label.font(),
                        fg,
                    );
                    if resp.clicked() {
                        action_refresh = true;
                    }
                    cur_y += 32.0;
                }

                // Separator before Delete
                cur_y += 4.0;
                ui.painter().line_segment(
                    [
                        Pos2::new(menu_rect.left() + 8.0, cur_y),
                        Pos2::new(menu_rect.right() - 8.0, cur_y),
                    ],
                    Stroke::new(1.0_f32, SB_BORDER),
                );
                cur_y += 5.0;

                // Delete Project
                {
                    let r = Rect::from_min_size(
                        Pos2::new(menu_rect.left() + 4.0, cur_y),
                        Vec2::new(menu_w - 8.0, 32.0),
                    );
                    let resp = ui
                        .interact(r, egui::Id::new("ctx_delete"), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    let bg = if resp.is_pointer_button_down_on() {
                        DESTRUCTIVE_ACTIVE_BG
                    } else if resp.hovered() {
                        DESTRUCTIVE_HOVER_BG
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter()
                        .rect_filled(r, CornerRadius::same(R_SM as u8), bg);
                    draw_outline_icon(
                        ui.painter(),
                        Pos2::new(r.left() + 16.0, r.center().y),
                        OutlineIcon::Trash,
                        DESTRUCTIVE_TEXT,
                    );
                    ui.painter().text(
                        Pos2::new(r.left() + 32.0, r.center().y),
                        Align2::LEFT_CENTER,
                        "Delete Project",
                        Typo::Label.font(),
                        DESTRUCTIVE_TEXT,
                    );
                    if resp.clicked() {
                        action_delete = true;
                    }
                }
            });

        // Submenu for Set Group...
        if state.submenu_open && !is_set_group_disabled {
            egui::Area::new(egui::Id::new("sidebar_custom_submenu_area"))
                .order(egui::Order::Foreground)
                .fixed_pos(Pos2::new(sub_x, sub_y))
                .show(ctx, |ui| {
                    // Submenu drop shadow
                    let shadow_rect = submenu_rect.expand(2.0).translate(Vec2::new(0.0, 4.0));
                    ui.painter().rect_filled(
                        shadow_rect,
                        CornerRadius::same(R_MD as u8),
                        SB_MENU_SHADOW,
                    );

                    // Submenu container
                    ui.painter().rect_filled(
                        submenu_rect,
                        CornerRadius::same(R_MD as u8),
                        SB_SURFACE,
                    );
                    ui.painter().rect_stroke(
                        submenu_rect,
                        CornerRadius::same(R_MD as u8),
                        Stroke::new(1.0_f32, SB_BORDER),
                        StrokeKind::Inside,
                    );

                    let mut sub_cur_y = submenu_rect.top() + 4.0;
                    for (g_idx, grp) in existing_groups.iter().enumerate() {
                        let r = Rect::from_min_size(
                            Pos2::new(submenu_rect.left() + 4.0, sub_cur_y),
                            Vec2::new(submenu_w - 8.0, 32.0),
                        );
                        let resp = ui
                            .interact(r, egui::Id::new(("ctx_sub_grp", g_idx)), Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        let is_cur = project.group.as_deref() == Some(grp.as_str());
                        let bg = if resp.hovered() {
                            SB_SURFACE_HOVER
                        } else if is_cur {
                            ACCENT_SOFT
                        } else {
                            Color32::TRANSPARENT
                        };
                        let fg = if is_cur { ACCENT } else { SB_TEXT };

                        ui.painter()
                            .rect_filled(r, CornerRadius::same(R_SM as u8), bg);
                        if is_cur {
                            draw_outline_icon(
                                ui.painter(),
                                Pos2::new(r.left() + 16.0, r.center().y),
                                OutlineIcon::Check,
                                ACCENT,
                            );
                        }
                        ui.painter().text(
                            Pos2::new(r.left() + 32.0, r.center().y),
                            Align2::LEFT_CENTER,
                            grp,
                            Typo::Label.font(),
                            fg,
                        );
                        if resp.clicked() {
                            action_set_group = Some(Some(grp.clone()));
                        }
                        sub_cur_y += 32.0;
                    }

                    if project.group.is_some() {
                        if !existing_groups.is_empty() {
                            sub_cur_y += 4.0;
                            ui.painter().line_segment(
                                [
                                    Pos2::new(submenu_rect.left() + 8.0, sub_cur_y),
                                    Pos2::new(submenu_rect.right() - 8.0, sub_cur_y),
                                ],
                                Stroke::new(1.0_f32, SB_BORDER),
                            );
                            sub_cur_y += 5.0;
                        }

                        let r = Rect::from_min_size(
                            Pos2::new(submenu_rect.left() + 4.0, sub_cur_y),
                            Vec2::new(submenu_w - 8.0, 32.0),
                        );
                        let resp = ui
                            .interact(r, egui::Id::new("ctx_sub_remove_grp"), Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        let bg = if resp.hovered() {
                            SB_SURFACE_HOVER
                        } else {
                            Color32::TRANSPARENT
                        };
                        ui.painter()
                            .rect_filled(r, CornerRadius::same(R_SM as u8), bg);
                        draw_outline_icon(
                            ui.painter(),
                            Pos2::new(r.left() + 16.0, r.center().y),
                            OutlineIcon::X,
                            SB_TEXT_MUTED,
                        );
                        ui.painter().text(
                            Pos2::new(r.left() + 32.0, r.center().y),
                            Align2::LEFT_CENTER,
                            "Remove from Group",
                            Typo::Label.font(),
                            SB_TEXT_MUTED,
                        );
                        if resp.clicked() {
                            action_set_group = Some(None);
                        }
                    }
                });
        }

        if let Some(ref mut cur_state) = self.sidebar_context_menu {
            cur_state.submenu_open = next_submenu_open;
        }

        // Execute any clicked action
        if action_move_up {
            self.storage.move_project_up(proj_idx);
            self.project_sort_mode = ProjectSortMode::Custom;
            self.sidebar_context_menu = None;
        } else if action_move_down {
            self.storage.move_project_down(proj_idx);
            self.project_sort_mode = ProjectSortMode::Custom;
            self.sidebar_context_menu = None;
        } else if action_edit {
            self.open_edit_project_modal(proj_idx);
            self.sidebar_context_menu = None;
        } else if let Some(grp_opt) = action_set_group {
            self.storage.set_project_group(&project.id, grp_opt);
            self.sidebar_context_menu = None;
        } else if action_duplicate {
            if let Some(new_proj) = self.storage.duplicate_project(&project.id) {
                self.selection = SidebarSelection::Project(new_proj.id.clone());
                self.refresh_selected_project_metadata(true);
                self.status_banner = Some((
                    format!("Duplicated project successfully as '{}'", new_proj.name),
                    true,
                ));
            }
            self.sidebar_context_menu = None;
        } else if action_refresh {
            self.selection = SidebarSelection::Project(project.id.clone());
            self.refresh_selected_project_metadata(true);
            self.sidebar_context_menu = None;
        } else if action_delete {
            self.pending_delete_confirm = Some(ConfirmDeleteAction::Project {
                project_id: project.id.clone(),
                project_name: project.name.clone(),
            });
            self.sidebar_context_menu = None;
        }
    }
}
