//! The same compact application header is used on every workspace surface.
use super::*;

impl FlowriteApp {
    pub(super) fn header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("app_header")
            .exact_height(48.0)
            .frame(
                Frame::none()
                    .fill(theme::current().canvas)
                    .inner_margin(Margin::symmetric(16.0, 8.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    self.brand_button(ui);
                    if icons::control(
                        ui,
                        Icon::Sidebar,
                        "Workspace sidebar",
                        "Show or hide workspace navigation",
                        true,
                        self.show_sidebar,
                    )
                    .clicked()
                    {
                        self.show_sidebar = !self.show_sidebar;
                    }
                    icons::menu(ui, Icon::Document, "", |ui| self.file_menu(ui, ctx));
                    let location = if self.settings_open {
                        "Settings".to_owned()
                    } else if self.show_home {
                        self.current_folder
                            .as_ref()
                            .and_then(|id| self.workspace.folders.iter().find(|f| &f.id == id))
                            .map(|f| format!("Home / {}", f.name))
                            .unwrap_or_else(|| "Home".into())
                    } else {
                        self.title.clone()
                    };
                    let location_width = (ui.available_width() - 188.0).max(40.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(location_width, 32.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.add(
                                egui::Label::new(RichText::new(&location).size(14.0)).truncate(),
                            )
                            .on_hover_text(&location);
                        },
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        icons::menu(ui, Icon::More, "", |ui| {
                            if icons::labelled(ui, Icon::Settings, "Settings · Ctrl+,").clicked() {
                                self.settings_open = true;
                                ui.close_menu();
                            }
                            ui.separator();
                            if icons::labelled(ui, Icon::PageSetup, "Expand writing · F11")
                                .clicked()
                            {
                                self.focus_mode = !self.focus_mode;
                                ui.close_menu();
                            }
                            if icons::labelled(ui, Icon::Info, "Writing help").clicked() {
                                self.show_help(ctx);
                                ui.close_menu();
                            }
                            if !self.show_home {
                                ui.separator();
                                self.document_menu(ui);
                            }
                        })
                        .response
                        .on_hover_text("Workspace and document options");
                        if icons::control(
                            ui,
                            Icon::Search,
                            "Search",
                            if self.show_home {
                                "Search pages and actions · Ctrl+K"
                            } else {
                                "Find in document · Ctrl+F"
                            },
                            true,
                            self.find_open,
                        )
                        .clicked()
                        {
                            if self.show_home {
                                self.palette.open = true;
                                self.palette.focus = true;
                            } else {
                                self.find_open = !self.find_open;
                                if self.find_open {
                                    ctx.memory_mut(|m| {
                                        m.request_focus(egui::Id::new("find_query"))
                                    });
                                }
                            }
                        }
                        if icons::control(
                            ui,
                            Icon::Text,
                            "Formatting",
                            "Show or hide formatting controls",
                            !self.show_home && !self.settings_open,
                            self.settings.editor.toolbar,
                        )
                        .clicked()
                        {
                            self.settings.editor.toolbar = !self.settings.editor.toolbar;
                            let _ = self.settings.save(&self.workspace.directory);
                        }
                        ui.add_enabled_ui(!self.show_home && !self.settings_open, |ui| {
                            icons::menu(ui, Icon::Export, "", |ui| {
                                if icons::labelled(ui, Icon::Preview, "Paper preview…").clicked()
                                {
                                    self.paper_preview();
                                    ui.close_menu();
                                }
                                if icons::labelled(ui, Icon::Export, "Export PDF…").clicked() {
                                    self.export(ExportTarget::Pdf);
                                    ui.close_menu();
                                }
                                if icons::labelled(ui, Icon::Export, "Export Markdown…").clicked()
                                {
                                    self.export(ExportTarget::Markdown);
                                    ui.close_menu();
                                }
                            });
                        });
                    });
                });
            });
    }

    fn file_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if icons::labelled(ui, Icon::Plus, "New page    Ctrl+N").clicked() {
            self.request_action(PendingAction::New, ctx);
            ui.close_menu();
        }
        if icons::labelled(ui, Icon::Folder, "New folder    Ctrl+Shift+N").clicked() {
            self.begin_folder();
            ui.close_menu();
        }
        if icons::labelled(ui, Icon::Open, "Open…    Ctrl+O").clicked() {
            self.request_action(PendingAction::Open, ctx);
            ui.close_menu();
        }
        ui.separator();
        if icons::labelled(ui, Icon::Save, "Save    Ctrl+S").clicked() {
            self.save_document(false);
            ui.close_menu();
        }
        if icons::labelled(ui, Icon::Save, "Save a copy…    Ctrl+Shift+S").clicked() {
            self.save_document(true);
            ui.close_menu();
        }
        ui.separator();
        if icons::labelled_enabled(
            ui,
            Icon::Undo,
            "Undo    Ctrl+Z",
            !self.undo_stack.is_empty(),
        )
        .clicked()
        {
            self.undo();
            ui.close_menu();
        }
        if icons::labelled_enabled(
            ui,
            Icon::Redo,
            "Redo    Ctrl+Shift+Z",
            !self.redo_stack.is_empty(),
        )
        .clicked()
        {
            self.redo();
            ui.close_menu();
        }
        if self.settings.saving.indicator && !self.show_home {
            ui.separator();
            ui.label(
                RichText::new(if self.dirty {
                    "Saving changes…"
                } else if self.active_document.is_empty() {
                    "Temporary draft · saved after your first edit"
                } else {
                    "All changes saved"
                })
                .color(theme::current().muted),
            );
        }
    }

    fn document_menu(&mut self, ui: &mut egui::Ui) {
        ui.label("Document format");
        for format in [PaperFormat::Freeform, PaperFormat::Mla, PaperFormat::Apa] {
            if ui
                .selectable_label(self.style.paper_format == format, format.label())
                .hand()
                .clicked()
            {
                self.style.apply_preset(format);
                self.dirty = true;
                self.force_history = true;
                ui.close_menu();
            }
        }
        ui.separator();
        if icons::labelled(ui, Icon::Citations, self.citations_heading()).clicked() {
            self.show_citations = !self.show_citations;
            self.show_inspector = false;
            ui.close_menu();
        }
        if icons::labelled(ui, Icon::PageSetup, "Page setup").clicked() {
            self.show_inspector = !self.show_inspector;
            self.show_citations = false;
            ui.close_menu();
        }
        if icons::labelled(ui, Icon::Info, "Review spelling & grammar").clicked() {
            self.proofing_open = true;
            ui.close_menu();
        }
    }

    fn show_help(&self, ctx: &egui::Context) {
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("writing_help"), true));
    }

    pub(super) fn writing_help(&self, ctx: &egui::Context) {
        let mut open = ctx
            .data(|d| d.get_temp::<bool>(egui::Id::new("writing_help")))
            .unwrap_or(false);
        if open {
            egui::Window::new("Writing help")
                .open(&mut open)
                .collapsible(false)
                .default_width(360.0)
                .show(ctx, |ui| {
                    for (keys, action) in [
                        ("/", "Insert a block at a paragraph's start"),
                        ("Enter / Shift+Enter", "New paragraph / line break"),
                        ("Tab / Shift+Tab", "Indent / outdent code and lists"),
                        ("Arrow keys", "Move through text and between blocks"),
                        ("Ctrl+B / I / U", "Bold / italic / underline"),
                        ("Ctrl+K", "Search pages and actions"),
                        ("F7", "Review spelling and grammar"),
                        ("F11", "Expand writing / restore workspace"),
                    ] {
                        ui.label(RichText::new(keys).strong());
                        ui.label(action);
                        ui.add_space(8.0);
                    }
                });
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("writing_help"), open));
        }
    }
}
