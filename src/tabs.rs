use super::*;
impl FlowriteApp {
    pub(super) fn close_temporary_draft(&mut self) {
        if self.dirty {
            if self.save_workspace() {
                let id = self.active_document.clone();
                self.close_tab(&id);
            }
        } else if let Some(id) = self.workspace.tabs.last().cloned() {
            self.load_workspace_document(&id);
        } else {
            self.show_home = true;
        }
    }

    pub(super) fn document_tabs(&mut self, ctx: &egui::Context) {
        let temporary = self.active_document.is_empty() && !self.show_home;
        let mut tabs: Vec<_> = self
            .workspace
            .tabs
            .iter()
            .filter_map(|id| {
                self.workspace
                    .documents
                    .iter()
                    .find(|d| &d.id == id)
                    .map(|d| {
                        (
                            id.clone(),
                            if id == &self.active_document {
                                self.title.clone()
                            } else {
                                d.document.title.clone()
                            },
                        )
                    })
            })
            .collect();
        if temporary {
            tabs.push((String::new(), self.title.clone()));
        }
        let mut open = None;
        let mut close = None;
        let mut new = false;
        egui::TopBottomPanel::top("document_tabs")
            .exact_height(38.0)
            .frame(
                Frame::none()
                    .fill(theme::current().canvas)
                    .inner_margin(Margin::symmetric(12.0, 3.0)),
            )
            .show(ctx, |ui| {
                let slots = ((ui.available_width() - 80.0) / 156.0).floor().max(1.0) as usize;
                let active = tabs
                    .iter()
                    .position(|(id, _)| id == &self.active_document)
                    .unwrap_or(0);
                let start = active.saturating_sub(slots - 1);
                let visible = tabs.len().min(slots);
                let width =
                    ((ui.available_width() - 88.0 - visible.saturating_sub(1) as f32 * 8.0)
                        / visible.max(1) as f32)
                        .clamp(90.0, 190.0);
                ui.horizontal(|ui| {
                    for (id, title) in tabs.iter().skip(start).take(slots) {
                        let selected = id == &self.active_document;
                        ui.push_id(("tab", id), |ui| {
                            Frame::none()
                                .fill(if selected {
                                    theme::current().hover
                                } else {
                                    theme::current().canvas
                                })
                                .rounding(5.0)
                                .inner_margin(Margin::symmetric(4.0, 0.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let response = ui
                                            .add_sized(
                                                [width - 52.0, 30.0],
                                                egui::Button::new(if id.is_empty() {
                                                    format!(
                                                        "{} · Draft",
                                                        sidebar::short_name(title, 24)
                                                    )
                                                } else {
                                                    title.clone()
                                                })
                                                .frame(false)
                                                .truncate(),
                                            )
                                            .hand()
                                            .on_hover_text(title);
                                        response.widget_info(|| {
                                            egui::WidgetInfo::selected(
                                                egui::WidgetType::Button,
                                                true,
                                                selected,
                                                if id.is_empty() {
                                                    format!("Temporary draft: {title}")
                                                } else {
                                                    format!("Document tab: {title}")
                                                },
                                            )
                                        });
                                        design::focus_ring(ui, &response);
                                        if response.clicked() {
                                            open = Some(id.clone());
                                        }
                                        if response.clicked_by(egui::PointerButton::Middle) {
                                            close = Some(id.clone());
                                        }
                                        if !id.is_empty() {
                                            response.context_menu(|ui| {
                                                if let Some(action) =
                                                    context_menu::page_menu(ui, id, &self.workspace)
                                                {
                                                    self.dialogs.action = Some(action);
                                                }
                                            });
                                        }
                                        if icons::tab_control(
                                            ui,
                                            Icon::Close,
                                            &format!("Close {title} tab"),
                                            "Close tab · Ctrl+W · page stays in your library",
                                        )
                                        .clicked()
                                        {
                                            close = Some(id.clone());
                                        }
                                    });
                                });
                        });
                    }
                    let menu = icons::tab_menu(ui, |ui| {
                        ui.label("Open tabs");
                        egui::ScrollArea::vertical()
                            .max_height(320.0)
                            .show(ui, |ui| {
                                for (id, title) in &tabs {
                                    if ui
                                        .selectable_label(id == &self.active_document, title)
                                        .hand()
                                        .clicked()
                                    {
                                        open = Some(id.clone());
                                        ui.close_menu();
                                    }
                                }
                            });
                    });
                    design::named(
                        menu.response
                            .on_hover_text("All open tabs · full document titles"),
                        egui::WidgetType::Button,
                        "Open tab overview",
                    );
                    if icons::tab_control(ui, Icon::Plus, "New page", "New page · Ctrl+N").clicked()
                    {
                        new = true;
                    }
                });
            });
        if let Some(id) = close {
            if id.is_empty() {
                self.close_temporary_draft();
            } else {
                self.close_tab(&id);
            }
        }
        if let Some(id) = open {
            if !id.is_empty() {
                self.switch_document(&id);
            }
        }
        if new {
            self.start_fresh();
        }
    }
}
