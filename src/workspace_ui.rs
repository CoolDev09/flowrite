use super::*;

impl FlowriteApp {
    pub(super) fn initialize_workspace(&mut self) {
        match Workspace::load(workspace::default_directory()) {
            Ok(workspace) => {
                self.notice = workspace.recovery_message.clone();
                self.workspace = workspace;
            }
            Err(error) => {
                self.workspace_available = false;
                self.workspace.read_only = true;
                self.workspace.directory = workspace::default_directory();
                self.recovery.error = Some(error.clone());
                self.recovery.open = true;
                self.notice = Some(format!("Could not load your library: {error}. Save a copy from File to keep this draft."));
                return;
            }
        }
        if let Some(id) = self.workspace.active.clone() {
            self.load_workspace_document(&id);
        }
    }

    pub(super) fn commit_current(&mut self) {
        if self.workspace.documents.iter().any(|d| {
            d.id != self.active_document && d.document.title.eq_ignore_ascii_case(self.title.trim())
        }) {
            self.title = self.workspace.unique_title(self.title.trim());
        }
        if self.title.trim().is_empty() {
            self.title = self.workspace.unique_title("Untitled document");
        }
        let mut document = self.local_document();
        for (block, live) in document.blocks.iter_mut().zip(&mut self.blocks) {
            wikilinks::store(block, &self.workspace);
            live.link_ids = block.link_ids.clone();
        }
        if self.active_document.is_empty() && self.dirty {
            self.active_document = self
                .workspace
                .add(document.clone(), self.current_path.clone());
            self.workspace
                .move_document(&self.active_document, self.current_folder.clone());
        }
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
        if !self.workspace_available {
            return false;
        }
        self.commit_current();
        let result = self.workspace.save().and_then(|()| {
            if let Some(path) = &self.current_path {
                self.write_bundle(path)?;
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

    pub(super) fn load_workspace_document(&mut self, id: &str) {
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
        self.image_cache.clear();
        self.title = entry.document.title;
        self.blocks = entry.document.blocks;
        for block in &mut self.blocks {
            wikilinks::display(block, &self.workspace);
        }
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
        self.resolve_document_font();
        self.reset_history_baseline();
    }

    pub(super) fn switch_document(&mut self, id: &str) {
        if id == self.active_document {
            self.show_home = false;
            self.show_sidebar = true;
            return;
        }
        if self.dirty && !self.save_workspace() {
            return;
        }
        self.load_workspace_document(id);
        self.show_home = false;
        self.show_sidebar = true;
        self.save_workspace();
    }

    pub(super) fn close_tab(&mut self, id: &str) {
        if !self.save_workspace() {
            return;
        }
        self.workspace.tabs.retain(|tab| tab != id);
        self.closed_tabs.push(id.to_owned());
        if self.active_document == id {
            if let Some(next) = self.workspace.tabs.last().cloned() {
                self.load_workspace_document(&next);
            } else {
                self.active_document.clear();
                self.workspace.active = None;
                self.show_home = true;
                self.dirty = false;
            }
        }
        let _ = self.workspace.save();
    }

    pub(super) fn citations_panel(&mut self, ctx: &egui::Context) {
        if !self.show_citations {
            return;
        }
        let mut remove = None;
        design::detail_panel(ctx, "citations", self.detail_overlay, |ui| {
            ui.horizontal(|ui| {
                ui.heading(self.citations_heading());
                if icons::control(ui, Icon::Close, "Close", "Close citations", true, false)
                    .clicked()
                {
                    self.show_citations = false;
                }
            });
            ui.add_space(12.0);
            ui.label(RichText::new("Paste one finished citation per entry. Entries are alphabetized and given hanging indents. PDF starts them on a new page.").size(12.0).color(theme::current().muted));
            ui.add_space(14.0);
            if icons::labelled(ui, Icon::Plus, "Add citation").clicked() {
                self.citations.push(String::new());
                self.dirty = true;
            }
            ui.add_space(12.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (index, citation) in self.citations.iter_mut().enumerate() {
                    ui.push_id(index, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("Citation {}", index + 1))
                                    .size(11.0)
                                    .color(theme::current().muted),
                            );
                            if icons::danger(ui, "Delete").clicked() {
                                remove = Some(index);
                            }
                        });
                        if design::named(
                            ui.add(
                                TextEdit::multiline(citation)
                                    .hint_text("Author. Title. Publisher, year.")
                                    .desired_rows(5)
                                    .desired_width(f32::INFINITY),
                            ),
                            egui::WidgetType::TextEdit,
                            &format!("Citation {}", index + 1),
                        )
                        .changed()
                        {
                            self.dirty = true;
                        }
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

    pub(super) fn citations_heading(&self) -> &str {
        if !self.settings.citations.heading.trim().is_empty() {
            return self.settings.citations.heading.trim();
        }
        if self.style.paper_format == PaperFormat::Apa {
            "References"
        } else {
            "Works Cited"
        }
    }

    pub(super) fn citations_preview(&self, ui: &mut egui::Ui, width: f32) {
        let entries = self.citation_entries();
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
            let indent = self.settings.citations.hanging_inches * 96.0;
            job.wrap.max_width = (width - indent).max(100.0);
            job.append(
                &entry,
                0.0,
                egui::TextFormat {
                    font_id: editor_ui::document_font(
                        &self.style,
                        InlineStyle::default(),
                        self.style.font_size * 4.0 / 3.0,
                    ),
                    color: theme::current().ink,
                    line_height: Some(self.style.font_size * 4.0 / 3.0 * 2.0),
                    ..Default::default()
                },
            );
            // Indent every wrapped row, then offset the first line back to the margin.
            job.first_row_min_height = 0.0;
            job.sections[0].leading_space = -indent;
            let galley = ui.fonts(|fonts| fonts.layout_job(job));
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(width, galley.size().y), Sense::hover());
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &entry));
            ui.painter().galley(
                rect.min + Vec2::new(indent, 0.0),
                galley,
                theme::current().ink,
            );
        }
    }

    pub(super) fn follow_document_link(&mut self, title: String) {
        self.commit_current();
        if let Some(id) = self
            .workspace
            .documents
            .iter()
            .find(|d| d.id == title)
            .or_else(|| self.workspace.by_title(&title))
            .map(|d| d.id.clone())
        {
            self.switch_document(&id);
        } else {
            self.notice = Some(format!(
                "No document named “{title}”. Create it with New document, then use this link."
            ));
        }
    }
    pub(super) fn citation_entries(&self) -> Vec<String> {
        if self.settings.citations.alphabetize {
            sorted_citations(&self.citations)
        } else {
            self.citations
                .iter()
                .filter(|c| !c.trim().is_empty())
                .map(|c| c.split_whitespace().collect::<Vec<_>>().join(" "))
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn untouched_drafts_do_not_create_pages() {
        let mut app = FlowriteApp::empty();
        app.workspace.directory =
            std::env::temp_dir().join(format!("flowrite-draft-{}", workspace::timestamp()));
        app.start_fresh();
        assert!(app.active_document.is_empty());
        assert!(app.workspace.documents.is_empty());
        app.go_home();
        assert!(app.workspace.documents.is_empty());
        app.start_fresh();
        app.blocks[0].text = "First edit".into();
        app.dirty = true;
        app.save_workspace();
        assert_eq!(app.workspace.documents.len(), 1);
        let id = app.active_document.clone();
        app.close_tab(&id);
        assert!(app.show_home);
        assert_eq!(app.workspace.documents.len(), 1);
        assert!(app.workspace.tabs.is_empty());
    }
    #[test]
    fn citation_preview_wraps_with_the_side_panel_open() {
        let ctx = egui::Context::default();
        configure_fonts(&ctx);
        let mut app = FlowriteApp::empty();
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
