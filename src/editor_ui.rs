use super::*;
use egui::text::{CCursor, CCursorRange};

enum BlockAction {
    Split(usize, usize, usize),
    Delete(usize),
    Duplicate(usize),
    Up(usize),
    Down(usize),
}

impl FlowriteApp {
    pub(super) fn active_index(&self) -> usize {
        self.focused_block
            .and_then(|id| self.blocks.iter().position(|b| b.id == id))
            .unwrap_or(0)
    }

    pub(super) fn request_action(&mut self, action: PendingAction, ctx: &egui::Context) {
        if !self.active_document.is_empty() && self.save_workspace() {
            self.execute_action(action, ctx);
            return;
        }
        if self.dirty
            && (self.title != "Untitled document" || self.blocks.iter().any(|b| !b.text.is_empty()))
        {
            self.pending_action = Some(action);
        } else {
            self.execute_action(action, ctx);
        }
    }

    fn execute_action(&mut self, action: PendingAction, ctx: &egui::Context) {
        self.pending_action = None;
        match action {
            PendingAction::New => self.start_fresh(),
            PendingAction::Open => self.open_document(),
            PendingAction::Close => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn restore(&mut self, snapshot: UndoSnapshot) {
        let document = snapshot.document;
        self.title = document.title;
        self.blocks = document.blocks;
        self.style = document.style;
        self.details = document.details;
        self.citations = document.citations;
        self.next_id = self.blocks.iter().map(|b| b.id).max().unwrap_or(0) + 1;
        self.dirty = true;
        self.slash_target = None;
        self.selection = None;
        let caret = snapshot
            .caret
            .filter(|(id, _)| self.blocks.iter().any(|b| b.id == *id))
            .or_else(|| self.blocks.first().map(|b| (b.id, 0)));
        self.focused_block = caret.map(|(id, _)| id);
        self.focus_request = caret;
        self.reset_history_baseline();
    }

    pub(super) fn undo(&mut self) {
        if let Some(previous) = self.undo_stack.pop() {
            self.history_navigation = true;
            self.redo_stack.push(self.history_entry());
            self.restore(previous);
        }
    }

    pub(super) fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.history_navigation = true;
            self.undo_stack.push(self.history_entry());
            self.restore(next);
        }
    }

    pub(super) fn toggle_decoration(&mut self, decoration: Decoration, ctx: &egui::Context) {
        let index = self.active_index();
        let block = &mut self.blocks[index];
        let mut styles = block.character_styles();
        let (start, end) = self
            .selection
            .filter(|(id, _, _)| *id == block.id)
            .map(|(_, a, b)| (a, b))
            .unwrap_or((0, 0));
        let current = styles.get(start).copied().unwrap_or(block.format);
        let value = !decoration_value(current, decoration);
        if start < end {
            for style in styles.iter_mut().take(end).skip(start) {
                set_decoration(style, decoration, value);
            }
        } else {
            set_decoration(&mut block.format, decoration, value);
            for style in &mut styles {
                set_decoration(style, decoration, value);
            }
        }
        block.set_character_styles(&styles);
        self.dirty = true;
        self.force_history = true;
        ctx.memory_mut(|m| {
            m.request_focus(egui::Id::new(("block", &self.active_document, block.id)))
        });
    }

    fn commands(&self) -> Vec<BlockKind> {
        let query = self
            .slash_target
            .and_then(|i| self.blocks.get(i))
            .map(|b| b.text.trim().trim_start_matches('/').to_lowercase())
            .unwrap_or_default();
        BlockKind::ALL
            .into_iter()
            .filter(|kind| {
                query.is_empty()
                    || kind.label().to_lowercase().contains(&query)
                    || kind.hint().to_lowercase().contains(&query)
                    || (query == "h1" && *kind == BlockKind::Heading1)
                    || (query == "h2" && *kind == BlockKind::Heading2)
                    || (query == "bullet" && *kind == BlockKind::Bulleted)
                    || (matches!(query.as_str(), "note" | "tip" | "warning" | "important")
                        && *kind == BlockKind::Callout)
            })
            .collect()
    }

    fn handle_keys(&mut self, ctx: &egui::Context) {
        if self.dialogs.delete.is_some() || self.dialogs.rename.is_some() || self.folder_dialog {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::K)) {
            self.palette.open = !self.palette.open;
            self.palette.focus = true;
            self.palette.query.clear();
            self.palette.index = 0;
        }
        if self.palette.open {
            return;
        }
        if ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, egui::Key::F11)
                || i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::F,
                )
        }) {
            self.focus_mode = !self.focus_mode;
        }
        if ctx.input_mut(|i| {
            i.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::T,
            )
        }) {
            while let Some(id) = self.closed_tabs.pop() {
                if self.workspace.documents.iter().any(|d| d.id == id) {
                    self.switch_document(&id);
                    break;
                }
            }
        }
        let tab = ctx.input_mut(|i| {
            let backwards = i.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::Tab,
            );
            let forwards = i.consume_key(egui::Modifiers::COMMAND, egui::Key::Tab);
            if backwards {
                Some(true)
            } else if forwards {
                Some(false)
            } else {
                None
            }
        });
        if let Some(backwards) = tab {
            let count = self.workspace.tabs.len();
            if count > 0 {
                let current = self
                    .workspace
                    .tabs
                    .iter()
                    .position(|id| id == &self.active_document)
                    .unwrap_or(0);
                let next = if backwards {
                    (current + count - 1) % count
                } else {
                    (current + 1) % count
                };
                let id = self.workspace.tabs[next].clone();
                self.switch_document(&id);
            }
        }
        for (index, key) in [
            egui::Key::Num1,
            egui::Key::Num2,
            egui::Key::Num3,
            egui::Key::Num4,
            egui::Key::Num5,
            egui::Key::Num6,
            egui::Key::Num7,
            egui::Key::Num8,
            egui::Key::Num9,
        ]
        .iter()
        .enumerate()
        {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, *key)) {
                if let Some(id) = self.workspace.tabs.get(index).cloned() {
                    self.switch_document(&id);
                }
            }
        }
        let (settings, folder, close, replace) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma),
                i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::N,
                ),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::W),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::H),
            )
        });
        if settings {
            self.settings_open = !self.settings_open;
        }
        if folder {
            self.begin_folder();
        }
        if close && !self.show_home {
            let id = self.active_document.clone();
            if id.is_empty() {
                self.close_temporary_draft();
            } else {
                self.close_tab(&id);
            }
        }
        if replace && !self.show_home {
            self.find_open = true;
            self.search.replace = true;
            ctx.memory_mut(|m| m.request_focus(egui::Id::new("find_query")));
        }
        if self.settings_open {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F7)) {
            if self.proofreader.issues.is_empty() {
                self.proofreader.invalidate();
                self.proofreader.manual = true;
            } else {
                let mut issues: Vec<_> = self
                    .blocks
                    .iter()
                    .flat_map(|b| {
                        self.proofreader.issues.get(&b.id).into_iter().flat_map(
                            move |(_, issues)| issues.iter().map(move |issue| (b.id, issue.start)),
                        )
                    })
                    .collect();
                let current = self.selection.map(|(id, _, end)| (id, end));
                let next = current
                    .and_then(|c| issues.iter().position(|&i| i == c))
                    .map(|i| (i + 1) % issues.len())
                    .unwrap_or(0);
                if let Some((id, cursor)) = issues.get_mut(next) {
                    self.focus_request = Some((*id, *cursor));
                    self.scroll_target = Some(*id);
                }
            }
        }
        self.navigate_blocks(ctx);
        self.block_shortcuts(ctx);
        if let Some(target) = self.slash_target {
            let keys = ctx.input_mut(|input| {
                (
                    input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                    input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                    input.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                    input.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
                )
            });
            let commands = self.commands();
            if keys.0 && !commands.is_empty() {
                self.command_scrolling = true;
                self.command_index = (self.command_index + 1) % commands.len();
            }
            if keys.1 && !commands.is_empty() {
                self.command_scrolling = true;
                self.command_index = (self.command_index + commands.len() - 1) % commands.len();
            }
            if keys.2 {
                if let Some(kind) =
                    commands.get(self.command_index.min(commands.len().saturating_sub(1)))
                {
                    self.apply_command(target, *kind);
                }
            }
            if keys.3 {
                self.dismissed_command = self.blocks.get(target).map(|b| b.id);
                self.slash_target = None;
            }
        }
        let keys = ctx.input_mut(|input| {
            (
                input.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::S,
                ),
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::S),
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::O),
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::N),
                input.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::Z,
                ),
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::Z),
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::Y),
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::F),
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::B),
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::I),
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::U),
            )
        });
        if keys.0 {
            self.save_document(true);
        } else if keys.1 {
            self.save_document(false);
        }
        if keys.2 {
            self.request_action(PendingAction::Open, ctx);
        }
        if keys.3 {
            self.request_action(PendingAction::New, ctx);
        }
        if !self.show_home && (keys.4 || keys.6) {
            self.redo();
        } else if !self.show_home && keys.5 {
            self.undo();
        }
        if keys.7 {
            if self.show_home {
                ctx.memory_mut(|m| m.request_focus(egui::Id::new("home_search")));
            } else {
                self.find_open = true;
                ctx.memory_mut(|m| m.request_focus(egui::Id::new("find_query")));
            }
        }
        if !self.show_home && keys.8 {
            self.toggle_decoration(Decoration::Bold, ctx);
        }
        if !self.show_home && keys.9 {
            self.toggle_decoration(Decoration::Italic, ctx);
        }
        if !self.show_home && keys.10 {
            self.toggle_decoration(Decoration::Underline, ctx);
        }
        if self.find_open
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.find_open = false;
        }
    }

    fn inspector(&mut self, ctx: &egui::Context) {
        if !self.show_inspector {
            return;
        }
        design::detail_panel(ctx, "page_setup", self.detail_overlay, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Page setup").size(15.0).strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icons::control(ui, Icon::Close, "Close", "Close panel", true, false)
                        .clicked()
                    {
                        self.show_inspector = false;
                    }
                });
            });
            ui.add_space(20.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label("This is a writing view, not paginated paper. Presets set layout defaults; check your assignment requirements.");
                    if icons::labelled(ui, Icon::Preview, "Open paper preview").clicked() { self.paper_preview(); }
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new("Document format")
                            .size(12.0)
                            .color(theme::current().muted),
                    );
                    ui.add_space(5.0);
                    for format in [PaperFormat::Freeform, PaperFormat::Apa, PaperFormat::Mla] {
                        if ui
                            .selectable_label(self.style.paper_format == format, format.label())
                            .hand()
                            .clicked()
                        {
                            self.style.apply_preset(format);
                            self.dirty = true;
                            self.force_history = true;
                        }
                    }
                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(14.0);
                    for (label, value, range, suffix) in [
                        ("Margins", &mut self.style.margin_inches, 0.3..=2.0, " in"),
                        (
                            "Paragraph indent",
                            &mut self.style.first_line_indent_inches,
                            0.0..=1.0,
                            " in",
                        ),
                        (
                            "Line spacing",
                            &mut self.style.line_spacing,
                            1.0..=3.0,
                            " ×",
                        ),
                    ] {
                        ui.label(
                            RichText::new(label)
                                .size(12.0)
                                .color(theme::current().muted),
                        );
                        let response = ui.add(
                                egui::DragValue::new(value)
                                    .range(range)
                                    .speed(0.05)
                                    .suffix(suffix),
                            );
                        response.widget_info(|| { let mut info = egui::WidgetInfo::drag_value(true, *value as f64); info.label = Some(label.into()); info });
                        if response.changed() {
                            self.dirty = true;
                        }
                        ui.add_space(12.0);
                    }
                    if self.style.paper_format != PaperFormat::Freeform {
                        ui.separator();
                        ui.add_space(14.0);
                        ui.label(RichText::new("Paper details").size(13.0).strong());
                        ui.add_space(12.0);
                        for (label, value) in [
                            ("Author", &mut self.details.author),
                            ("Last name", &mut self.details.last_name),
                            ("Instructor", &mut self.details.instructor),
                            ("Course", &mut self.details.course),
                            ("Date", &mut self.details.due_date),
                        ] {
                            ui.label(
                                RichText::new(label)
                                    .size(12.0)
                                    .color(theme::current().muted),
                            );
                            if design::named(ui.add(
                                    TextEdit::singleline(value)
                                        .hint_text(label)
                                        .desired_width(f32::INFINITY)
                                        .margin(Margin::same(7.0)),
                                ), egui::WidgetType::TextEdit, label)
                                .changed()
                            {
                                self.dirty = true;
                            }
                            ui.add_space(10.0);
                        }
                    }
                });
        });
    }

    fn status_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(30.0)
            .frame(
                Frame::none()
                    .fill(theme::current().canvas)
                    .inner_margin(Margin::symmetric(22.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let words = self
                        .blocks
                        .iter()
                        .map(|b| b.text.split_whitespace().count())
                        .sum::<usize>();
                    let chars = self
                        .blocks
                        .iter()
                        .map(|b| b.text.chars().count())
                        .sum::<usize>();
                    let mut stats = Vec::new();
                    if self.settings.editor.words {
                        stats.push(format!("{words} words"));
                    }
                    if self.settings.editor.characters {
                        stats.push(format!("{chars} characters"));
                    }
                    if self.settings.editor.reading_time {
                        stats.push(format!("{} min read", words.div_ceil(200).max(1)));
                    }
                    if let Some((id, start, end)) =
                        self.selection.filter(|(_, start, end)| start < end)
                    {
                        if let Some(block) = self.blocks.iter().find(|b| b.id == id) {
                            stats.push(format!(
                                "{} words selected",
                                block
                                    .text
                                    .chars()
                                    .skip(start)
                                    .take(end - start)
                                    .collect::<String>()
                                    .split_whitespace()
                                    .count()
                            ));
                        }
                    }
                    ui.label(
                        RichText::new(stats.join(" · "))
                            .size(12.0)
                            .color(theme::current().muted),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(self.style.paper_format.label())
                            .size(12.0)
                            .color(theme::current().muted),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if self.settings.editor.word_goal > 0 {
                            ui.add(
                                egui::ProgressBar::new(
                                    (words as f32 / self.settings.editor.word_goal as f32).min(1.0),
                                )
                                .desired_width(150.0)
                                .text(format!("{words}/{}", self.settings.editor.word_goal)),
                            );
                            return;
                        }
                        ui.label(
                            RichText::new(if self.dirty {
                                "Saving…"
                            } else {
                                "Saved locally"
                            })
                            .size(12.0)
                            .color(theme::current().muted),
                        );
                    });
                });
            });
    }

    fn canvas(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                Frame::none().fill(
                    self.style
                        .page_color
                        .map(|c| Color32::from_rgb(c[0], c[1], c[2]))
                        .unwrap_or(theme::current().canvas),
                ),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt(("document_scroll", &self.active_document))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let width = (ui.available_width() - 32.0).max(180.0).min(780.0);
                        let leading = ((ui.available_width() - width) * 0.5).max(8.0);
                        ui.add_space(28.0);
                        ui.horizontal(|ui| {
                            ui.add_space(leading);
                            ui.allocate_ui_with_layout(
                                Vec2::new(width, 0.0),
                                Layout::top_down(Align::LEFT),
                                |ui| self.document(ui, ctx, width),
                            );
                        });
                        ui.add_space(180.0);
                    });
            });
    }

    fn document(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, width: f32) {
        let academic = self.style.paper_format != PaperFormat::Freeform;
        if self.style.paper_format == PaperFormat::Mla {
            for value in [
                &self.details.author,
                &self.details.instructor,
                &self.details.course,
                &self.details.due_date,
            ] {
                if !value.is_empty() {
                    ui.label(RichText::new(value).font(document_font(
                        &self.style,
                        InlineStyle::default(),
                        16.0,
                    )));
                    ui.add_space(9.0);
                }
            }
            if !self.details.author.is_empty() {
                ui.add_space(12.0);
            }
        }
        let title = ui.add(
            TextEdit::multiline(&mut self.title)
                .id(egui::Id::new(("document_title", &self.active_document)))
                .font(if academic {
                    document_font(&self.style, InlineStyle::default(), 16.0)
                } else {
                    FontId::new(36.0, FontFamily::Name("Liberation Sans Bold".into()))
                })
                .horizontal_align(if academic { Align::Center } else { Align::LEFT })
                .frame(false)
                .desired_rows(1)
                .return_key(None)
                .desired_width(width)
                .margin(Margin::symmetric(8.0, 7.0))
                .hint_text("Untitled"),
        );
        if title.changed() {
            self.dirty = true;
        }
        design::named(title, egui::WidgetType::TextEdit, "Document title");
        ui.add_space(if academic { 20.0 } else { 28.0 });
        let mut action = None;
        let mut counters = [0usize; 7];
        for index in 0..self.blocks.len() {
            let id = self.blocks[index].id;
            let kind = self.blocks[index].kind;
            if kind == BlockKind::Numbered {
                let level = self.blocks[index].indent.min(6) as usize;
                counters[level] += 1;
                counters[level + 1..].fill(0);
            } else if !matches!(kind, BlockKind::Bulleted | BlockKind::Todo) {
                counters.fill(0);
            }
            let number = counters[self.blocks[index].indent.min(6) as usize];
            let row = ui.horizontal_top(|ui| {
                let (handle_rect, handle) =
                    ui.allocate_exact_size(Vec2::new(24.0, 28.0), Sense::click());
                handle.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Block options")
                });
                if handle.hand().clicked() {
                    self.block_picker_target = Some(index);
                    self.command_position = handle_rect.left_bottom() + Vec2::new(0.0, 4.0);
                }
                let response = if kind == BlockKind::Image {
                    let body_width = ui.available_width().min(width - 30.0);
                    ui.vertical(|ui| self.image_editor(ui, ctx, index, body_width))
                        .inner
                } else if kind == BlockKind::Divider {
                    self.divider_editor(ui, ctx, index, ui.available_width().min(width - 30.0))
                } else {
                    ui.add_space(self.blocks[index].indent.min(6) as f32 * 24.0);
                    if kind == BlockKind::Todo {
                        let task = self.blocks[index].text.clone();
                        let response = ui.checkbox(&mut self.blocks[index].checked, "").hand();
                        response.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::Checkbox,
                                true,
                                self.blocks[index].checked,
                                format!("Task: {task}"),
                            )
                        });
                        if response.changed() {
                            self.dirty = true;
                        }
                    }
                    if kind == BlockKind::Bulleted {
                        ui.label(
                            RichText::new(list_marker(self.blocks[index].indent, number, true))
                                .size(19.0)
                                .color(theme::current().ink),
                        );
                    }
                    if kind == BlockKind::Numbered {
                        ui.label(
                            RichText::new(list_marker(self.blocks[index].indent, number, false))
                                .size(16.0)
                                .color(theme::current().muted),
                        );
                    }
                    let quote_x = (kind == BlockKind::Quote).then(|| {
                        let x = ui.cursor().left() + 3.0;
                        ui.add_space(16.0);
                        x
                    });
                    let body_width = ui.available_width().min(width - 30.0);
                    let response = self.container_editor(ui, ctx, index, body_width);
                    if let Some(x) = quote_x {
                        // Measure after TextEdit has wrapped every line; a glyph
                        // or fixed-height widget cannot follow multiline content.
                        ui.painter().line_segment(
                            [
                                Pos2::new(x, response.rect.top()),
                                Pos2::new(x, response.rect.bottom()),
                            ],
                            Stroke::new(2.0_f32, theme::current().muted),
                        );
                    }
                    response
                };
                response.context_menu(|ui| {
                    if kind == BlockKind::Image {
                        self.image_options(ui, index);
                        ui.separator();
                    }
                    self.proofing_menu(ui, index);
                    ui.label(
                        RichText::new("Block actions")
                            .size(12.0)
                            .color(theme::current().muted),
                    );
                    if icons::labelled(ui, Icon::Copy, "Duplicate").clicked() {
                        action = Some(BlockAction::Duplicate(index));
                        ui.close_menu();
                    }
                    if icons::labelled_enabled(ui, Icon::Up, "Move up", index > 0).clicked() {
                        action = Some(BlockAction::Up(index));
                        ui.close_menu();
                    }
                    if icons::labelled_enabled(
                        ui,
                        Icon::Down,
                        "Move down",
                        index + 1 < self.blocks.len(),
                    )
                    .clicked()
                    {
                        action = Some(BlockAction::Down(index));
                        ui.close_menu();
                    }
                    ui.separator();
                    if icons::danger(ui, "Delete").clicked() {
                        action = Some(BlockAction::Delete(index));
                        ui.close_menu();
                    }
                });
                if response.has_focus()
                    && !matches!(
                        kind,
                        BlockKind::Code
                            | BlockKind::Callout
                            | BlockKind::Image
                            | BlockKind::Divider
                    )
                    && !self.blocks[index].text.trim_start().starts_with('/')
                    && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift)
                {
                    let (_, start, end) = self.selection.unwrap_or((
                        id,
                        self.blocks[index].text.chars().count(),
                        self.blocks[index].text.chars().count(),
                    ));
                    action = Some(BlockAction::Split(index, start, end));
                }
                (handle_rect, response.rect)
            });
            if self.settings.editor.handles
                && (row.response.hovered() || self.focused_block == Some(id))
            {
                for x in [-3.0, 3.0] {
                    for y in [-5.0, 0.0, 5.0] {
                        ui.painter().circle_filled(
                            row.inner.0.center() + Vec2::new(x, y),
                            1.1,
                            theme::current().muted,
                        );
                    }
                }
            }
            let links = wikilinks::links(&self.blocks[index].text);
            if !links.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(30.0);
                    for link in links {
                        let target =
                            wikilinks::resolve(&link, &self.blocks[index], &self.workspace);
                        let label = if let Some(target) = target {
                            format!("Open {}", target.document.title)
                        } else {
                            format!("{} (not found)", link.title)
                        };
                        if ui.link(label).hand().clicked() {
                            self.link_request =
                                Some(target.map(|d| d.id.clone()).unwrap_or(link.title));
                        }
                    }
                });
            }
            if self.scroll_target == Some(id) {
                row.response.scroll_to_me(Some(Align::Center));
                self.scroll_target = None;
            }
            ui.add_space(
                if matches!(kind, BlockKind::Heading1 | BlockKind::Heading2) {
                    12.0
                } else if academic {
                    0.0
                } else {
                    6.0
                },
            );
        }
        if let Some(action) = action {
            self.block_action(action);
        }
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 64.0), Sense::click());
        if response.hovered() {
            ui.painter().rect_filled(rect, 4.0, theme::current().panel);
        }
        if response.on_hover_cursor(egui::CursorIcon::Text).clicked() {
            let last = self.blocks.len() - 1;
            if self.blocks[last].text.is_empty()
                && !matches!(
                    self.blocks[last].kind,
                    BlockKind::Image | BlockKind::Divider
                )
            {
                self.focus_request = Some((self.blocks[last].id, 0));
            } else {
                self.block_action(BlockAction::Split(
                    last,
                    self.blocks[last].text.chars().count(),
                    self.blocks[last].text.chars().count(),
                ));
            }
        }
        self.citations_preview(ui, width);
    }

    pub(super) fn block_editor(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        index: usize,
        width: f32,
    ) -> egui::Response {
        let id = self.blocks[index].id;
        let kind = self.blocks[index].kind;
        let widget_id = egui::Id::new(("block", &self.active_document, id));
        if let Some((target, cursor)) = self.focus_request {
            if target == id {
                ctx.memory_mut(|m| m.request_focus(widget_id));
                let mut state = TextEdit::load_state(ctx, widget_id).unwrap_or_default();
                state
                    .cursor
                    .set_char_range(Some(CCursorRange::one(CCursor::new(cursor))));
                state.store(ctx, widget_id);
                self.focus_request = None;
            }
        }
        let old_text = self.blocks[index].text.clone();
        let old_styles = self.blocks[index].character_styles();
        let marks = self.blocks[index].marks.clone();
        let base = self.blocks[index].format;
        let style = self.style.clone();
        let completed = kind == BlockKind::Todo && self.blocks[index].checked;
        let matches = if self.find_open {
            search::ranges(
                &old_text,
                &self.find_query,
                self.search.case_sensitive,
                self.search.whole_word,
            )
        } else {
            Vec::new()
        };
        let size = match kind {
            BlockKind::Heading1 => 28.0,
            BlockKind::Heading2 => 22.0,
            BlockKind::Code => 16.0,
            _ => style.font_size * 4.0 / 3.0,
        };
        let mut layouter = |ui: &egui::Ui, text: &str, wrap_width: f32| {
            ui.fonts(|fonts| {
                let mut job =
                    text_layout::job(text, base, &marks, &style, kind, size, wrap_width, &matches);
                if completed {
                    for section in &mut job.sections {
                        section.format.color = theme::current().muted;
                        section.format.strikethrough = Stroke::new(0.8_f32, theme::current().muted);
                    }
                }
                fonts.layout_job(job)
            })
        };
        let output = TextEdit::multiline(&mut self.blocks[index].text)
            .id(widget_id)
            .lock_focus(matches!(
                kind,
                BlockKind::Code | BlockKind::Bulleted | BlockKind::Numbered | BlockKind::Todo
            ))
            .frame(false)
            .desired_width(width)
            .desired_rows(1)
            .font(document_font(&style, base, size))
            .layouter(&mut layouter)
            .margin(Margin::symmetric(3.0, 4.0))
            .hint_text(
                RichText::new(if !ctx.memory(|m| m.has_focus(widget_id)) {
                    ""
                } else if kind == BlockKind::Code {
                    "Write code…"
                } else if kind == BlockKind::Callout {
                    "Type a note…"
                } else if kind == BlockKind::Heading1 || kind == BlockKind::Heading2 {
                    "Heading"
                } else {
                    "Write something, or type '/' to insert…"
                })
                .color(theme::current().muted),
            )
            .return_key(Some(egui::KeyboardShortcut::new(
                if matches!(kind, BlockKind::Code | BlockKind::Callout) {
                    egui::Modifiers::NONE
                } else {
                    egui::Modifiers::SHIFT
                },
                egui::Key::Enter,
            )))
            .show(ui);
        ctx.data_mut(|data| data.insert_temp(widget_id.with("layout"), output.galley.clone()));
        design::named(
            output.response.clone(),
            egui::WidgetType::TextEdit,
            &format!("{} block text", self.blocks[index].kind.label()),
        );
        self.paint_proofing(ui, index, &output);
        if output.response.gained_focus() {
            // egui installs TextEdit's arrow-key focus lock on the following pass.
            // Finish that pass now so consecutive arrow presses cannot trigger
            // spatial widget navigation and skip an empty block.
            ctx.request_discard("Settle document block keyboard focus");
        } else if output.response.lost_focus() {
            ctx.request_repaint();
        }
        if output.response.has_focus() {
            self.focused_block = Some(id);
            if let Some(range) = output.state.cursor.char_range() {
                let [a, b] = range.sorted();
                self.selection = Some((id, a.index, b.index));
                if a.index != b.index {
                    self.selection_popup = Some(output.response.rect.min + Vec2::new(0.0, -42.0));
                }
            }
        }
        if output.response.changed() {
            self.blocks[index].rebase_formatting(&old_text, &old_styles);
            self.dirty = true;
            self.dismissed_command = None;
        }
        if output.response.has_focus()
            && self.settings.editor.slash
            && self.blocks[index].text.trim_start().starts_with('/')
            && self.dismissed_command != Some(id)
        {
            let query = self.blocks[index].text.clone();
            if self.command_query != query {
                self.command_index = 0;
                self.command_query = query;
            }
            self.slash_target = Some(index);
            self.command_position = output.response.rect.left_bottom() + Vec2::new(0.0, 3.0);
        } else if self.slash_target == Some(index)
            && !self.blocks[index].text.trim_start().starts_with('/')
        {
            self.slash_target = None;
        }
        output.response
    }

    fn block_action(&mut self, action: BlockAction) {
        self.force_history = true;
        self.dirty = true;
        self.slash_target = None;
        self.block_picker_target = None;
        match action {
            BlockAction::Split(index, start, end) => {
                let mut current = self.blocks[index].clone();
                if current.text.is_empty()
                    && matches!(
                        current.kind,
                        BlockKind::Bulleted | BlockKind::Numbered | BlockKind::Todo
                    )
                {
                    if current.indent > 0 {
                        self.blocks[index].indent -= 1;
                    } else {
                        self.blocks[index].kind = BlockKind::Text;
                    }
                    return;
                }
                let length = current.text.chars().count();
                let start = start.min(length);
                let end = end.max(start).min(length);
                let styles = current.character_styles();
                let left: String = current.text.chars().take(start).collect();
                let right: String = current.text.chars().skip(end).collect();
                current.text = left;
                current.set_character_styles(&styles[..start]);
                let next_kind = if matches!(
                    current.kind,
                    BlockKind::Bulleted | BlockKind::Numbered | BlockKind::Todo
                ) {
                    current.kind
                } else {
                    BlockKind::Text
                };
                let mut next = self.make_block(next_kind, right);
                next.format = current.format;
                next.indent = current.indent;
                next.set_character_styles(&styles[end..]);
                let id = next.id;
                self.blocks[index] = current;
                self.blocks.insert(index + 1, next);
                self.focus_request = Some((id, 0));
                self.focused_block = Some(id);
            }
            BlockAction::Delete(index) => {
                if self.blocks.len() == 1 {
                    self.blocks[0].text.clear();
                    self.blocks[0].marks.clear();
                    self.blocks[0].kind = BlockKind::Text;
                } else {
                    self.blocks.remove(index);
                    let previous = index.saturating_sub(1).min(self.blocks.len() - 1);
                    let block = &self.blocks[previous];
                    self.focus_request = Some((block.id, block.text.chars().count()));
                }
            }
            BlockAction::Duplicate(index) => {
                let mut block = self.blocks[index].clone();
                block.id = self.next_id;
                self.next_id += 1;
                let id = block.id;
                self.blocks.insert(index + 1, block);
                self.focus_request = Some((id, 0));
            }
            BlockAction::Up(index) => {
                self.move_block_tree(index, false);
            }
            BlockAction::Down(index) => {
                self.move_block_tree(index, true);
            }
        }
    }

    fn popups(&mut self, ctx: &egui::Context) {
        if let Some(target) = self.slash_target {
            let commands = self.commands();
            self.command_index = self.command_index.min(commands.len().saturating_sub(1));
            let mut position = self.command_position;
            let screen = ctx.screen_rect();
            position.x = position.x.min(screen.right() - 312.0).max(8.0);
            if position.y + 270.0 > screen.bottom() - 36.0 {
                position.y = (position.y - 280.0).max(112.0);
            }
            let mut selected = None;
            let popup = egui::Area::new(egui::Id::new("slash_popup"))
                .order(egui::Order::Foreground)
                .fixed_pos(position)
                .show(ctx, |ui| {
                    Frame::popup(ui.style())
                        .fill(theme::current().panel)
                        .rounding(8.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.set_width(290.0);
                            ui.label(
                                RichText::new("Insert a block")
                                    .size(11.0)
                                    .color(theme::current().muted),
                            );
                            ui.add_space(6.0);
                            egui::ScrollArea::vertical()
                                .max_height(204.0)
                                .show(ui, |ui| {
                                    if commands.is_empty() {
                                        ui.label(
                                            RichText::new("No matching blocks")
                                                .color(theme::current().muted),
                                        );
                                    }
                                    for (i, kind) in commands.iter().enumerate() {
                                        let response = ui.add_sized(
                                            [286.0, 32.0],
                                            egui::Button::image_and_text(
                                                icons::block(*kind).image(ui, theme::current().ink),
                                                RichText::new(kind.label()).size(13.0),
                                            )
                                            .selected(i == self.command_index),
                                        );
                                        if response.clone().hand().clicked() {
                                            selected = Some(*kind);
                                        }
                                        if i == self.command_index && self.command_scrolling {
                                            response.scroll_to_me(Some(Align::Center));
                                        }
                                        response.on_hover_text(kind.hint());
                                    }
                                });
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("↑ ↓ choose   Enter insert   Esc close")
                                    .size(10.0)
                                    .color(theme::current().muted),
                            );
                        });
                });
            if let Some(kind) = selected {
                self.apply_command(target, kind);
            } else if ctx.input(|i| {
                i.pointer.any_pressed()
                    && i.pointer
                        .interact_pos()
                        .map(|p| !popup.response.rect.contains(p))
                        .unwrap_or(false)
            }) {
                self.dismissed_command = self.blocks.get(target).map(|b| b.id);
                self.slash_target = None;
            }
        }
        if let Some(index) = self.block_picker_target {
            let mut action = None;
            let mut new_kind = None;
            let mut position = self.command_position;
            position.x = position.x.min(ctx.screen_rect().right() - 228.0).max(8.0);
            position.y = position.y.min(ctx.screen_rect().bottom() - 210.0);
            egui::Area::new(egui::Id::new("block_options"))
                .order(egui::Order::Foreground)
                .fixed_pos(position)
                .show(ctx, |ui| {
                    Frame::popup(ui.style()).inner_margin(8.0).show(ui, |ui| {
                        ui.set_width(200.0);
                        icons::menu(ui, Icon::Text, "Turn into", |ui| {
                            for kind in BlockKind::ALL {
                                if icons::labelled(ui, icons::block(kind), kind.label()).clicked() {
                                    new_kind = Some(kind);
                                    ui.close_menu();
                                }
                            }
                        });
                        if icons::labelled(ui, Icon::Copy, "Duplicate").clicked() {
                            action = Some(BlockAction::Duplicate(index));
                        }
                        if icons::labelled_enabled(ui, Icon::Up, "Move up", index > 0).clicked() {
                            action = Some(BlockAction::Up(index));
                        }
                        if icons::labelled_enabled(
                            ui,
                            Icon::Down,
                            "Move down",
                            index + 1 < self.blocks.len(),
                        )
                        .clicked()
                        {
                            action = Some(BlockAction::Down(index));
                        }
                        ui.separator();
                        if icons::danger(ui, "Delete").clicked() {
                            action = Some(BlockAction::Delete(index));
                        }
                        if icons::control(
                            ui,
                            Icon::Close,
                            "Close",
                            "Close block options",
                            true,
                            false,
                        )
                        .clicked()
                        {
                            self.block_picker_target = None;
                        }
                    });
                });
            if let Some(kind) = new_kind {
                self.apply_command(index, kind);
            }
            if let Some(action) = action {
                self.block_action(action);
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.block_picker_target = None;
            }
        }
        if let Some(pending) = self.pending_action {
            let mut save = false;
            let mut discard = false;
            let mut cancel = false;
            egui::Window::new("Save your changes?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label("This document has changes that haven’t been saved.");
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        save = ui.button("Save and continue").hand().clicked();
                        discard = ui.button("Discard changes").hand().clicked();
                        cancel = ui.button("Cancel").hand().clicked();
                    });
                });
            if cancel {
                self.pending_action = None;
            }
            if discard {
                self.execute_action(pending, ctx);
            }
            if save && self.save_document(false) {
                self.execute_action(pending, ctx);
            }
        }
        self.notification(ctx);
    }
}

impl FlowriteApp {
    pub(super) fn workspace_view(&mut self, ctx: &egui::Context) {
        if self.settings_open {
            self.settings_page(ctx);
            return;
        }
        self.header(ctx);
        // Navigation precedes document-only chrome, preserving its visual landmarks.
        if !self.focus_mode {
            self.library_sidebar(ctx);
        }
        if self.show_home {
            self.home_page(ctx);
            return;
        }
        if !self.focus_mode {
            self.document_tabs(ctx);
            if self.settings.editor.toolbar {
                self.toolbar(ctx);
            }
        }
        self.find_bar(ctx);
        self.status_bar(ctx);
        self.detail_overlay = ctx.available_rect().width() < design::WRITING_MIN + 286.0;
        if self.show_citations && self.show_inspector {
            self.show_inspector = false;
        }
        if !self.focus_mode {
            self.citations_panel(ctx);
            self.inspector(ctx);
        }
        self.canvas(ctx);
        self.selection_toolbar(ctx);
    }
}

impl eframe::App for FlowriteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.history_navigation = false;
        if ctx.input(|i| i.viewport().close_requested())
            && !self.allow_close
            && self.dirty
            && !self.save_workspace()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending_action = Some(PendingAction::Close);
        }
        if self.history_baseline.is_none() {
            self.reset_history_baseline();
        }
        self.handle_keys(ctx);
        self.workspace_view(ctx);
        self.popups(ctx);
        self.folder_dialog_ui(ctx);
        self.library_dialogs(ctx);
        self.recovery_dialog(ctx);
        self.command_palette(ctx);
        self.writing_help(ctx);
        self.proofing_review(ctx);
        let now = ctx.input(|i| i.time);
        self.record_ui_edits(ctx, now);
        self.update_proofing(ctx);
        if let Some(title) = self.link_request.take() {
            self.follow_document_link(title);
        }
        if self.dirty && !self.active_document.is_empty() {
            if now - self.last_document_change >= self.settings.saving.delay {
                self.save_workspace();
                self.last_document_change = now;
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        self.force_history = false;
        self.command_scrolling = false;
    }
}

fn decoration_value(style: InlineStyle, decoration: Decoration) -> bool {
    match decoration {
        Decoration::Bold => style.bold,
        Decoration::Italic => style.italic,
        Decoration::Underline => style.underline,
    }
}
fn set_decoration(style: &mut InlineStyle, decoration: Decoration, value: bool) {
    match decoration {
        Decoration::Bold => style.bold = value,
        Decoration::Italic => style.italic = value,
        Decoration::Underline => style.underline = value,
    }
}

pub(super) fn document_font(style: &DocumentStyle, format: InlineStyle, size: f32) -> FontId {
    let base = fonts::family(&style.font);
    let name = match (format.bold, format.italic) {
        (true, true) => format!("{base} Bold Italic"),
        (true, false) => format!("{base} Bold"),
        (false, true) => format!("{base} Italic"),
        _ => base.to_owned(),
    };
    FontId::new(size, FontFamily::Name(name.into()))
}

pub(super) fn configure_fonts(ctx: &egui::Context) {
    fonts::configure(ctx);
}

pub(super) fn configure_visuals(ctx: &egui::Context, theme: &theme::Theme) {
    theme::apply(*theme);
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = theme::current().panel;
    visuals.window_fill = theme::current().panel;
    visuals.extreme_bg_color = theme::current().canvas;
    visuals.override_text_color = Some(theme::current().ink);
    visuals.faint_bg_color = theme.surface;
    visuals.selection.bg_fill = theme.surface;
    visuals.selection.stroke = Stroke::NONE;
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.bg_fill = theme.surface;
        widget.weak_bg_fill = theme.surface;
        widget.bg_stroke = Stroke::new(1.0_f32, theme::current().stroke);
        widget.rounding = Rounding::same(5.0);
    }
    visuals.widgets.hovered.bg_fill = theme.hover;
    visuals.widgets.hovered.weak_bg_fill = theme.hover;
    visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, theme.accent);
    visuals.window_rounding = Rounding::same(9.0);
    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style.spacing.item_spacing = Vec2::new(8.0, 8.0);
        style.spacing.button_padding = Vec2::new(12.0, 6.0);
        style.spacing.interact_size.y = design::ROW_HEIGHT;
        style.spacing.window_margin = Margin::same(design::SPACE[4]);
        style.interaction.tooltip_delay = 0.25;
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(14.0));
    });
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod render_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_rule_spans_explicit_lines_and_wrapped_text() {
        for text in [
            "Test\nfdsn\nfndslkn".to_owned(),
            "A long quotation that wraps naturally across the writing column. ".repeat(12),
        ] {
            for scale in [1.0, 1.5] {
                let ctx = egui::Context::default();
                configure_fonts(&ctx);
                ctx.set_zoom_factor(scale);
                let mut app = FlowriteApp::empty();
                app.blocks = vec![Block::new(1, BlockKind::Quote, &text)];
                let mut output = None;
                for _ in 0..3 {
                    output = Some(ctx.run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                Pos2::ZERO,
                                Vec2::new(500.0, 820.0) / scale,
                            )),
                            ..Default::default()
                        },
                        |ctx| app.canvas(ctx),
                    ));
                }
                let output = output.unwrap();
                let painted = output
                    .shapes
                    .iter()
                    .find(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.job.text == text))
                    .expect("quote text rendered");
                let text_bounds = painted.shape.visual_bounding_rect();
                let line = output
                    .shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::LineSegment { points, stroke }
                            if stroke.width == 2.0
                                && points[0].x == points[1].x
                                && points[0].x < text_bounds.left() =>
                        {
                            Some(points)
                        }
                        _ => None,
                    })
                    .expect("quote has a measured vertical rule");
                assert!(line[0].y <= text_bounds.top());
                assert!(
                    line[1].y >= text_bounds.bottom(),
                    "rule stops above the quote's final line"
                );
                assert!(line[1].y - line[0].y > 40.0);
            }
        }
    }

    #[test]
    fn header_actions_do_not_paint_immediate_labels() {
        let mut app = FlowriteApp::empty();
        let ctx = egui::Context::default();
        configure_fonts(&ctx);
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    Pos2::ZERO,
                    Vec2::new(1000.0, 300.0),
                )),
                ..Default::default()
            },
            |ctx| app.header(ctx),
        );
        for forbidden in ["File", "Save", "Export", "Find", "Undo", "Redo"] {
            assert!(
                !output.shapes.iter().any(
                    |s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.job.text == forbidden)
                ),
                "Visible action label: {forbidden}"
            );
        }
    }

    fn block_frame(
        app: &mut FlowriteApp,
        ctx: &egui::Context,
        key: Option<egui::Key>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                events: key
                    .into_iter()
                    .map(|key| egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    })
                    .collect(),
                ..Default::default()
            },
            |ctx| {
                app.handle_keys(ctx);
                egui::CentralPanel::default().show(ctx, |ui| {
                    for i in 0..app.blocks.len() {
                        if app.blocks[i].kind == BlockKind::Divider {
                            app.divider_editor(ui, ctx, i, 180.0);
                        } else if app.blocks[i].kind == BlockKind::Image {
                            app.image_editor(ui, ctx, i, 180.0);
                        } else {
                            app.block_editor(ui, ctx, i, 180.0);
                        }
                    }
                });
            },
        )
    }

    fn test_editor() -> (FlowriteApp, egui::Context) {
        let ctx = egui::Context::default();
        configure_fonts(&ctx);
        let mut app = FlowriteApp::empty();
        app.show_home = false;
        (app, ctx)
    }

    fn input_frame(
        app: &mut FlowriteApp,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        modifiers: egui::Modifiers,
    ) {
        let _ = ctx.run(
            egui::RawInput {
                events,
                modifiers,
                ..Default::default()
            },
            |ctx| {
                app.handle_keys(ctx);
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.block_editor(ui, ctx, 0, 300.0);
                    let _ = ui.button("Next focusable control").hand();
                });
                app.record_edits(ctx.input(|i| i.time));
            },
        );
    }

    #[test]
    fn indentation_retains_real_textedit_focus_and_next_character() {
        for kind in [
            BlockKind::Code,
            BlockKind::Bulleted,
            BlockKind::Numbered,
            BlockKind::Todo,
        ] {
            for original in ["parent", "café Ω"] {
                let (mut app, ctx) = test_editor();
                app.active_document = "test".into();
                app.blocks[0].kind = kind;
                app.blocks[0].text = original.into();
                app.focus_request = Some((1, original.chars().count()));
                input_frame(&mut app, &ctx, vec![], egui::Modifiers::NONE);
                app.reset_history_baseline();
                let key = |modifiers| egui::Event::Key {
                    key: egui::Key::Tab,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                };
                input_frame(
                    &mut app,
                    &ctx,
                    vec![key(egui::Modifiers::NONE)],
                    egui::Modifiers::NONE,
                );
                let widget = egui::Id::new(("block", &app.active_document, 1u64));
                assert!(
                    ctx.memory(|m| m.has_focus(widget)),
                    "Tab lost {kind:?} focus"
                );
                input_frame(
                    &mut app,
                    &ctx,
                    vec![egui::Event::Text("x".into())],
                    egui::Modifiers::NONE,
                );
                assert!(
                    app.blocks[0].text.ends_with('x'),
                    "next character lost for {kind:?}"
                );
                if kind != BlockKind::Code {
                    assert_eq!(app.blocks[0].indent, 1);
                }
                input_frame(
                    &mut app,
                    &ctx,
                    vec![key(egui::Modifiers::SHIFT)],
                    egui::Modifiers::NONE,
                );
                assert!(ctx.memory(|m| m.has_focus(widget)), "Shift+Tab lost focus");
                if kind != BlockKind::Code {
                    assert_eq!(app.blocks[0].indent, 0);
                }
                app.undo();
                input_frame(&mut app, &ctx, vec![], egui::Modifiers::NONE);
                assert!(ctx.memory(|m| m.has_focus(widget)), "Undo lost focus");
            }
        }
    }

    #[test]
    fn only_focused_empty_block_paints_a_placeholder() {
        let (mut app, ctx) = test_editor();
        app.blocks.push(Block::new(2, BlockKind::Text, ""));
        app.focus_request = Some((2, 0));
        block_frame(&mut app, &ctx, None);
        let output = block_frame(&mut app, &ctx, None);
        let hints = output
            .shapes
            .iter()
            .filter(|shape| {
                matches!(
                    &shape.shape, egui::Shape::Text(text)
                        if text.galley.job.text.contains("Write something")
                )
            })
            .count();
        assert_eq!(hints, 1);
        ctx.memory_mut(|m| m.request_focus(egui::Id::new("other field")));
        let output = block_frame(&mut app, &ctx, None);
        assert!(!output.shapes.iter().any(|shape| matches!(
            &shape.shape, egui::Shape::Text(text)
                if text.galley.job.text.contains("Write something")
        )));
    }

    #[test]
    fn arrows_cross_empty_blocks_and_unicode_edges_without_editing() {
        let (mut app, ctx) = test_editor();
        app.blocks[0].text = "résumé 🎉".into();
        app.blocks.push(Block::new(2, BlockKind::Divider, ""));
        app.blocks.push(Block::new(3, BlockKind::Text, ""));
        app.blocks.push(Block::new(4, BlockKind::Code, "last"));
        app.focus_request = Some((1, 8));
        block_frame(&mut app, &ctx, None);
        let before = app.blocks.clone();
        block_frame(&mut app, &ctx, Some(egui::Key::ArrowRight));
        assert_eq!(app.selection, Some((2, 0, 0)));
        block_frame(&mut app, &ctx, Some(egui::Key::ArrowDown));
        assert_eq!(app.selection, Some((3, 0, 0)));
        block_frame(&mut app, &ctx, Some(egui::Key::ArrowDown));
        assert_eq!(app.selection, Some((4, 0, 0)));
        block_frame(&mut app, &ctx, Some(egui::Key::ArrowUp));
        assert_eq!(app.selection, Some((3, 0, 0)));
        block_frame(&mut app, &ctx, Some(egui::Key::ArrowLeft));
        assert_eq!(app.selection, Some((2, 0, 0)));
        block_frame(&mut app, &ctx, Some(egui::Key::ArrowLeft));
        assert_eq!(app.selection, Some((1, 8, 8)));
        block_frame(&mut app, &ctx, Some(egui::Key::ArrowRight));
        assert_eq!(app.selection, Some((2, 0, 0)));
        assert_eq!(
            serde_json::to_value(&app.blocks).unwrap(),
            serde_json::to_value(&before).unwrap()
        );
        assert!(!app.dirty);
    }

    #[test]
    fn vertical_arrows_use_visual_lines_and_preserve_horizontal_position() {
        let (mut app, ctx) = test_editor();
        app.blocks[0].text =
            "A paragraph that wraps into several visual lines at this narrow width.".into();
        app.blocks.push(Block::new(
            2,
            BlockKind::Text,
            "Second paragraph also wraps into more than one visual line.",
        ));
        app.focus_request = Some((1, 3));
        block_frame(&mut app, &ctx, None);
        let first_id = egui::Id::new(("block", &app.active_document, 1_u64));
        let second_id = egui::Id::new(("block", &app.active_document, 2_u64));
        let first = ctx
            .data(|d| d.get_temp::<std::sync::Arc<egui::Galley>>(first_id.with("layout")))
            .unwrap();
        let second = ctx
            .data(|d| d.get_temp::<std::sync::Arc<egui::Galley>>(second_id.with("layout")))
            .unwrap();
        assert!(first.rows.len() > 1);
        block_frame(&mut app, &ctx, Some(egui::Key::ArrowDown));
        assert_eq!(
            app.focused_block,
            Some(1),
            "movement inside a wrapped paragraph stays in it"
        );
        let end = first.end().ccursor.index;
        app.focus_request = Some((1, end));
        block_frame(&mut app, &ctx, None);
        let x = first.pos_from_cursor(&first.end()).center().x;
        let expected = second
            .cursor_from_pos(Vec2::new(x, second.rows[0].rect.center().y))
            .ccursor
            .index;
        block_frame(&mut app, &ctx, Some(egui::Key::ArrowDown));
        assert_eq!(app.selection, Some((2, expected, expected)));
        block_frame(&mut app, &ctx, Some(egui::Key::ArrowUp));
        assert_eq!(app.focused_block, Some(1));
        let offset = app.selection.unwrap().1;
        assert_eq!(
            first.from_ccursor(CCursor::new(offset)).rcursor.row,
            first.rows.len() - 1
        );
        // Column zero on the last wrapped row must not become the previous row's end.
        app.focus_request = Some((2, 0));
        block_frame(&mut app, &ctx, None);
        block_frame(&mut app, &ctx, Some(egui::Key::ArrowUp));
        let range = TextEdit::load_state(&ctx, first_id)
            .unwrap()
            .cursor
            .char_range()
            .unwrap();
        assert_eq!(
            first.from_ccursor(range.primary).rcursor.row,
            first.rows.len() - 1
        );
    }

    #[test]
    fn navigation_respects_selection_document_edges_and_slash_menu() {
        let (mut app, ctx) = test_editor();
        app.blocks[0].text = "first".into();
        app.blocks.push(Block::new(2, BlockKind::Text, "/"));
        app.focus_request = Some((1, 0));
        block_frame(&mut app, &ctx, None);
        block_frame(&mut app, &ctx, Some(egui::Key::ArrowUp));
        assert_eq!(app.selection, Some((1, 0, 0)));
        let id = egui::Id::new(("block", &app.active_document, 1_u64));
        let mut state = TextEdit::load_state(&ctx, id).unwrap();
        state
            .cursor
            .set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(5))));
        state.store(&ctx, id);
        block_frame(&mut app, &ctx, Some(egui::Key::ArrowRight));
        assert_eq!(
            app.selection,
            Some((1, 5, 5)),
            "collapse selection before leaving a block"
        );
        app.focus_request = Some((2, 1));
        block_frame(&mut app, &ctx, None);
        let command = app.command_index;
        block_frame(&mut app, &ctx, Some(egui::Key::ArrowDown));
        assert_eq!(app.focused_block, Some(2));
        assert_eq!(app.command_index, command + 1);
        block_frame(&mut app, &ctx, Some(egui::Key::Escape));
        block_frame(&mut app, &ctx, Some(egui::Key::ArrowRight));
        assert_eq!(
            app.selection,
            Some((2, 1, 1)),
            "last block does not wrap to the first"
        );
    }

    #[test]
    fn splitting_in_the_middle_preserves_unicode_and_formatting() {
        let mut app = FlowriteApp::empty();
        app.blocks[0].text = "Hello résumé world".to_owned();
        app.blocks[0].marks = vec![FormatSpan {
            start: 6,
            end: 12,
            style: InlineStyle {
                italic: true,
                ..Default::default()
            },
        }];
        app.block_action(BlockAction::Split(0, 13, 13));
        assert_eq!(app.blocks[0].text, "Hello résumé ");
        assert_eq!(app.blocks[1].text, "world");
        assert!(app.blocks[0].character_styles()[6].italic);
        assert!(!app.blocks[1].character_styles()[0].italic);
        assert_eq!(app.focus_request, Some((app.blocks[1].id, 0)));
    }

    #[test]
    fn slash_commands_select_with_enter_and_return_focus() {
        let ctx = egui::Context::default();
        let mut app = FlowriteApp::empty();
        app.blocks[0].text = "/h1".to_owned();
        app.slash_target = Some(0);
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ctx| app.handle_keys(ctx),
        );
        assert_eq!(app.blocks[0].kind, BlockKind::Heading1);
        assert_eq!(app.blocks[0].text, "");
        assert!(app.slash_target.is_none());
        assert_eq!(app.focus_request, Some((1, 0)));
    }

    #[test]
    fn editor_content_fits_in_a_normal_window() {
        let ctx = egui::Context::default();
        configure_fonts(&ctx);
        configure_visuals(&ctx, &theme::Theme::default());
        let mut app = FlowriteApp::empty();
        app.blocks[0].text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit.".to_owned();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    Pos2::ZERO,
                    Vec2::new(1280.0, 820.0),
                )),
                ..Default::default()
            },
            |ctx| {
                app.header(ctx);
                app.toolbar(ctx);
                app.status_bar(ctx);
                app.canvas(ctx);
            },
        );
        let paragraph = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text.contains("Lorem ipsum") => {
                    Some(shape.shape.visual_bounding_rect())
                }
                _ => None,
            })
            .expect("body text should be painted inside the document");
        assert!(
            paragraph.left() > 100.0 && paragraph.right() < 1180.0,
            "paragraph must remain inside the viewport: {paragraph:?}"
        );
        assert!(
            paragraph.top() > 130.0 && paragraph.bottom() < 700.0,
            "paragraph should appear below the title: {paragraph:?}"
        );
    }
}
