use super::*;
#[derive(Default)]
pub(super) struct Palette {
    pub open: bool,
    pub query: String,
    pub index: usize,
    pub focus: bool,
}
#[derive(Clone)]
enum Action {
    New,
    Folder,
    Home,
    Settings(usize),
    Find,
    Replace,
    Save,
    Export(ExportTarget),
    Page(context_menu::PageAction),
    Check,
    Focus,
}
fn results(workspace: &Workspace, query: &str, active: &str) -> Vec<(String, String, Action)> {
    let mut items: Vec<(String, String, Action)> = vec![
        ("New page".into(), "Ctrl+N".into(), Action::New),
        ("New folder".into(), "Ctrl+Shift+N".into(), Action::Folder),
        ("Go home".into(), "Your library".into(), Action::Home),
        ("Open settings".into(), "Ctrl+,".into(), Action::Settings(0)),
        ("Find in document".into(), "Ctrl+F".into(), Action::Find),
        (
            "Replace in document".into(),
            "Ctrl+H".into(),
            Action::Replace,
        ),
        ("Save document".into(), "Ctrl+S".into(), Action::Save),
        (
            "Export PDF".into(),
            "PDF document".into(),
            Action::Export(ExportTarget::Pdf),
        ),
        (
            "Export Markdown".into(),
            "Markdown document".into(),
            Action::Export(ExportTarget::Markdown),
        ),
        (
            "Check document".into(),
            "Offline spelling and grammar".into(),
            Action::Check,
        ),
        ("Toggle focus mode".into(), "F11".into(), Action::Focus),
    ];
    if !active.is_empty() {
        items.push((
            "Delete current page".into(),
            "Requires confirmation".into(),
            Action::Page(context_menu::PageAction::Delete(active.into())),
        ));
    }
    for (i, section) in [
        "General",
        "Appearance",
        "Editor",
        "Saving",
        "Citations & formatting",
        "Export",
        "Keyboard shortcuts",
        "About & privacy",
        "Writing & proofing",
    ]
    .iter()
    .enumerate()
    {
        items.push((
            format!("Settings: {section}"),
            "Settings section".into(),
            Action::Settings(i),
        ));
    }
    for doc in &workspace.documents {
        items.push((
            doc.document.title.clone(),
            "Document".into(),
            Action::Page(context_menu::PageAction::Open(doc.id.clone())),
        ));
    }
    for folder in &workspace.folders {
        items.push((
            folder.name.clone(),
            "Folder".into(),
            Action::Page(context_menu::PageAction::OpenFolder(folder.id.clone())),
        ));
    }
    let query = query.trim().to_lowercase();
    items.retain(|(name, description, _)| {
        name.to_lowercase().contains(&query) || description.to_lowercase().contains(&query)
    });
    items
}
impl FlowriteApp {
    pub(super) fn command_palette(&mut self, ctx: &egui::Context) {
        if !self.palette.open {
            return;
        }
        let items = results(&self.workspace, &self.palette.query, &self.active_document);
        let (up, down, enter, escape) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
            )
        });
        if escape {
            self.palette.open = false;
            return;
        }
        let count = items.len();
        if count > 0 {
            if up {
                self.palette.index = (self.palette.index + count - 1) % count;
            }
            if down {
                self.palette.index = (self.palette.index + 1) % count;
            }
            self.palette.index = self.palette.index.min(count - 1);
        }
        let mut action = if enter {
            items.get(self.palette.index).map(|(_, _, a)| a.clone())
        } else {
            None
        };
        egui::Window::new("Search & commands")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, Vec2::new(0.0, 100.0))
            .fixed_size(Vec2::new(520.0, 440.0))
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("Search pages, folders, and actions")
                        .small()
                        .color(theme::current().muted),
                );
                let edit = ui.add(
                    TextEdit::singleline(&mut self.palette.query)
                        .hint_text("What would you like to do?")
                        .desired_width(f32::INFINITY)
                        .margin(12.0),
                );
                if self.palette.focus {
                    edit.request_focus();
                    self.palette.focus = false;
                }
                if edit.changed() {
                    self.palette.index = 0;
                }
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(340.0)
                    .show(ui, |ui| {
                        if items.is_empty() {
                            ui.label("No matching pages or commands.");
                        }
                        for (i, (name, description, item)) in items.iter().enumerate() {
                            let response = ui
                                .add_sized(
                                    [ui.available_width(), 34.0],
                                    egui::Button::image_and_text(
                                        {
                                            let icon = match item {
                                                Action::New => Icon::Document,
                                                Action::Folder => Icon::Folder,
                                                Action::Settings(_) => Icon::Settings,
                                                Action::Find | Action::Replace => Icon::Search,
                                                Action::Save => Icon::Save,
                                                Action::Export(_) => Icon::Export,
                                                Action::Check => Icon::Check,
                                                Action::Focus => Icon::PageSetup,
                                                Action::Page(context_menu::PageAction::Delete(
                                                    _,
                                                )) => Icon::Trash,
                                                Action::Page(
                                                    context_menu::PageAction::OpenFolder(_),
                                                ) => Icon::Folder,
                                                _ => Icon::Document,
                                            };
                                            let color = if icon == Icon::Trash {
                                                icons::DANGER
                                            } else {
                                                theme::current().ink
                                            };
                                            icon.image(ui, color)
                                        },
                                        RichText::new(format!("{name}    ·    {description}"))
                                            .color(
                                                if matches!(
                                                    item,
                                                    Action::Page(context_menu::PageAction::Delete(
                                                        _
                                                    ))
                                                ) {
                                                    icons::DANGER
                                                } else {
                                                    theme::current().ink
                                                },
                                            ),
                                    )
                                    .selected(i == self.palette.index),
                                )
                                .hand();
                            if response.clicked() {
                                action = Some(item.clone());
                            }
                            if i == self.palette.index && (up || down) {
                                response.scroll_to_me(Some(Align::Center));
                            }
                        }
                    });
                ui.label(
                    RichText::new("↑ ↓ navigate · Enter choose · Esc close")
                        .small()
                        .color(theme::current().muted),
                );
            });
        if let Some(action) = action {
            self.palette.open = false;
            match action {
                Action::New => self.start_fresh(),
                Action::Folder => self.begin_folder(),
                Action::Home => self.go_home(),
                Action::Settings(section) => {
                    self.settings_open = true;
                    self.settings_section = section;
                }
                Action::Find | Action::Replace => {
                    self.find_open = true;
                    self.search.replace = matches!(action, Action::Replace);
                    ctx.memory_mut(|m| m.request_focus(egui::Id::new("find_query")));
                }
                Action::Save => {
                    self.save_workspace();
                }
                Action::Export(target) => self.export(target),
                Action::Page(action) => self.dialogs.action = Some(action),
                Action::Check => {
                    self.proofreader.invalidate();
                    self.proofreader.manual = true;
                }
                Action::Focus => self.focus_mode = !self.focus_mode,
            }
        }
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn palette_includes_pages_and_confirmation_only_delete() {
        let mut app = crate::FlowriteApp::empty();
        app.title = "Research notes".into();
        let id = app.workspace.add(app.local_document(), None);
        let items = super::results(&app.workspace, "research", &id);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, "Research notes");
        assert_eq!(super::results(&app.workspace, "delete", &id).len(), 1);
    }
}
