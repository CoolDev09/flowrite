use super::*;
use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};

enum BlockAction {
    Split(usize, usize, usize),
    Delete(usize),
    Duplicate(usize),
    Up(usize),
    Down(usize),
}

impl FlowWriteApp {
    fn active_index(&self) -> usize {
        self.focused_block
            .and_then(|id| self.blocks.iter().position(|b| b.id == id))
            .unwrap_or(0)
    }

    fn request_action(&mut self, action: PendingAction, ctx: &egui::Context) {
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

    fn restore(&mut self, document: SavedDocument) {
        self.title = document.title;
        self.blocks = document.blocks;
        self.style = document.style;
        self.details = document.details;
        self.citations = document.citations;
        self.next_id = self.blocks.iter().map(|b| b.id).max().unwrap_or(0) + 1;
        self.dirty = true;
        self.slash_target = None;
        self.selection = None;
        self.focused_block = None;
    }

    fn undo(&mut self) {
        if let Some(previous) = self.undo_stack.pop() {
            self.history_navigation = true;
            self.redo_stack.push(self.local_document());
            self.restore(previous);
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.history_navigation = true;
            self.undo_stack.push(self.local_document());
            self.restore(next);
        }
    }

    fn toggle_decoration(&mut self, decoration: Decoration, ctx: &egui::Context) {
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
            })
            .collect()
    }

    fn handle_keys(&mut self, ctx: &egui::Context) {
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

    fn header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("app_header")
            .exact_height(58.0)
            .frame(
                Frame::none()
                    .fill(CANVAS)
                    .inner_margin(Margin::symmetric(22.0, 10.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    self.brand_button(ui);
                    ui.add_space(10.0);
                    ui.menu_button("File", |ui| {
                        if ui.button("New document               Ctrl+N").clicked() {
                            self.request_action(PendingAction::New, ctx);
                            ui.close_menu();
                        }
                        if ui
                            .button("Open…                           Ctrl+O")
                            .clicked()
                        {
                            self.request_action(PendingAction::Open, ctx);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .button("Save                               Ctrl+S")
                            .clicked()
                        {
                            self.save_document(false);
                            ui.close_menu();
                        }
                        if ui.button("Save as…             Ctrl+Shift+S").clicked() {
                            self.save_document(true);
                            ui.close_menu();
                        }
                    });
                    if ui
                        .add_enabled(!self.undo_stack.is_empty(), egui::Button::new("Undo"))
                        .on_hover_text("Undo · Ctrl+Z")
                        .clicked()
                    {
                        self.undo();
                    }
                    if ui
                        .add_enabled(!self.redo_stack.is_empty(), egui::Button::new("Redo"))
                        .on_hover_text("Redo · Ctrl+Shift+Z")
                        .clicked()
                    {
                        self.redo();
                    }
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(if self.dirty {
                            "Saving…"
                        } else if self.current_path.is_some() || !self.active_document.is_empty() {
                            "All changes saved"
                        } else {
                            "New document"
                        })
                        .size(12.0)
                        .color(MUTED),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.menu_button("Export", |ui| {
                            if ui.button("PDF document (.pdf)").clicked() {
                                ui.close_menu();
                                self.export(ExportTarget::Pdf);
                            }
                            if ui.button("Markdown (.md)").clicked() {
                                ui.close_menu();
                                self.export(ExportTarget::Markdown);
                            }
                        });
                        if ui
                            .button("Save")
                            .on_hover_text("Save editable document · Ctrl+S")
                            .clicked()
                        {
                            self.save_document(false);
                        }
                        if ui
                            .button("Find")
                            .on_hover_text("Find in document · Ctrl+F")
                            .clicked()
                        {
                            self.find_open = !self.find_open;
                            if self.find_open {
                                ctx.memory_mut(|m| m.request_focus(egui::Id::new("find_query")));
                            }
                        }
                    });
                });
            });
    }

    fn toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("format_toolbar")
            .exact_height(48.0)
            .frame(
                Frame::none()
                    .fill(PANEL)
                    .inner_margin(Margin::symmetric(18.0, 7.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if ui
                        .selectable_label(self.show_sidebar, "Sidebar")
                        .on_hover_text("Show your documents")
                        .clicked()
                    {
                        self.show_sidebar = !self.show_sidebar;
                    }
                    ui.separator();
                    let index = self.active_index();
                    let mut kind = self.blocks[index].kind;
                    egui::ComboBox::from_id_salt("block_type")
                        .selected_text(if kind == BlockKind::Text {
                            "Paragraph"
                        } else {
                            kind.label()
                        })
                        .width(126.0)
                        .show_ui(ui, |ui| {
                            for candidate in BlockKind::ALL {
                                ui.selectable_value(
                                    &mut kind,
                                    candidate,
                                    if candidate == BlockKind::Text {
                                        "Paragraph"
                                    } else {
                                        candidate.label()
                                    },
                                );
                            }
                        });
                    if kind != self.blocks[index].kind {
                        self.apply_command(index, kind);
                    }
                    let old_font = self.style.font.clone();
                    egui::ComboBox::from_id_salt("font")
                        .selected_text(&self.style.font)
                        .width(112.0)
                        .show_ui(ui, |ui| {
                            for font in ["Segoe UI", "Arial", "Times New Roman", "Consolas"] {
                                ui.selectable_value(&mut self.style.font, font.to_owned(), font);
                            }
                        });
                    if old_font != self.style.font {
                        self.dirty = true;
                        self.force_history = true;
                    }
                    if ui
                        .add(
                            egui::DragValue::new(&mut self.style.font_size)
                                .range(8.0..=36.0)
                                .speed(0.5)
                                .suffix(" pt"),
                        )
                        .on_hover_text("Document font size")
                        .changed()
                    {
                        self.dirty = true;
                    }
                    ui.separator();
                    let block = &self.blocks[index];
                    let character = self
                        .selection
                        .filter(|(id, _, _)| *id == block.id)
                        .map(|(_, start, _)| start)
                        .unwrap_or(0);
                    let style = block
                        .character_styles()
                        .get(character)
                        .copied()
                        .unwrap_or(block.format);
                    for (label, decoration, active, hint) in [
                        ("B", Decoration::Bold, style.bold, "Bold · Ctrl+B"),
                        ("I", Decoration::Italic, style.italic, "Italic · Ctrl+I"),
                        (
                            "U",
                            Decoration::Underline,
                            style.underline,
                            "Underline · Ctrl+U",
                        ),
                    ] {
                        let text = match decoration {
                            Decoration::Bold => RichText::new(label).strong(),
                            Decoration::Italic => RichText::new(label).italics(),
                            Decoration::Underline => RichText::new(label).underline(),
                        };
                        if ui
                            .add_sized([30.0, 28.0], egui::Button::new(text).selected(active))
                            .on_hover_text(hint)
                            .clicked()
                        {
                            self.toggle_decoration(decoration, ctx);
                        }
                    }
                    ui.separator();
                    ui.menu_button("Spacing", |ui| {
                        for (label, spacing) in [
                            ("Single", 1.0),
                            ("1.15", 1.15),
                            ("1.5", 1.5),
                            ("Double", 2.0),
                        ] {
                            if ui
                                .selectable_label(
                                    (self.style.line_spacing - spacing).abs() < 0.01,
                                    label,
                                )
                                .clicked()
                            {
                                self.style.line_spacing = spacing;
                                self.dirty = true;
                                self.force_history = true;
                                ui.close_menu();
                            }
                        }
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .selectable_label(self.show_inspector, "Page setup")
                            .clicked()
                        {
                            self.show_inspector = !self.show_inspector;
                            self.show_citations = false;
                        }
                        if ui
                            .selectable_label(self.show_citations, "Citations")
                            .clicked()
                        {
                            self.show_citations = !self.show_citations;
                            self.show_inspector = false;
                        }
                        ui.menu_button(self.style.paper_format.label(), |ui| {
                            for format in
                                [PaperFormat::Freeform, PaperFormat::Apa, PaperFormat::Mla]
                            {
                                if ui
                                    .selectable_label(
                                        self.style.paper_format == format,
                                        format.label(),
                                    )
                                    .clicked()
                                {
                                    self.style.apply_preset(format);
                                    self.dirty = true;
                                    self.force_history = true;
                                    self.show_inspector = format != PaperFormat::Freeform;
                                    self.show_citations = false;
                                    ui.close_menu();
                                }
                            }
                        });
                    });
                });
            });
    }

    fn inspector(&mut self, ctx: &egui::Context) {
        if !self.show_inspector {
            return;
        }
        egui::SidePanel::right("page_setup")
            .exact_width(264.0)
            .resizable(false)
            .frame(Frame::none().fill(PANEL).inner_margin(Margin::same(20.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Page setup").size(15.0).strong());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("×").clicked() {
                            self.show_inspector = false;
                        }
                    });
                });
                ui.add_space(20.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(RichText::new("Document format").size(12.0).color(MUTED));
                    ui.add_space(5.0);
                    for format in [PaperFormat::Freeform, PaperFormat::Apa, PaperFormat::Mla] {
                        if ui
                            .selectable_label(self.style.paper_format == format, format.label())
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
                        ui.label(RichText::new(label).size(12.0).color(MUTED));
                        if ui
                            .add(
                                egui::DragValue::new(value)
                                    .range(range)
                                    .speed(0.05)
                                    .suffix(suffix),
                            )
                            .changed()
                        {
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
                            ui.label(RichText::new(label).size(12.0).color(MUTED));
                            if ui
                                .add(
                                    TextEdit::singleline(value)
                                        .hint_text(label)
                                        .desired_width(220.0)
                                        .margin(Margin::same(7.0)),
                                )
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

    fn find_bar(&mut self, ctx: &egui::Context) {
        if !self.find_open {
            return;
        }
        egui::TopBottomPanel::top("find_bar")
            .exact_height(44.0)
            .frame(
                Frame::none()
                    .fill(PANEL)
                    .inner_margin(Margin::symmetric(22.0, 7.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Find");
                    let response = ui.add(
                        TextEdit::singleline(&mut self.find_query)
                            .id(egui::Id::new("find_query"))
                            .hint_text("Find in this document…")
                            .desired_width(280.0),
                    );
                    let query = self.find_query.to_lowercase();
                    let matches: Vec<_> = self
                        .blocks
                        .iter()
                        .filter(|b| !query.is_empty() && b.text.to_lowercase().contains(&query))
                        .map(|b| b.id)
                        .collect();
                    ui.label(
                        RichText::new(format!("{} paragraphs", matches.len()))
                            .size(12.0)
                            .color(MUTED),
                    );
                    if ui
                        .add_enabled(!matches.is_empty(), egui::Button::new("Next"))
                        .clicked()
                        || (response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        let next = self
                            .scroll_target
                            .and_then(|id| matches.iter().position(|item| *item == id))
                            .map(|i| (i + 1) % matches.len().max(1))
                            .unwrap_or(0);
                        self.scroll_target = matches.get(next).copied();
                    }
                    if ui.small_button("×").clicked() {
                        self.find_open = false;
                    }
                });
            });
    }

    fn status_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(30.0)
            .frame(
                Frame::none()
                    .fill(CANVAS)
                    .inner_margin(Margin::symmetric(22.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let words = self
                        .blocks
                        .iter()
                        .map(|b| b.text.split_whitespace().count())
                        .sum::<usize>();
                    ui.label(
                        RichText::new(format!("{words} words"))
                            .size(11.0)
                            .color(MUTED),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(self.style.paper_format.label())
                            .size(11.0)
                            .color(MUTED),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(
                                "/ insert  ·  Enter paragraph  ·  Shift+Enter line break",
                            )
                            .size(11.0)
                            .color(MUTED),
                        );
                    });
                });
            });
    }

    fn canvas(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(Frame::none().fill(CANVAS))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt(("document_scroll", &self.active_document))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let width = (ui.available_width() - 64.0).clamp(260.0, 780.0);
                        let leading = ((ui.available_width() - width) * 0.5).max(16.0);
                        ui.add_space(54.0);
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
            TextEdit::singleline(&mut self.title)
                .id(egui::Id::new(("document_title", &self.active_document)))
                .font(if academic {
                    document_font(&self.style, InlineStyle::default(), 16.0)
                } else {
                    FontId::new(36.0, FontFamily::Name("Segoe UI Bold".into()))
                })
                .horizontal_align(if academic { Align::Center } else { Align::LEFT })
                .frame(false)
                .desired_width(width)
                .margin(Margin::symmetric(30.0, 7.0))
                .hint_text("Untitled"),
        );
        if title.changed() {
            self.dirty = true;
        }
        ui.add_space(if academic { 20.0 } else { 28.0 });
        let mut action = None;
        let mut number = 0usize;
        for index in 0..self.blocks.len() {
            let id = self.blocks[index].id;
            let kind = self.blocks[index].kind;
            if kind == BlockKind::Numbered {
                number += 1;
            } else {
                number = 0;
            }
            let row = ui.horizontal_top(|ui| {
                let (handle_rect, handle) =
                    ui.allocate_exact_size(Vec2::new(24.0, 28.0), Sense::click());
                handle.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Block options")
                });
                if handle.clicked() {
                    self.block_picker_target = Some(index);
                    self.command_position = handle_rect.left_bottom() + Vec2::new(0.0, 4.0);
                }
                let response = if kind == BlockKind::Divider {
                    ui.add(egui::Separator::default().horizontal().spacing(24.0))
                } else {
                    if kind == BlockKind::Todo {
                        if ui.checkbox(&mut self.blocks[index].checked, "").changed() {
                            self.dirty = true;
                        }
                    }
                    if kind == BlockKind::Bulleted {
                        ui.label(RichText::new("•").size(19.0).color(INK));
                    }
                    if kind == BlockKind::Numbered {
                        ui.label(RichText::new(format!("{number}.")).size(16.0).color(MUTED));
                    }
                    if kind == BlockKind::Quote {
                        ui.label(RichText::new("│").size(22.0).color(MUTED));
                    }
                    let body_width = ui.available_width().min(width - 30.0);
                    self.block_editor(ui, ctx, index, body_width)
                };
                response.context_menu(|ui| {
                    if ui.button("Duplicate").clicked() {
                        action = Some(BlockAction::Duplicate(index));
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(index > 0, egui::Button::new("Move up"))
                        .clicked()
                    {
                        action = Some(BlockAction::Up(index));
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            index + 1 < self.blocks.len(),
                            egui::Button::new("Move down"),
                        )
                        .clicked()
                    {
                        action = Some(BlockAction::Down(index));
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Delete").clicked() {
                        action = Some(BlockAction::Delete(index));
                        ui.close_menu();
                    }
                });
                if response.has_focus()
                    && kind != BlockKind::Code
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
                if response.has_focus()
                    && self.blocks[index].text.is_empty()
                    && index > 0
                    && ui.input(|i| i.key_pressed(egui::Key::Backspace))
                {
                    action = Some(BlockAction::Delete(index));
                }
                (handle_rect, response.rect)
            });
            if row.response.hovered() || self.focused_block == Some(id) {
                for x in [-3.0, 3.0] {
                    for y in [-5.0, 0.0, 5.0] {
                        ui.painter().circle_filled(
                            row.inner.0.center() + Vec2::new(x, y),
                            1.1,
                            MUTED,
                        );
                    }
                }
            }
            let links = wikilinks::links(&self.blocks[index].text);
            if !links.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(30.0);
                    for link in links {
                        let exists = self.workspace.by_title(&link.title).is_some();
                        let label = if exists {
                            format!("Open {}", link.title)
                        } else {
                            format!("{} (not found)", link.title)
                        };
                        if ui.link(label).clicked() {
                            self.link_request = Some(link.title);
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
        let response = ui.add_sized(
            [width, 64.0],
            egui::Button::new(
                RichText::new("Click to continue writing…")
                    .size(13.0)
                    .color(MUTED),
            )
            .frame(false),
        );
        if response.clicked() {
            let last = self.blocks.len() - 1;
            if self.blocks[last].text.is_empty() {
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

    fn block_editor(
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
        let query = if self.find_open {
            self.find_query.to_lowercase()
        } else {
            String::new()
        };
        let size = match kind {
            BlockKind::Heading1 => 28.0,
            BlockKind::Heading2 => 22.0,
            BlockKind::Code => 16.0,
            _ => style.font_size * 4.0 / 3.0,
        };
        let mut layouter = |ui: &egui::Ui, text: &str, wrap_width: f32| {
            let mut job = LayoutJob::default();
            job.wrap.max_width = wrap_width;
            let mut default = base;
            if matches!(kind, BlockKind::Heading1 | BlockKind::Heading2) {
                default.bold = true;
            }
            let mut positions: Vec<_> = text.char_indices().map(|(byte, _)| byte).collect();
            positions.push(text.len());
            for (i, character) in text.chars().enumerate() {
                let mut character_style = marks
                    .iter()
                    .find(|span| i >= span.start && i < span.end)
                    .map(|span| span.style)
                    .unwrap_or(default);
                if matches!(kind, BlockKind::Heading1 | BlockKind::Heading2) {
                    character_style.bold = true;
                }
                let mut format = TextFormat {
                    font_id: if kind == BlockKind::Code {
                        FontId::monospace(size)
                    } else {
                        document_font(&style, character_style, size)
                    },
                    color: INK,
                    italics: kind == BlockKind::Code && character_style.italic,
                    underline: if character_style.underline {
                        Stroke::new(1.0_f32, INK)
                    } else {
                        Stroke::NONE
                    },
                    line_height: Some(size * style.line_spacing),
                    ..Default::default()
                };
                if kind == BlockKind::Code {
                    format.background = Color32::from_rgb(33, 35, 39);
                }
                if !query.is_empty() && text.to_lowercase().contains(&query) {
                    format.background = Color32::from_rgb(66, 61, 43);
                }
                job.append(
                    &character.to_string(),
                    if i == 0
                        && style.paper_format != PaperFormat::Freeform
                        && kind == BlockKind::Text
                    {
                        style.first_line_indent_inches * 96.0
                    } else {
                        0.0
                    },
                    format,
                );
            }
            ui.fonts(|fonts| fonts.layout_job(job))
        };
        let output = TextEdit::multiline(&mut self.blocks[index].text)
            .id(widget_id)
            .frame(false)
            .desired_width(width)
            .desired_rows(1)
            .font(document_font(&style, base, size))
            .layouter(&mut layouter)
            .margin(Margin::symmetric(3.0, 4.0))
            .hint_text(if kind == BlockKind::Code {
                "Write code…"
            } else {
                "Write something, or type '/' to insert…"
            })
            .return_key(Some(egui::KeyboardShortcut::new(
                if kind == BlockKind::Code {
                    egui::Modifiers::NONE
                } else {
                    egui::Modifiers::SHIFT
                },
                egui::Key::Enter,
            )))
            .show(ui);
        if output.response.has_focus() {
            self.focused_block = Some(id);
            if let Some(range) = output.state.cursor.char_range() {
                let [a, b] = range.sorted();
                self.selection = Some((id, a.index, b.index));
            }
        }
        if output.response.changed() {
            self.blocks[index].rebase_formatting(&old_text, &old_styles);
            self.dirty = true;
            self.dismissed_command = None;
        }
        if output.response.has_focus()
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
                    self.blocks[index].kind = BlockKind::Text;
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
                if index > 0 {
                    self.blocks.swap(index, index - 1);
                }
            }
            BlockAction::Down(index) => {
                if index + 1 < self.blocks.len() {
                    self.blocks.swap(index, index + 1);
                }
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
                        .fill(PANEL)
                        .rounding(8.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.set_width(290.0);
                            ui.label(RichText::new("Insert a block").size(11.0).color(MUTED));
                            ui.add_space(6.0);
                            egui::ScrollArea::vertical()
                                .max_height(204.0)
                                .show(ui, |ui| {
                                    if commands.is_empty() {
                                        ui.label(RichText::new("No matching blocks").color(MUTED));
                                    }
                                    for (i, kind) in commands.iter().enumerate() {
                                        let response = ui.add_sized(
                                            [286.0, 32.0],
                                            egui::Button::new(
                                                RichText::new(format!(
                                                    "{}   {}",
                                                    kind.icon(),
                                                    kind.label()
                                                ))
                                                .size(13.0),
                                            )
                                            .selected(i == self.command_index),
                                        );
                                        if response.clicked() {
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
                                    .color(MUTED),
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
                        ui.menu_button("Turn into", |ui| {
                            for kind in BlockKind::ALL {
                                if ui.button(kind.label()).clicked() {
                                    new_kind = Some(kind);
                                    ui.close_menu();
                                }
                            }
                        });
                        if ui.button("Duplicate").clicked() {
                            action = Some(BlockAction::Duplicate(index));
                        }
                        if ui
                            .add_enabled(index > 0, egui::Button::new("Move up"))
                            .clicked()
                        {
                            action = Some(BlockAction::Up(index));
                        }
                        if ui
                            .add_enabled(
                                index + 1 < self.blocks.len(),
                                egui::Button::new("Move down"),
                            )
                            .clicked()
                        {
                            action = Some(BlockAction::Down(index));
                        }
                        ui.separator();
                        if ui.button("Delete").clicked() {
                            action = Some(BlockAction::Delete(index));
                        }
                        if ui.small_button("Close").clicked() {
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
                        save = ui.button("Save and continue").clicked();
                        discard = ui.button("Discard changes").clicked();
                        cancel = ui.button("Cancel").clicked();
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
        if let Some(message) = self.notice.clone() {
            egui::Area::new(egui::Id::new("notice"))
                .anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0.0, -42.0))
                .show(ctx, |ui| {
                    Frame::popup(ui.style())
                        .inner_margin(Margin::symmetric(14.0, 10.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(message).size(12.0));
                                if ui.small_button("×").clicked() {
                                    self.notice = None;
                                }
                            });
                        });
                });
        }
    }
}

impl eframe::App for FlowWriteApp {
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
        let before = self.local_document();
        self.handle_keys(ctx);
        if self.show_home {
            self.home_header(ctx);
            self.home_page(ctx);
        } else {
            self.header(ctx);
            if !self.show_home {
                self.document_tabs(ctx);
                self.toolbar(ctx);
                self.find_bar(ctx);
                self.status_bar(ctx);
                self.library_sidebar(ctx);
                self.citations_panel(ctx);
                self.inspector(ctx);
                self.canvas(ctx);
            } else {
                ctx.request_repaint();
            }
        }
        self.popups(ctx);
        if self.dirty && !self.history_navigation && before != self.local_document() {
            let now = ctx.input(|i| i.time);
            if self.force_history
                || now - self.last_history_time > 0.75
                || self.undo_stack.is_empty()
            {
                self.undo_stack.push(before.clone());
                if self.undo_stack.len() > 100 {
                    self.undo_stack.remove(0);
                }
            }
            self.last_history_time = now;
            self.redo_stack.clear();
        }
        let now = ctx.input(|i| i.time);
        if self.dirty && before != self.local_document() {
            self.last_document_change = now;
        }
        if let Some(title) = self.link_request.take() {
            self.follow_document_link(title);
        }
        if self.dirty && !self.active_document.is_empty() {
            if now - self.last_document_change >= 0.8 {
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
    let base = match style.font.as_str() {
        "Times New Roman" => "Times New Roman",
        "Consolas" | "JetBrains Mono" => "Consolas",
        "Arial" => "Arial",
        _ => "Segoe UI",
    };
    let name = match (format.bold, format.italic) {
        (true, true) => format!("{base} Bold Italic"),
        (true, false) => format!("{base} Bold"),
        (false, true) => format!("{base} Italic"),
        _ => base.to_owned(),
    };
    FontId::new(size, FontFamily::Name(name.into()))
}

pub(super) fn configure_fonts(ctx: &egui::Context) {
    let mut definitions = egui::FontDefinitions::default();
    for (family, file) in [
        ("Segoe UI", "segoeui.ttf"),
        ("Segoe UI Bold", "segoeuib.ttf"),
        ("Arial", "arial.ttf"),
        ("Arial Bold", "arialbd.ttf"),
        ("Times New Roman", "times.ttf"),
        ("Times New Roman Bold", "timesbd.ttf"),
        ("Consolas", "consola.ttf"),
        ("Consolas Bold", "consolab.ttf"),
        ("Segoe UI Italic", "segoeuii.ttf"),
        ("Segoe UI Bold Italic", "segoeuiz.ttf"),
        ("Arial Italic", "ariali.ttf"),
        ("Arial Bold Italic", "arialbi.ttf"),
        ("Times New Roman Italic", "timesi.ttf"),
        ("Times New Roman Bold Italic", "timesbi.ttf"),
        ("Consolas Italic", "consolai.ttf"),
        ("Consolas Bold Italic", "consolaz.ttf"),
    ] {
        let mut fallback = definitions
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default();
        if let Ok(bytes) = std::fs::read(Path::new("C:\\Windows\\Fonts").join(file)) {
            definitions
                .font_data
                .insert(family.to_owned(), egui::FontData::from_owned(bytes));
            fallback.insert(0, family.to_owned());
        }
        definitions
            .families
            .insert(FontFamily::Name(family.into()), fallback);
    }
    if definitions.font_data.contains_key("Segoe UI") {
        definitions
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "Segoe UI".to_owned());
    }
    ctx.set_fonts(definitions);
}

pub(super) fn configure_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = PANEL;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = CANVAS;
    visuals.override_text_color = Some(INK);
    visuals.faint_bg_color = Color32::from_rgb(37, 39, 43);
    visuals.selection.bg_fill = Color32::from_rgb(62, 78, 93);
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.bg_fill = Color32::from_rgb(43, 45, 50);
        widget.weak_bg_fill = Color32::from_rgb(37, 39, 43);
        widget.bg_stroke = Stroke::new(1.0_f32, PANEL_STROKE);
        widget.rounding = Rounding::same(5.0);
    }
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(56, 59, 65);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(45, 47, 52);
    visuals.window_rounding = Rounding::same(9.0);
    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style.spacing.item_spacing = Vec2::new(6.0, 6.0);
        style.spacing.button_padding = Vec2::new(10.0, 6.0);
        style.spacing.interact_size.y = 28.0;
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(13.0));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitting_in_the_middle_preserves_unicode_and_formatting() {
        let mut app = FlowWriteApp::empty();
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
        let mut app = FlowWriteApp::empty();
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
        configure_visuals(&ctx);
        let mut app = FlowWriteApp::empty();
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
