use super::*;
impl FlowriteApp {
    pub(super) fn home_page(&mut self, ctx: &egui::Context) {
        let compact = ctx.screen_rect().height() < 500.0;
        let mut open = None;
        let mut new = None;
        let mut folder = None;
        let before = self.settings.appearance.clone();
        egui::CentralPanel::default()
            .frame(
                Frame::none()
                    .fill(theme::current().canvas)
                    .inner_margin(if compact { 16.0 } else { 24.0 }),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt(("home_scroll", &self.current_folder))
                    .show(ui, |ui| {
                        let width = ui.available_width().min(1080.0);
                        ui.set_max_width(width);
                        ui.add_space(8.0);
                        let heading = self
                            .current_folder
                            .as_ref()
                            .and_then(|id| self.workspace.folders.iter().find(|f| &f.id == id))
                            .map(|f| f.name.as_str())
                            .unwrap_or("Your workspace");
                        ui.label(RichText::new(heading).size(26.0).strong());
                        if !compact {
                            ui.label(
                                RichText::new("Notes, papers, and ideas. Saved on this computer.")
                                    .color(theme::current().muted),
                            );
                        }
                        ui.add_space(if compact { 12.0 } else { 24.0 });
                        ui.horizontal_wrapped(|ui| {
                            let button = egui::Button::image_and_text(
                                Icon::Plus.image(ui, theme::current().canvas),
                                RichText::new("New page").color(theme::current().canvas),
                            )
                            .fill(theme::current().accent)
                            .min_size(Vec2::new(120.0, 34.0));
                            if ui.add(button).hand().clicked() {
                                new = Some(PaperFormat::Freeform);
                            }
                            if icons::labelled(ui, Icon::Folder, "New folder").clicked() {
                                self.begin_folder();
                            }
                            icons::menu(ui, Icon::Document, "Templates", |ui| {
                                ui.label("Paper layouts · check your instructor's requirements");
                                for (label, format) in [
                                    ("Blank note", PaperFormat::Freeform),
                                    ("MLA essay", PaperFormat::Mla),
                                    ("APA student paper", PaperFormat::Apa),
                                ] {
                                    if ui.button(label).hand().clicked() {
                                        new = Some(format);
                                        ui.close_menu();
                                    }
                                }
                            });
                        });
                        ui.add_space(if compact { 8.0 } else { 16.0 });
                        let response = ui.add(
                            TextEdit::singleline(&mut self.home_query)
                                .hint_text("Search titles and document text…")
                                .desired_width(width)
                                .margin(8.0),
                        );
                        design::named(
                            response,
                            egui::WidgetType::TextEdit,
                            "Search documents on Home",
                        );
                        if !compact
                            && self.current_folder.is_none()
                            && !self.workspace.folders.is_empty()
                        {
                            ui.add_space(if compact { 8.0 } else { 16.0 });
                            egui::CollapsingHeader::new(format!(
                                "Folders · {}",
                                self.workspace.folders.len()
                            ))
                            .id_salt("home_folders")
                            .default_open(false)
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    for entry in &self.workspace.folders {
                                        let count = self
                                            .workspace
                                            .documents
                                            .iter()
                                            .filter(|d| d.folder.as_ref() == Some(&entry.id))
                                            .count();
                                        let response = icons::labelled(
                                            ui,
                                            Icon::Folder,
                                            &format!(
                                                "{} · {count}",
                                                sidebar::short_name(&entry.name, 24)
                                            ),
                                        )
                                        .on_hover_text(&entry.name);
                                        if response.clicked() {
                                            folder = Some(entry.id.clone());
                                        }
                                        response.context_menu(|ui| {
                                            if let Some(action) =
                                                context_menu::folder_menu(ui, &entry.id)
                                            {
                                                self.dialogs.action = Some(action);
                                            }
                                        });
                                    }
                                });
                            });
                        }
                        ui.add_space(if compact { 12.0 } else { 24.0 });
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(if self.home_query.is_empty() {
                                    "Recent documents"
                                } else {
                                    "Search results"
                                })
                                .size(16.0)
                                .strong(),
                            );
                            let response = egui::ComboBox::from_id_salt("home_sort")
                                .selected_text(
                                    ["Last edited", "Title A–Z", "Oldest edited"]
                                        [self.settings.appearance.sort.min(2)],
                                )
                                .show_ui(ui, |ui| {
                                    for (sort, label) in
                                        ["Last edited", "Title A–Z", "Oldest edited"]
                                            .iter()
                                            .enumerate()
                                    {
                                        ui.selectable_value(
                                            &mut self.settings.appearance.sort,
                                            sort,
                                            *label,
                                        )
                                        .hand();
                                    }
                                })
                                .response
                                .hand();
                            design::named(response, egui::WidgetType::ComboBox, "Sort documents");
                            if icons::control(
                                ui,
                                Icon::List,
                                "List view",
                                "Document list",
                                true,
                                self.settings.appearance.list_view,
                            )
                            .clicked()
                            {
                                self.settings.appearance.list_view = true;
                            }
                            if icons::control(
                                ui,
                                Icon::Grid,
                                "Gallery view",
                                "Document gallery",
                                true,
                                !self.settings.appearance.list_view,
                            )
                            .clicked()
                            {
                                self.settings.appearance.list_view = false;
                            }
                        });
                        ui.add_space(12.0);
                        let query = self.home_query.trim().to_lowercase();
                        let mut documents: Vec<_> = self
                            .workspace
                            .documents
                            .iter()
                            .filter(|entry| {
                                (self.current_folder.is_none()
                                    || entry.folder == self.current_folder)
                                    && (query.is_empty()
                                        || entry.document.title.to_lowercase().contains(&query)
                                        || entry
                                            .document
                                            .blocks
                                            .iter()
                                            .any(|b| b.text.to_lowercase().contains(&query)))
                            })
                            .collect();
                        match self.settings.appearance.sort {
                            1 => documents.sort_by_key(|d| d.document.title.to_lowercase()),
                            2 => documents.sort_by_key(|d| d.modified),
                            _ => documents.sort_by_key(|d| std::cmp::Reverse(d.modified)),
                        }
                        if query.is_empty() {
                            documents.truncate(self.settings.appearance.recent_limit);
                        }
                        if documents.is_empty() {
                            ui.add_space(if compact { 12.0 } else { 24.0 });
                            ui.label(if query.is_empty() {
                                "Make a new page to start writing."
                            } else {
                                "No documents match. Try another title or phrase."
                            });
                        }
                        let columns = if self.settings.appearance.list_view {
                            1
                        } else {
                            ((width + 16.0) / 260.0).floor().max(1.0) as usize
                        };
                        let card_width = (width - (columns - 1) as f32 * 16.0) / columns as f32;
                        egui::Grid::new("home_documents")
                            .spacing(Vec2::new(16.0, 16.0))
                            .show(ui, |ui| {
                                for (index, entry) in documents.iter().enumerate() {
                                    ui.push_id(&entry.id, |ui| {
                                        let (response, menu) = document_card(
                                            ui,
                                            &entry.document,
                                            entry.modified,
                                            card_width,
                                            self.settings.appearance.list_view,
                                            self.settings.appearance.thumbnails,
                                        );
                                        if response.clicked() {
                                            open = Some(entry.id.clone());
                                        }
                                        response.context_menu(|ui| {
                                            if let Some(action) = context_menu::page_menu(
                                                ui,
                                                &entry.id,
                                                &self.workspace,
                                            ) {
                                                self.dialogs.action = Some(action);
                                            }
                                        });
                                        ui.allocate_new_ui(
                                            egui::UiBuilder::new().max_rect(menu),
                                            |ui| {
                                                let response =
                                                    icons::menu(ui, Icon::More, "", |ui| {
                                                        if let Some(action) =
                                                            context_menu::page_menu(
                                                                ui,
                                                                &entry.id,
                                                                &self.workspace,
                                                            )
                                                        {
                                                            self.dialogs.action = Some(action);
                                                        }
                                                    })
                                                    .response;
                                                design::named(
                                                    response,
                                                    egui::WidgetType::Button,
                                                    &format!(
                                                        "Options for {}",
                                                        entry.document.title
                                                    ),
                                                );
                                            },
                                        );
                                    });
                                    if (index + 1) % columns == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                        ui.add_space(32.0);
                    });
            });
        if let Some(id) = folder {
            self.current_folder = Some(id);
            self.home_query.clear();
        }
        if let Some(id) = open {
            self.switch_document(&id);
        }
        if let Some(format) = new {
            self.start_fresh();
            if !self.show_home && format != PaperFormat::Freeform {
                self.style.apply_preset(format);
                self.title = self.workspace.unique_title(if format == PaperFormat::Mla {
                    "Untitled MLA essay"
                } else {
                    "Untitled APA paper"
                });
                self.show_inspector = true;
                self.dirty = true;
                self.save_workspace();
            }
        }
        if before != self.settings.appearance {
            let _ = self.settings.save(&self.workspace.directory);
        }
    }
}

fn document_card(
    ui: &mut egui::Ui,
    document: &SavedDocument,
    modified: u64,
    width: f32,
    list: bool,
    previews: bool,
) -> (egui::Response, egui::Rect) {
    let height = if list { 96.0 } else { 176.0 };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!("Open {}", document.title),
        )
    });
    ui.painter().rect_filled(
        rect,
        design::SURFACE_RADIUS,
        if response.hovered() {
            theme::current().hover
        } else {
            theme::current().surface
        },
    );
    ui.painter().rect_stroke(
        rect,
        design::SURFACE_RADIUS,
        Stroke::new(1.0_f32, theme::current().stroke),
    );
    let mut title = egui::text::LayoutJob::simple_singleline(
        document.title.replace(['\r', '\n'], " "),
        FontId::new(15.0, FontFamily::Name("Liberation Sans Bold".into())),
        theme::current().ink,
    );
    title.wrap.max_width = (width - 54.0).max(40.0);
    title.wrap.max_rows = 1;
    ui.painter().galley(
        rect.min + Vec2::new(16.0, 12.0),
        ui.fonts(|f| f.layout_job(title)),
        theme::current().ink,
    );
    let preview = document
        .blocks
        .iter()
        .filter(|b| !b.text.trim().is_empty())
        .take(3)
        .map(|b| b.text.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join(" · ");
    let preview = if preview.is_empty() {
        "Empty page".into()
    } else {
        preview.chars().take(360).collect::<String>()
    };
    let mut job = egui::text::LayoutJob::simple(
        preview,
        FontId::proportional(14.0),
        theme::current().muted,
        (width - 32.0).max(60.0),
    );
    job.wrap.max_rows = if list { 1 } else { 4 };
    if previews {
        ui.painter().galley(
            rect.min + Vec2::new(16.0, 38.0),
            ui.fonts(|f| f.layout_job(job)),
            theme::current().muted,
        );
    }
    ui.painter().text(
        rect.left_bottom() + Vec2::new(16.0, -18.0),
        egui::Align2::LEFT_CENTER,
        home_ui::relative_time(modified),
        FontId::proportional(12.0),
        theme::current().muted,
    );
    design::focus_ring(ui, &response);
    (
        response.on_hover_text(&document.title).hand(),
        egui::Rect::from_min_size(
            Pos2::new(rect.right() - 38.0, rect.top() + 5.0),
            Vec2::new(30.0, 30.0),
        ),
    )
}
