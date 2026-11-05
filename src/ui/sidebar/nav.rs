//! Navigation items and search bar for the sidebar.

use super::*;

impl AppLoadApp {
    pub(crate) fn render_sidebar_nav(
        &mut self,
        ui: &mut egui::Ui,
        full_w: f32,
        sort_trigger_rect_out: &mut Option<Rect>,
    ) {
        ui.add_space(SP_12);

        // Nav Item 1: All Deployments (36px high)
        let is_overview = matches!(self.selection, SidebarSelection::AllDeployments);
        let (nav_rect, nav_resp) = ui.allocate_exact_size(Vec2::new(full_w, 36.0), Sense::click());
        let nav_resp = nav_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        let nav_bg = if is_overview || nav_resp.hovered() {
            SB_SURFACE_HOVER
        } else {
            Color32::TRANSPARENT
        };
        let nav_fg = if is_overview {
            Color32::WHITE
        } else if nav_resp.hovered() {
            SB_TEXT
        } else {
            SB_TEXT_MUTED
        };
        ui.painter()
            .rect_filled(nav_rect, CornerRadius::same(R_MD as u8), nav_bg);
        if is_overview {
            let bar_rect = Rect::from_min_max(
                Pos2::new(nav_rect.left(), nav_rect.top() + 6.0),
                Pos2::new(nav_rect.left() + 2.0, nav_rect.bottom() - 6.0),
            );
            ui.painter().rect_filled(
                bar_rect,
                CornerRadius {
                    nw: 0,
                    ne: 2,
                    se: 2,
                    sw: 0,
                },
                SB_ACCENT,
            );
        }
        draw_icon_layers(
            ui.painter(),
            Pos2::new(nav_rect.left() + 16.0, nav_rect.center().y),
            nav_fg,
        );
        ui.painter().text(
            Pos2::new(nav_rect.left() + 34.0, nav_rect.center().y),
            Align2::LEFT_CENTER,
            "All Deployments",
            Typo::Label.font(),
            nav_fg,
        );
        if nav_resp.clicked() {
            self.selection = SidebarSelection::AllDeployments;
        }

        // Nav Item 2: Bulk Deploy (36px high)
        if !self.storage.projects.is_empty() {
            ui.add_space(SP_4);
            let is_bulk_active = self.show_bulk_deploy_modal;
            let (bulk_rect, bulk_resp) =
                ui.allocate_exact_size(Vec2::new(full_w, 36.0), Sense::click());
            let bulk_resp = bulk_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            let bulk_bg = if is_bulk_active || bulk_resp.hovered() {
                SB_SURFACE_HOVER
            } else {
                Color32::TRANSPARENT
            };
            let bulk_fg = if is_bulk_active {
                Color32::WHITE
            } else if bulk_resp.hovered() {
                SB_TEXT
            } else {
                SB_TEXT_MUTED
            };
            ui.painter()
                .rect_filled(bulk_rect, CornerRadius::same(R_MD as u8), bulk_bg);
            if is_bulk_active {
                let bar_rect = Rect::from_min_max(
                    Pos2::new(bulk_rect.left(), bulk_rect.top() + 6.0),
                    Pos2::new(bulk_rect.left() + 2.0, bulk_rect.bottom() - 6.0),
                );
                ui.painter().rect_filled(
                    bar_rect,
                    CornerRadius {
                        nw: 0,
                        ne: 2,
                        se: 2,
                        sw: 0,
                    },
                    SB_ACCENT,
                );
            }
            draw_icon_send(
                ui.painter(),
                Pos2::new(bulk_rect.left() + 16.0, bulk_rect.center().y),
                bulk_fg,
            );
            ui.painter().text(
                Pos2::new(bulk_rect.left() + 34.0, bulk_rect.center().y),
                Align2::LEFT_CENTER,
                "Bulk Deploy...",
                Typo::Label.font(),
                bulk_fg,
            );
            if bulk_resp
                .on_hover_text("Select multiple projects to deploy sequentially")
                .clicked()
            {
                self.open_bulk_deploy_modal();
            }
        }

        // Nav Item 3: Secret Management (36px high)
        ui.add_space(SP_4);
        let is_secrets_active = matches!(self.selection, SidebarSelection::SecretManagement);
        let (sec_rect, sec_resp) = ui.allocate_exact_size(Vec2::new(full_w, 36.0), Sense::click());
        let sec_resp = sec_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        let sec_bg = if is_secrets_active || sec_resp.hovered() {
            SB_SURFACE_HOVER
        } else {
            Color32::TRANSPARENT
        };
        let sec_fg = if is_secrets_active {
            Color32::WHITE
        } else if sec_resp.hovered() {
            SB_TEXT
        } else {
            SB_TEXT_MUTED
        };
        ui.painter()
            .rect_filled(sec_rect, CornerRadius::same(R_MD as u8), sec_bg);
        if is_secrets_active {
            let bar_rect = Rect::from_min_max(
                Pos2::new(sec_rect.left(), sec_rect.top() + 6.0),
                Pos2::new(sec_rect.left() + 2.0, sec_rect.bottom() - 6.0),
            );
            ui.painter().rect_filled(
                bar_rect,
                CornerRadius {
                    nw: 0,
                    ne: 2,
                    se: 2,
                    sw: 0,
                },
                SB_ACCENT,
            );
        }
        draw_outline_icon(
            ui.painter(),
            Pos2::new(sec_rect.left() + 16.0, sec_rect.center().y),
            OutlineIcon::Key,
            sec_fg,
        );
        ui.painter().text(
            Pos2::new(sec_rect.left() + 34.0, sec_rect.center().y),
            Align2::LEFT_CENTER,
            "Secret Management",
            Typo::Label.font(),
            sec_fg,
        );
        if !self.storage.secrets.is_empty() {
            let count_str = self.storage.secrets.len().to_string();
            let cg = ui
                .painter()
                .layout_no_wrap(count_str, Typo::MonoSm.font(), SB_TEXT_MUTED);
            let cw = (cg.size().x + 10.0).max(20.0);
            let ch = 18.0;
            let crect = Rect::from_min_size(
                Pos2::new(sec_rect.right() - cw - 8.0, sec_rect.center().y - ch / 2.0),
                Vec2::new(cw, ch),
            );
            ui.painter()
                .rect_filled(crect, CornerRadius::same(R_SM as u8), SB_SURFACE);
            ui.painter().galley(
                Pos2::new(
                    crect.center().x - cg.size().x / 2.0,
                    crect.center().y - cg.size().y / 2.0,
                ),
                cg,
                SB_TEXT_MUTED,
            );
        }
        if sec_resp
                        .on_hover_text(
                            "Manage SSH credentials, passwords, and pipeline secrets in your OS Credential Manager",
                        )
                        .clicked()
                    {
                        self.selection = SidebarSelection::SecretManagement;
                    }

        ui.add_space(SP_24);

        // Projects Header Row (28px)
        let (hdr_rect, _) = ui.allocate_exact_size(Vec2::new(full_w, 28.0), Sense::hover());
        ui.painter().text(
            Pos2::new(hdr_rect.left(), hdr_rect.center().y),
            Align2::LEFT_CENTER,
            "PROJECTS",
            Typo::Caption.font(),
            SB_TEXT_FAINT,
        );

        let add_w = 46.0_f32;
        let sort_w = 78.0_f32;

        // "+ Add" Button as Ghost Sm
        let add_btn_rect = Rect::from_min_size(
            Pos2::new(hdr_rect.right() - add_w, hdr_rect.center().y - 12.0),
            Vec2::new(add_w, 24.0),
        );
        let add_btn_resp = ui
            .interact(
                add_btn_rect,
                egui::Id::new("sidebar_add_proj_btn"),
                Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text("Add new project");
        let add_bg = if add_btn_resp.hovered() {
            SB_SURFACE_HOVER
        } else {
            Color32::TRANSPARENT
        };
        let add_fg = if add_btn_resp.hovered() {
            Color32::WHITE
        } else {
            SB_TEXT_MUTED
        };
        ui.painter()
            .rect_filled(add_btn_rect, CornerRadius::same(R_MD as u8), add_bg);
        ui.painter().text(
            add_btn_rect.center(),
            Align2::CENTER_CENTER,
            "+ Add",
            Typo::Caption.font(),
            add_fg,
        );
        if add_btn_resp.clicked() {
            self.open_new_project_modal();
        }

        // Sort Control as Ghost Sm
        let sort_area_rect = Rect::from_min_size(
            Pos2::new(
                add_btn_rect.left() - SP_8 - sort_w,
                hdr_rect.center().y - 12.0,
            ),
            Vec2::new(sort_w, 24.0),
        );
        *sort_trigger_rect_out = Some(sort_area_rect);

        let sort_btn_resp = ui
            .interact(
                sort_area_rect,
                egui::Id::new("sidebar_project_sort_trigger"),
                Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if sort_btn_resp.clicked() {
            self.sidebar_sort_dropdown_open = !self.sidebar_sort_dropdown_open;
        }

        let is_sort_open = self.sidebar_sort_dropdown_open;
        let sort_bg = if is_sort_open || sort_btn_resp.hovered() {
            SB_SURFACE_HOVER
        } else {
            Color32::TRANSPARENT
        };
        let sort_fg = if is_sort_open || sort_btn_resp.hovered() {
            Color32::WHITE
        } else {
            SB_TEXT_MUTED
        };
        ui.painter()
            .rect_filled(sort_area_rect, CornerRadius::same(R_MD as u8), sort_bg);

        let sort_label = match self.project_sort_mode {
            ProjectSortMode::Custom => "Custom",
            ProjectSortMode::NameAsc => "A-Z",
            ProjectSortMode::NameDesc => "Z-A",
            ProjectSortMode::RecentDeploy => "Recent",
        };
        ui.painter().text(
            Pos2::new(sort_area_rect.left() + 8.0, sort_area_rect.center().y),
            Align2::LEFT_CENTER,
            sort_label,
            Typo::Caption.font(),
            sort_fg,
        );
        draw_outline_icon(
            ui.painter(),
            Pos2::new(sort_area_rect.right() - 10.0, sort_area_rect.center().y),
            if is_sort_open {
                OutlineIcon::ChevronUp
            } else {
                OutlineIcon::ChevronDown
            },
            sort_fg,
        );

        ui.add_space(SP_8);

        // Search input
        sidebar_search_text_edit(
            ui,
            "sidebar_project_search_box",
            &mut self.project_search_query,
            full_w,
            Some("Search projects..."),
        );

        ui.add_space(SP_16);
    }
}
