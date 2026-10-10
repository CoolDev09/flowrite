use super::*;

impl FlowWriteApp {
    pub(super) fn initialize_workspace(&mut self) {
        match Workspace::load(workspace::default_directory()) {
            Ok(workspace) => self.workspace = workspace,
            Err(error) => {
                self.workspace_available = false;
                self.notice = Some(format!("Could not load your library: {error}. Save a copy from File to keep this draft."));
                return;
            }
        }
        if let Some(id) = self.workspace.active.clone() {
            self.load_workspace_document(&id);
        }
    }

    pub(super) fn commit_current(&mut self) {
        let document = self.local_document();
        if let Some(entry) = self
            .workspace
            .documents
            .iter_mut()
            .find(|d| d.id == self.active_document)
        {
            if entry.document != document {
                entry.modified = workspace::timestamp();
            }
            entry.document = document;
            entry.external_path = self.current_path.clone();
        }
    }

    pub(super) fn save_workspace(&mut self) -> bool {
        if !self.workspace_available || self.active_document.is_empty() {
            return false;
        }
        self.commit_current();
        let result = self.workspace.save().and_then(|()| {
            if let Some(path) = &self.current_path {
                let bytes =
                    serde_json::to_vec_pretty(&self.local_document()).map_err(|e| e.to_string())?;
                workspace::write_atomic(path, &bytes).map_err(|e| e.to_string())?;
            }
            Ok(())
        });
        match result {
            Ok(()) => {
                self.dirty = false;
                true
            }
            Err(error) => {
                self.notice = Some(format!(
                    "Autosave needs attention: {error}. Use File > Save as to save a copy."
                ));
                false
            }
        }
    }

    fn load_workspace_document(&mut self, id: &str) {
        let Some(entry) = self
            .workspace
            .documents
            .iter()
            .find(|d| d.id == id)
            .cloned()
        else {
            return;
        };
        self.active_document = id.to_owned();
        self.title = entry.document.title;
        self.blocks = entry.document.blocks;
        if self.blocks.is_empty() {
            self.blocks.push(Block::new(1, BlockKind::Text, ""));
        }
        self.next_id = self.blocks.iter().map(|b| b.id).max().unwrap_or(0) + 1;
        self.style = entry.document.style;
        self.details = entry.document.details;
        self.citations = entry.document.citations;
        self.current_path = entry.external_path;
        self.dirty = false;
        self.slash_target = None;
        self.block_picker_target = None;
        self.focused_block = None;
        self.focus_request = None;
        self.selection = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.history_navigation = true;
        self.workspace.open_tab(id);
    }

    pub(super) fn switch_document(&mut self, id: &str) {
        if id == self.active_document {
            self.show_home = false;
            self.show_sidebar = true;
            return;
        }
        if !self.save_workspace() {
            return;
        }
        self.load_workspace_document(id);
        self.show_home = false;
        self.show_sidebar = true;
        self.save_workspace();
    }

    fn close_tab(&mut self, id: &str) {
        if !self.save_workspace() {
            return;
        }
        self.workspace.tabs.retain(|tab| tab != id);
        if self.active_document == id {
            if let Some(next) = self.workspace.tabs.last().cloned() {
                self.load_workspace_document(&next);
            } else {
                self.start_fresh();
            }
        }
        self.save_workspace();
    }

    pub(super) fn document_tabs(&mut self, ctx: &egui::Context) {
        let mut open = None;
        let mut close = None;
        egui::TopBottomPanel::top("document_tabs")
            .exact_height(38.0)
            .frame(
                Frame::none()
                    .fill(CANVAS)
                    .inner_margin(Margin::symmetric(16.0, 5.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for id in &self.workspace.tabs {
                            if let Some(doc) = self.workspace.documents.iter().find(|d| &d.id == id)
                            {
                                let title = if id == &self.active_document {
                                    &self.title
                                } else {
                                    &doc.document.title
                                };
                                let label: String = title.chars().take(28).collect();
                                ui.push_id(id, |ui| {
                                    if ui
                                        .selectable_label(id == &self.active_document, label)
                                        .on_hover_text(title)
                                        .clicked()
                                    {
                                        open = Some(id.clone());
                                    }
                                    if ui
                                        .small_button("×")
                                        .on_hover_text("Close tab · document stays in your library")
                                        .clicked()
                                    {
                                        close = Some(id.clone());
                                    }
                                });
                                ui.separator();
                            }
                        }
                        if ui.small_button("+").on_hover_text("New document").clicked() {
                            open = Some(String::new());
                        }
                    });
                });
            });
        if let Some(id) = close {
            self.close_tab(&id);
        }
        if let Some(id) = open {
            if id.is_empty() {
                self.start_fresh();
            } else {
                self.switch_document(&id);
            }
        }
    }

    pub(super) fn library_sidebar(&mut self, ctx: &egui::Context) {
        if !self.show_sidebar {
            return;
        }
        let mut open = None;
        egui::SidePanel::left("library")
            .default_width(222.0)
            .width_range(180.0..=320.0)
            .frame(Frame::none().fill(PANEL).inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                ui.add_space(12.0);
                ui.label(RichText::new("Your workspace").size(14.0).strong());
                ui.add_space(16.0);
                ui.add(
                    TextEdit::singleline(&mut self.sidebar_query)
                        .hint_text("Search documents…")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(10.0);
                if ui
                    .add_sized(
                        [ui.available_width(), 30.0],
                        egui::Button::new("+  New document").frame(false),
                    )
                    .clicked()
                {
                    open = Some(String::new());
                }
                ui.add_space(18.0);
                ui.label(RichText::new("DOCUMENTS").size(10.0).color(MUTED));
                ui.add_space(8.0);
                let query = self.sidebar_query.to_lowercase();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for entry in &self.workspace.documents {
                        let title = if entry.id == self.active_document {
                            &self.title
                        } else {
                            &entry.document.title
                        };
                        if !title.to_lowercase().contains(&query) {
                            continue;
                        }
                        if ui
                            .add_sized(
                                [ui.available_width(), 30.0],
                                egui::Button::new(RichText::new(title).size(13.0))
                                    .selected(entry.id == self.active_document)
                                    .frame(false),
                            )
                            .clicked()
                        {
                            open = Some(entry.id.clone());
                        }
                    }
                    ui.add_space(28.0);
                    ui.label(RichText::new("ON THIS PAGE").size(10.0).color(MUTED));
                    for block in &self.blocks {
                        if matches!(block.kind, BlockKind::Heading1 | BlockKind::Heading2)
                            && ui.selectable_label(false, &block.text).clicked()
                        {
                            self.scroll_target = Some(block.id);
                        }
                    }
                    ui.add_space(20.0);
                    if ui
                        .selectable_label(
                            self.show_citations,
                            format!(
                                "Works cited  ·  {}",
                                sorted_citations(&self.citations).len()
                            ),
                        )
                        .clicked()
                    {
                        self.show_citations = !self.show_citations;
                        self.show_inspector = false;
                    }
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new("Link pages with [[Document title]]")
                            .size(11.0)
                            .color(MUTED),
                    );
                });
            });
        if let Some(id) = open {
            if id.is_empty() {
                self.start_fresh();
            } else {
                self.switch_document(&id);
            }
        }
    }

    pub(super) fn citations_panel(&mut self, ctx: &egui::Context) {
        if !self.show_citations {
            return;
        }
        let mut remove = None;
        egui::SidePanel::right("citations").default_width(292.0).width_range(250.0..=420.0)
            .frame(Frame::none().fill(PANEL).inner_margin(Margin::same(18.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(self.citations_heading());
                    if ui.small_button("×").clicked() { self.show_citations=false; }
                });
                ui.add_space(12.0);
                ui.label(RichText::new("Paste one finished citation per entry. Entries are alphabetized and given hanging indents. PDF starts them on a new page.").size(12.0).color(MUTED));
                ui.add_space(14.0);
                if ui.button("+ Add citation").clicked() { self.citations.push(String::new()); self.dirty=true; }
                ui.add_space(12.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (index,citation) in self.citations.iter_mut().enumerate() {
                        ui.push_id(index, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("Citation {}",index+1)).size(11.0).color(MUTED));
                                if ui.small_button("Remove").clicked() { remove=Some(index); }
                            });
                            if ui.add(TextEdit::multiline(citation).hint_text("Author. Title. Publisher, year.").desired_rows(5).desired_width(f32::INFINITY)).changed() { self.dirty=true; }
                            ui.add_space(18.0);
                        });
                    }
                });
            });
        if let Some(index) = remove {
            self.citations.remove(index);
            self.dirty = true;
            self.force_history = true;
        }
    }

    pub(super) fn citations_heading(&self) -> &'static str {
        if self.style.paper_format == PaperFormat::Apa {
            "References"
        } else {
            "Works Cited"
        }
    }

    pub(super) fn citations_preview(&self, ui: &mut egui::Ui, width: f32) {
        let entries = sorted_citations(&self.citations);
        if entries.is_empty() {
            return;
        }
        ui.add_space(40.0);
        ui.separator();
        ui.add_space(28.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(self.citations_heading()).font(editor_ui::document_font(
                    &self.style,
                    InlineStyle::default(),
                    16.0,
                )),
            );
        });
        ui.add_space(24.0);
        for entry in entries {
            let mut job = egui::text::LayoutJob::default();
            job.wrap.max_width = (width - 48.0).max(100.0);
            job.append(
                &entry,
                0.0,
                egui::TextFormat {
                    font_id: editor_ui::document_font(
                        &self.style,
                        InlineStyle::default(),
                        self.style.font_size * 4.0 / 3.0,
                    ),
                    color: INK,
                    line_height: Some(self.style.font_size * 4.0 / 3.0 * 2.0),
                    ..Default::default()
                },
            );
            // Indent every wrapped row, then offset the first line back to the margin.
            job.first_row_min_height = 0.0;
            job.sections[0].leading_space = -48.0;
            let galley = ui.fonts(|fonts| fonts.layout_job(job));
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(width, galley.size().y), Sense::hover());
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &entry));
            ui.painter()
                .galley(rect.min + Vec2::new(48.0, 0.0), galley, INK);
        }
    }

    pub(super) fn follow_document_link(&mut self, title: String) {
        self.commit_current();
        if let Some(id) = self.workspace.by_title(&title).map(|d| d.id.clone()) {
            self.switch_document(&id);
        } else {
            self.notice = Some(format!(
                "No document named “{title}”. Create it with New document, then use this link."
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn citation_preview_wraps_with_the_side_panel_open() {
        let ctx = egui::Context::default();
        configure_fonts(&ctx);
        let mut app = FlowWriteApp::empty();
        app.citations=vec!["Adams, Jordan. Reading the Everyday: Essays on Language, Culture, and the Stories We Tell. Sample University Press, 2023.".into()];
        let result = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.citations_preview(ui, 320.0));
        });
        let text = result
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text.starts_with("Adams,") => Some(text),
                _ => None,
            })
            .unwrap();
        assert!(text.galley.rows.len() > 1);
        assert!(
            text.galley.rect.max.x <= 272.1,
            "citation extends past its right margin: {:?}",
            text.galley.rect
        );
        let first = text.galley.rows[0].glyphs[0].pos.x;
        let next = text.galley.rows[1].glyphs[0].pos.x;
        assert!(
            (next - first - 48.0).abs() < 1.0,
            "hanging indent: {first} vs {next}"
        );
    }
}
