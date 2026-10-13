use super::*;
impl FlowriteApp {
    pub(super) fn library_sidebar(&mut self, ctx: &egui::Context) {
        if !self.show_sidebar {
            return;
        }
        let mut open = None;
        let mut folder = None;
        let mut home = false;
        let mut action = None;
        let width = design::panel_widths(
            ctx.screen_rect().width(),
            self.settings.appearance.sidebar_width,
            false,
        )
        .0;
        egui::SidePanel::left("library")
            .exact_width(width)
            .resizable(false)
            .frame(
                Frame::none()
                    .fill(theme::current().panel)
                    .inner_margin(12.0),
            )
            .show(ctx, |ui| {
                let content_height = (ui.available_height() - 52.0).max(80.0);
                egui::ScrollArea::vertical()
                    .id_salt("sidebar_surface")
                    .max_height(content_height)
                    .show(ui, |ui| {
                        ui.add_space(4.0);
                        if design::nav_row(
                            ui,
                            Icon::Home,
                            "Home",
                            self.show_home && self.current_folder.is_none(),
                        )
                        .clicked()
                        {
                            home = true;
                        }
                        if design::nav_row(ui, Icon::Search, "Search · Ctrl+K", false).clicked() {
                            self.palette.open = true;
                            self.palette.focus = true;
                        }
                        ui.add_space(8.0);
                        if design::nav_row(ui, Icon::Plus, "New page", false).clicked() {
                            open = Some(String::new());
                        }
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Documents")
                                    .size(12.0)
                                    .color(theme::current().muted),
                            );
                            if icons::control(
                                ui,
                                Icon::Folder,
                                "New folder",
                                "Create a folder · Ctrl+Shift+N",
                                true,
                                false,
                            )
                            .clicked()
                            {
                                self.begin_folder();
                            }
                        });
                        let response = ui.add(
                            TextEdit::singleline(&mut self.sidebar_query)
                                .hint_text("Filter pages…")
                                .desired_width(f32::INFINITY),
                        );
                        design::named(
                            response,
                            egui::WidgetType::TextEdit,
                            "Filter workspace pages",
                        );
                        let query = self.sidebar_query.trim().to_lowercase();
                        let max_height = if self.show_home {
                            ui.available_height() - 48.0
                        } else {
                            (ui.available_height() * 0.6).max(90.0)
                        };
                        egui::ScrollArea::vertical()
                            .id_salt("workspace_navigation")
                            .max_height(max_height.max(64.0))
                            .show(ui, |ui| {
                                for group in self
                                    .workspace
                                    .folders
                                    .iter()
                                    .map(Some)
                                    .chain(std::iter::once(None))
                                {
                                    let entries: Vec<_> = self
                                        .workspace
                                        .documents
                                        .iter()
                                        .filter(|entry| {
                                            (if self.settings.appearance.folders {
                                                entry.folder.as_deref()
                                                    == group.map(|f| f.id.as_str())
                                            } else {
                                                group.is_none()
                                            }) && (query.is_empty()
                                                || entry
                                                    .document
                                                    .title
                                                    .to_lowercase()
                                                    .contains(&query)
                                                || group.is_some_and(|f| {
                                                    f.name.to_lowercase().contains(&query)
                                                }))
                                        })
                                        .collect();
                                    let mut pages = |ui: &mut egui::Ui| {
                                        for entry in &entries {
                                            ui.push_id(&entry.id, |ui| {
                                                ui.horizontal(|ui| {
                                                    let width =
                                                        (ui.available_width() - 36.0).max(48.0);
                                                    let response = ui
                                                        .allocate_ui_with_layout(
                                                            Vec2::new(width, design::ROW_HEIGHT),
                                                            Layout::top_down(Align::LEFT),
                                                            |ui| {
                                                                design::nav_row(
                                                                    ui,
                                                                    Icon::Document,
                                                                    &entry.document.title,
                                                                    !self.show_home
                                                                        && self.active_document
                                                                            == entry.id,
                                                                )
                                                            },
                                                        )
                                                        .inner;
                                                    if response.clicked() {
                                                        open = Some(entry.id.clone());
                                                    }
                                                    response.context_menu(|ui| {
                                                        if let Some(next) = context_menu::page_menu(
                                                            ui,
                                                            &entry.id,
                                                            &self.workspace,
                                                        ) {
                                                            action = Some(next);
                                                        }
                                                    });
                                                    let menu =
                                                        icons::menu(ui, Icon::More, "", |ui| {
                                                            if let Some(next) =
                                                                context_menu::page_menu(
                                                                    ui,
                                                                    &entry.id,
                                                                    &self.workspace,
                                                                )
                                                            {
                                                                action = Some(next);
                                                            }
                                                        });
                                                    design::named(
                                                        menu.response.on_hover_text(
                                                            "Page options · move, rename, export",
                                                        ),
                                                        egui::WidgetType::Button,
                                                        &format!(
                                                            "Options for {}",
                                                            entry.document.title
                                                        ),
                                                    );
                                                });
                                            });
                                        }
                                    };
                                    if let Some(group) = group {
                                        if !self.settings.appearance.folders {
                                            continue;
                                        }
                                        let label = short_name(&group.name, 22);
                                        let header = egui::CollapsingHeader::new(label)
                                            .id_salt((&group.id, "navigation_folder"))
                                            .default_open(true)
                                            .show(ui, |ui| {
                                                if ui
                                                    .add(
                                                        egui::Button::new("Open folder")
                                                            .frame(false),
                                                    )
                                                    .hand()
                                                    .clicked()
                                                {
                                                    folder = Some(group.id.clone());
                                                }
                                                pages(ui);
                                            });
                                        header
                                            .header_response
                                            .on_hover_text(&group.name)
                                            .context_menu(|ui| {
                                                if let Some(next) =
                                                    context_menu::folder_menu(ui, &group.id)
                                                {
                                                    action = Some(next);
                                                }
                                            });
                                    } else {
                                        pages(ui);
                                    }
                                }
                            });
                        if !self.show_home {
                            ui.add_space(8.0);
                            ui.separator();
                            egui::CollapsingHeader::new("On this page")
                                .id_salt("page_outline")
                                .default_open(true)
                                .show(ui, |ui| {
                                    egui::ScrollArea::vertical()
                                        .id_salt("outline_navigation")
                                        .max_height((ui.available_height() - 80.0).max(64.0))
                                        .show(ui, |ui| {
                                            for block in &self.blocks {
                                                if !matches!(
                                                    block.kind,
                                                    BlockKind::Heading1 | BlockKind::Heading2
                                                ) {
                                                    continue;
                                                }
                                                ui.horizontal(|ui| {
                                                    if block.kind == BlockKind::Heading2 {
                                                        ui.add_space(12.0);
                                                    }
                                                    if design::nav_row(
                                                        ui,
                                                        Icon::Heading,
                                                        &block.text,
                                                        self.focused_block == Some(block.id),
                                                    )
                                                    .clicked()
                                                    {
                                                        self.scroll_target = Some(block.id);
                                                    }
                                                });
                                            }
                                        });
                                    let title = format!(
                                        "{} · {}",
                                        self.citations_heading(),
                                        sorted_citations(&self.citations).len()
                                    );
                                    if design::nav_row(
                                        ui,
                                        Icon::Citations,
                                        &title,
                                        self.show_citations,
                                    )
                                    .clicked()
                                    {
                                        self.show_citations = !self.show_citations;
                                        self.show_inspector = false;
                                    }
                                });
                        }
                    });
                ui.add_space(12.0);
                ui.separator();
                if design::nav_row(ui, Icon::Settings, "Settings", self.settings_open).clicked() {
                    self.settings_open = true;
                }
            });
        if home {
            self.go_home();
        }
        if let Some(id) = folder {
            self.go_home();
            self.current_folder = Some(id);
        }
        if let Some(id) = open {
            if id.is_empty() {
                self.start_fresh();
            } else {
                self.switch_document(&id);
            }
        }
        if let Some(action) = action {
            self.dialogs.action = Some(action);
        }
    }
}

pub(super) fn short_name(text: &str, limit: usize) -> String {
    let text = text.replace(['\n', '\r'], " ");
    if text.chars().count() <= limit {
        text
    } else {
        text.chars()
            .take(limit.saturating_sub(1))
            .collect::<String>()
            + "…"
    }
}
