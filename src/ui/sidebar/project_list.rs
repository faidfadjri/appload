//! Project cards list, accordion grouping, and drag-and-drop reordering.

use super::*;

impl AppLoadApp {
    pub(crate) fn render_sidebar_project_list(&mut self, ui: &mut egui::Ui, full_w: f32) {
        // Skeleton loading on initial load or refresh:
        let is_refreshing = self
            .sidebar_refreshing_until
            .as_ref()
            .is_some_and(|t| Instant::now() < *t);
        if is_refreshing && self.storage.projects.is_empty() {
            ui.ctx().request_repaint();
            let time = ui.input(|i| i.time);
            for _ in 0..3 {
                let (skel_rect, _) =
                    ui.allocate_exact_size(Vec2::new(full_w, 58.0), Sense::hover());
                ui.painter()
                    .rect_filled(skel_rect, CornerRadius::same(R_MD as u8), SB_SURFACE);

                let name_w = (skel_rect.width() - 34.0) * 0.6;
                let name_rect = Rect::from_min_size(
                    Pos2::new(skel_rect.left() + 12.0, skel_rect.top() + 10.0),
                    Vec2::new(name_w, 14.0),
                );
                paint_shimmer_rect(ui.painter(), name_rect, 4.0, time);

                let pill1 = Rect::from_min_size(
                    Pos2::new(skel_rect.left() + 12.0, skel_rect.top() + 30.0),
                    Vec2::new(44.0, 18.0),
                );
                paint_shimmer_rect(ui.painter(), pill1, R_SM, time);
                let pill2 = Rect::from_min_size(
                    Pos2::new(skel_rect.left() + 12.0 + 48.0, skel_rect.top() + 30.0),
                    Vec2::new(52.0, 18.0),
                );
                paint_shimmer_rect(ui.painter(), pill2, R_SM, time);

                ui.add_space(SP_2);
            }
        } else if self.storage.projects.is_empty() {
            let (empty_rect, _) = ui.allocate_exact_size(Vec2::new(full_w, 58.0), Sense::hover());
            ui.painter()
                .rect_filled(empty_rect, CornerRadius::same(R_MD as u8), SB_SURFACE);
            ui.painter().text(
                empty_rect.center(),
                Align2::CENTER_CENTER,
                "No projects yet.\nClick '+ ADD' or import .ps1",
                Typo::Small.font(),
                SB_TEXT_MUTED,
            );
        } else {
            let is_pointer_released = ui.input(|i| i.pointer.any_released());
            let pointer_pos = ui.input(|i| i.pointer.hover_pos());

            if self.dragged_project_idx.is_some() {
                ui.ctx().request_repaint();
            }

            let query = self.project_search_query.trim().to_lowercase();
            let has_groups = self
                .storage
                .projects
                .iter()
                .any(|p| p.group.as_ref().is_some_and(|g| !g.trim().is_empty()));

            let mut group_sections: Vec<(Option<String>, Vec<usize>)> = Vec::new();
            if has_groups {
                for (idx, p) in self.storage.projects.iter().enumerate() {
                    if !query.is_empty() && !p.name.to_lowercase().contains(&query) {
                        continue;
                    }
                    let grp = p
                        .group
                        .as_ref()
                        .map(|g| g.trim().to_string())
                        .filter(|g| !g.is_empty());
                    if let Some(pos) = group_sections
                        .iter()
                        .position(|(g, _)| g.as_ref() == grp.as_ref())
                    {
                        group_sections[pos].1.push(idx);
                    } else {
                        group_sections.push((grp, vec![idx]));
                    }
                }
                group_sections.sort_by(|(g1, _), (g2, _)| match (g1, g2) {
                    (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                });
            } else {
                let indices: Vec<usize> = self
                    .storage
                    .projects
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| query.is_empty() || p.name.to_lowercase().contains(&query))
                    .map(|(idx, _)| idx)
                    .collect();
                group_sections.push((None, indices));
            }

            let mut clicked_project_id = None;
            let mut drag_reorder = None;

            for (grp_opt, proj_indices) in group_sections {
                if proj_indices.is_empty() {
                    continue;
                }
                if has_groups {
                    let group_title = grp_opt.as_deref().unwrap_or("General");
                    let group_key = grp_opt.clone().unwrap_or_else(|| "_general".to_string());
                    let is_collapsed = self.collapsed_project_groups.contains(&group_key);

                    let (grp_rect, grp_resp) =
                        ui.allocate_exact_size(Vec2::new(full_w, 28.0), Sense::click());
                    let grp_resp = grp_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                    if grp_resp.hovered() {
                        ui.painter().rect_filled(
                            grp_rect,
                            CornerRadius::same(R_SM as u8),
                            SB_SURFACE_HOVER,
                        );
                    }

                    let chev = if is_collapsed {
                        OutlineIcon::ChevronRight
                    } else {
                        OutlineIcon::ChevronDown
                    };
                    draw_outline_icon(
                        ui.painter(),
                        Pos2::new(grp_rect.left() + 4.0, grp_rect.center().y),
                        chev,
                        SB_TEXT_FAINT,
                    );
                    ui.painter().text(
                        Pos2::new(grp_rect.left() + 18.0, grp_rect.center().y),
                        Align2::LEFT_CENTER,
                        group_title.to_uppercase(),
                        Typo::Caption.font(),
                        SB_TEXT_FAINT,
                    );

                    paint_pill_right_aligned(
                        ui.painter(),
                        Pos2::new(grp_rect.right(), grp_rect.center().y),
                        &format!("{}", proj_indices.len()),
                        SB_SURFACE,
                        SB_TEXT_MUTED,
                    );

                    if grp_resp.clicked() {
                        if is_collapsed {
                            self.collapsed_project_groups.remove(&group_key);
                        } else {
                            self.collapsed_project_groups.insert(group_key.clone());
                        }
                    }

                    ui.add_space(SP_4);

                    if is_collapsed {
                        continue;
                    }
                }

                for proj_idx in proj_indices {
                    let project = &self.storage.projects[proj_idx];
                    let is_selected = matches!(&self.selection, SidebarSelection::Project(id) if id == &project.id);
                    let is_being_dragged = self.dragged_project_idx == Some(proj_idx);
                    let is_switching =
                        self.project_switch_loading_id.as_deref() == Some(&project.id);

                    // Project Card: borderless list item, 58px high, radius 6px
                    let (p_rect, p_resp) =
                        ui.allocate_exact_size(Vec2::new(full_w, 58.0), Sense::click());
                    let p_resp = p_resp.on_hover_cursor(egui::CursorIcon::PointingHand);

                    let fill = if is_selected {
                        SB_SURFACE
                    } else if p_resp.hovered() {
                        SB_SURFACE_HOVER
                    } else {
                        Color32::TRANSPARENT
                    };
                    if is_being_dragged {
                        let shadow_rect = p_rect.translate(Vec2::new(0.0, 3.0)).expand(1.0);
                        ui.painter().rect_filled(
                            shadow_rect,
                            CornerRadius::same(R_MD as u8),
                            SB_MENU_SHADOW_SOFT,
                        );
                    }
                    if fill != Color32::TRANSPARENT {
                        ui.painter()
                            .rect_filled(p_rect, CornerRadius::same(R_MD as u8), fill);
                    }

                    // Drag handle on left
                    let grip_rect = Rect::from_min_size(
                        Pos2::new(p_rect.left() + 2.0, p_rect.top() + 6.0),
                        Vec2::new(10.0, 46.0),
                    );
                    let grip_resp = ui
                        .interact(
                            grip_rect,
                            egui::Id::new(("sidebar_proj_grip", &project.id)),
                            Sense::drag(),
                        )
                        .on_hover_cursor(egui::CursorIcon::Grab);

                    if grip_resp.drag_started() || grip_resp.dragged() {
                        self.dragged_project_idx = Some(proj_idx);
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    }

                    let grip_color = if is_being_dragged || grip_resp.hovered() {
                        SB_ACCENT
                    } else if p_resp.hovered() {
                        SB_TEXT_FAINT
                    } else {
                        Color32::TRANSPARENT
                    };
                    draw_outline_icon(
                        ui.painter(),
                        grip_rect.center(),
                        OutlineIcon::GripVertical,
                        grip_color,
                    );

                    // Reordering drop preview & animation
                    if let Some(from_idx) = self.dragged_project_idx {
                        if from_idx != proj_idx {
                            if let Some(pos) = pointer_pos {
                                if p_rect.contains(pos) {
                                    let is_top_half = pos.y < p_rect.center().y;
                                    let line_y = if is_top_half {
                                        p_rect.top() - 1.0
                                    } else {
                                        p_rect.bottom() + 1.0
                                    };
                                    let insert_line = [
                                        Pos2::new(p_rect.left() + 6.0, line_y),
                                        Pos2::new(p_rect.right() - 6.0, line_y),
                                    ];
                                    // Drop insertion glow & line
                                    ui.painter().line_segment(
                                        insert_line,
                                        Stroke::new(4.0_f32, SB_ACCENT.gamma_multiply(0.4)),
                                    );
                                    ui.painter()
                                        .line_segment(insert_line, Stroke::new(2.5_f32, SB_ACCENT));
                                    ui.painter().circle_filled(
                                        Pos2::new(p_rect.left() + 6.0, line_y),
                                        3.5,
                                        SB_ACCENT,
                                    );
                                    ui.painter().rect_filled(
                                        p_rect,
                                        CornerRadius::same(R_MD as u8),
                                        SB_SELECTED_BG,
                                    );
                                    if is_pointer_released {
                                        drag_reorder = Some((from_idx, proj_idx));
                                    }
                                }
                            }
                        }
                    }

                    // Selected item: 2px SB_ACCENT left bar (no border)
                    if is_selected || is_switching {
                        let bar_rect = Rect::from_min_max(
                            Pos2::new(p_rect.left(), p_rect.top() + 6.0),
                            Pos2::new(p_rect.left() + 2.0, p_rect.bottom() - 6.0),
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

                    let content_x = p_rect.left() + 12.0;
                    let is_deploying = self.is_project_deploying(&project.id);

                    // Row 1: Project Name (13px, weight 500, --sb-text)
                    let name_color = SB_TEXT;
                    let avail_name_w = if is_deploying || is_switching {
                        (p_rect.right() - 28.0 - content_x).max(20.0)
                    } else {
                        (p_rect.right() - 12.0 - content_x).max(20.0)
                    };
                    let mut job = egui::text::LayoutJob::simple_singleline(
                        project.name.clone(),
                        Typo::Label.font(),
                        name_color,
                    );
                    job.wrap.max_width = avail_name_w;
                    job.wrap.max_rows = 1;
                    job.wrap.break_anywhere = false;
                    let name_galley = ui.painter().layout_job(job);
                    ui.painter().galley(
                        Pos2::new(content_x, p_rect.top() + 10.0),
                        name_galley,
                        name_color,
                    );

                    // Spinner slot on Row 1 (vertically aligned with project title)
                    if is_deploying || is_switching {
                        ui.ctx().request_repaint();
                        let spinner_center = Pos2::new(p_rect.right() - 16.0, p_rect.top() + 18.0);
                        let spinner_color = if is_deploying {
                            SB_ACCENT
                        } else {
                            SB_TEXT_FAINT
                        };
                        draw_circle_loading_spinner(
                            ui.painter(),
                            spinner_center,
                            5.0,
                            spinner_color,
                            ui.input(|i| i.time),
                        );
                    }

                    let p_resp = if is_deploying {
                        p_resp.on_hover_text(format!(
                            "{}\n(A deployment is currently running)",
                            project.name
                        ))
                    } else {
                        p_resp.on_hover_text(&project.name)
                    };

                    if p_resp.secondary_clicked() {
                        if let Some(pos) = ui.input(|i| i.pointer.latest_pos()) {
                            self.sidebar_context_menu = Some(SidebarContextMenuState {
                                project_id: project.id.clone(),
                                click_pos: pos,
                                submenu_open: false,
                            });
                        }
                    }

                    // Row 2: Env Badges below (SP_4 below Row 1: top = 30.0, height 18.0, pad 6, gap SP_4)
                    let row2_y = p_rect.top() + 30.0;
                    let max_pill_x = p_rect.right() - 12.0;
                    if is_switching {
                        ui.ctx().request_repaint();
                        let time = ui.input(|i| i.time);
                        let pill1 = Rect::from_min_size(
                            Pos2::new(content_x, row2_y),
                            Vec2::new(44.0, 18.0),
                        );
                        paint_shimmer_rect(ui.painter(), pill1, R_SM, time);
                        let pill2 = Rect::from_min_size(
                            Pos2::new(content_x + 48.0, row2_y),
                            Vec2::new(52.0, 18.0),
                        );
                        paint_shimmer_rect(ui.painter(), pill2, R_SM, time);
                    } else if project.environments.is_empty() {
                        paint_sidebar_env_badge(
                            ui.painter(),
                            Pos2::new(content_x, row2_y),
                            "No envs",
                            SB_SURFACE,
                            SB_TEXT_FAINT,
                            Some(max_pill_x - content_x),
                        );
                    } else {
                        let total_envs = project.environments.len();
                        let mut pill_x = content_x;
                        let mut drawn_count = 0;
                        for (i, env) in project.environments.iter().enumerate() {
                            let remaining_after_this = total_envs - (i + 1);
                            let badge = env.badge_label();
                            let is_env_running = self.is_env_deploying(&project.id, &env.key)
                                || self.storage.deployments.iter().any(|d| {
                                    d.project_id == project.id
                                        && d.environment == env.key
                                        && (d.status == DeploymentStatus::Running
                                            || self.is_deployment_active(&d.id))
                                });
                            let key_lower = env.key.to_lowercase();
                            let is_prod = key_lower.contains("prod");

                            let (bg, fg) = if is_prod {
                                (BADGE_PROD_BG, BADGE_PROD_TEXT)
                            } else {
                                (BADGE_DEV_BG, BADGE_DEV_TEXT)
                            };

                            let label = if is_env_running {
                                format!("{badge} Deploying...")
                            } else if let Some(ok_dep) = self
                                .storage
                                .last_succeeded_deployment(&project.id, &env.key)
                            {
                                format!("{badge} v{}", ok_dep.version)
                            } else {
                                format!("{badge} —")
                            };

                            let font_id = Typo::MonoSm.font();
                            let text_w = ui
                                .painter()
                                .layout_no_wrap(label.clone(), font_id, fg)
                                .size()
                                .x;
                            let badge_w = text_w + 12.0; // pad_x 6.0 on each side

                            let need_overflow = remaining_after_this > 0;
                            let reserved_for_overflow =
                                if need_overflow { SP_4 + 28.0 } else { 0.0 };

                            if pill_x + badge_w + reserved_for_overflow <= max_pill_x {
                                let w = paint_sidebar_env_badge(
                                    ui.painter(),
                                    Pos2::new(pill_x, row2_y),
                                    &label,
                                    bg,
                                    fg,
                                    Some(max_pill_x - pill_x - reserved_for_overflow),
                                );
                                pill_x += w + SP_4;
                                drawn_count += 1;
                            } else {
                                let remaining = total_envs - drawn_count;
                                if remaining > 0 && pill_x + 20.0 <= max_pill_x {
                                    paint_sidebar_env_badge(
                                        ui.painter(),
                                        Pos2::new(pill_x, row2_y),
                                        &format!("+{remaining}"),
                                        SB_SURFACE,
                                        SB_TEXT_MUTED,
                                        Some(max_pill_x - pill_x),
                                    );
                                }
                                break;
                            }
                        }
                    }

                    if p_resp.clicked() {
                        clicked_project_id = Some(project.id.clone());
                    }

                    ui.add_space(SP_2);
                }
            }

            if let Some((from, to)) = drag_reorder {
                self.storage.reorder_project(from, to);
                self.project_sort_mode = ProjectSortMode::Custom;
                self.dragged_project_idx = None;
            } else if is_pointer_released {
                self.dragged_project_idx = None;
            }

            if let Some(pid) = clicked_project_id {
                self.sidebar_context_menu = None;
                let was_different = match &self.selection {
                    SidebarSelection::Project(cur) => cur != &pid,
                    SidebarSelection::AllDeployments | SidebarSelection::SecretManagement => true,
                };
                self.selection = SidebarSelection::Project(pid.clone());
                if was_different {
                    self.project_switch_loading_id = Some(pid);
                    self.project_switch_loading_start = Some(Instant::now());
                }
                self.refresh_selected_project_metadata(true);
            }
        }
    }
}
