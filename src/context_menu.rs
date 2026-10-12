use super::*;

#[derive(Clone)]
pub(super) enum PageAction {
    Open(String),
    Rename(String),
    Duplicate(String),
    Move(String, Option<String>),
    NewFolder(String),
    Export(String, ExportTarget),
    Delete(String),
    OpenFolder(String),
    RenameFolder(String),
    DeleteFolder(String),
    Cleanup,
    ResetSettings,
}
#[derive(Default)]
pub(super) struct Dialogs {
    pub action: Option<PageAction>,
    pub rename: Option<(bool, String, String)>,
    pub rename_focus: bool,
    pub error: Option<String>,
    pub delete: Option<PageAction>,
    pub cancel_focus: bool,
    pub delete_pages: bool,
    pub move_after_create: Option<String>,
}

pub(super) fn page_menu(ui: &mut egui::Ui, id: &str, workspace: &Workspace) -> Option<PageAction> {
    let mut action = None;
    for label in ["Open", "Open in new tab"] {
        if icons::labelled(ui, Icon::Open, label).clicked() {
            action = Some(PageAction::Open(id.into()));
            ui.close_menu();
        }
    }
    if icons::labelled(ui, Icon::Rename, "Rename").clicked() {
        action = Some(PageAction::Rename(id.into()));
        ui.close_menu();
    }
    if icons::labelled(ui, Icon::Copy, "Duplicate").clicked() {
        action = Some(PageAction::Duplicate(id.into()));
        ui.close_menu();
    }
    icons::menu(ui, Icon::Folder, "Move to folder", |ui| {
        if ui.button("No folder").hand().clicked() {
            action = Some(PageAction::Move(id.into(), None));
            ui.close_menu();
        }
        for folder in &workspace.folders {
            if ui.button(&folder.name).hand().clicked() {
                action = Some(PageAction::Move(id.into(), Some(folder.id.clone())));
                ui.close_menu();
            }
        }
        ui.separator();
        if ui.button("New folder…").hand().clicked() {
            action = Some(PageAction::NewFolder(id.into()));
            ui.close_menu();
        }
    })
    .response
    .hand();
    icons::menu(ui, Icon::Export, "Export", |ui| {
        for (label, target) in [
            ("PDF", ExportTarget::Pdf),
            ("Markdown", ExportTarget::Markdown),
        ] {
            if icons::labelled(ui, Icon::Export, label).clicked() {
                action = Some(PageAction::Export(id.into(), target));
                ui.close_menu();
            }
        }
    })
    .response
    .hand();
    ui.separator();
    if icons::danger(ui, "Delete").clicked() {
        action = Some(PageAction::Delete(id.into()));
        ui.close_menu();
    }
    action
}
pub(super) fn folder_menu(ui: &mut egui::Ui, id: &str) -> Option<PageAction> {
    let mut action = None;
    if icons::labelled(ui, Icon::Open, "Open").clicked() {
        action = Some(PageAction::OpenFolder(id.into()));
        ui.close_menu();
    }
    if icons::labelled(ui, Icon::Rename, "Rename").clicked() {
        action = Some(PageAction::RenameFolder(id.into()));
        ui.close_menu();
    }
    ui.separator();
    if icons::danger(ui, "Delete").clicked() {
        action = Some(PageAction::DeleteFolder(id.into()));
        ui.close_menu();
    }
    action
}

impl FlowriteApp {
    pub(super) fn begin_folder(&mut self) {
        self.folder_dialog = true;
        self.folder_name.clear();
        self.folder_error = None;
        self.folder_focus = true;
    }
    pub(super) fn library_dialogs(&mut self, ctx: &egui::Context) {
        if let Some(action) = self.dialogs.action.take() {
            match action {
                PageAction::Open(id) => self.switch_document(&id),
                PageAction::OpenFolder(id) => {
                    self.go_home();
                    self.current_folder = Some(id);
                }
                PageAction::Rename(id) => {
                    if let Some(doc) = self.workspace.documents.iter().find(|d| d.id == id) {
                        self.dialogs.rename = Some((false, id, doc.document.title.clone()));
                        self.dialogs.rename_focus = true;
                        self.dialogs.error = None;
                    }
                }
                PageAction::RenameFolder(id) => {
                    if let Some(f) = self.workspace.folders.iter().find(|f| f.id == id) {
                        self.dialogs.rename = Some((true, id, f.name.clone()));
                        self.dialogs.rename_focus = true;
                        self.dialogs.error = None;
                    }
                }
                PageAction::Duplicate(id) => {
                    if self.save_workspace() {
                        match self.workspace.duplicate_document(&id) {
                            Ok(new) => self.switch_document(&new),
                            Err(e) => self.notice = Some(e),
                        }
                    }
                }
                PageAction::Move(id, folder) => {
                    if self.save_workspace() {
                        self.workspace.move_document(&id, folder);
                        if let Err(e) = self.workspace.save() {
                            self.notice = Some(e);
                        }
                    }
                }
                PageAction::NewFolder(id) => {
                    self.dialogs.move_after_create = Some(id);
                    self.begin_folder();
                }
                PageAction::Export(id, target) => {
                    self.switch_document(&id);
                    self.export(target);
                }
                other => {
                    self.dialogs.delete = Some(other);
                    self.dialogs.cancel_focus = true;
                    self.dialogs.delete_pages = false;
                }
            }
        }
        if let Some((folder, id, mut name)) = self.dialogs.rename.clone() {
            let mut save = false;
            let mut cancel = false;
            egui::Window::new(if folder {
                "Rename folder"
            } else {
                "Rename page"
            })
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                let edit = ui.add(TextEdit::singleline(&mut name).desired_width(320.0));
                if self.dialogs.rename_focus {
                    edit.request_focus();
                    self.dialogs.rename_focus = false;
                }
                if name.contains(['/', '\\']) {
                    ui.label("Slashes will become hyphens in export filenames.");
                }
                if let Some(error) = &self.dialogs.error {
                    ui.colored_label(Color32::from_rgb(235, 112, 112), error);
                }
                ui.horizontal(|ui| {
                    save = icons::labelled(ui, Icon::Rename, "Rename").clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Enter));
                    cancel = ui.button("Cancel").hand().clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Escape));
                });
            });
            self.dialogs.rename = Some((folder, id.clone(), name.clone()));
            if save {
                let result = if folder {
                    self.workspace.rename_folder(&id, &name)
                } else {
                    self.workspace.rename_document(&id, &name)
                };
                match result {
                    Ok(()) => {
                        for block in &mut self.blocks {
                            let before = block.text.clone();
                            wikilinks::display(block, &self.workspace);
                            if block.text != before {
                                self.dirty = true;
                            }
                        }
                        if id == self.active_document {
                            self.title = name.trim().into();
                            self.dirty = true;
                        }
                        let _ = self.workspace.save();
                        self.dialogs.rename = None;
                    }
                    Err(e) => self.dialogs.error = Some(e),
                }
            }
            if cancel {
                self.dialogs.rename = None;
            }
        }
        if let Some(action) = self.dialogs.delete.clone() {
            let (title, description) = match &action {
                PageAction::Delete(id) => {
                    let doc = self.workspace.documents.iter().find(|d| &d.id == id);
                    (
                        format!(
                            "Delete “{}”?",
                            doc.map(|d| d.document.title.as_str()).unwrap_or("page")
                        ),
                        if doc.is_some_and(|d| d.external_path.is_some()) {
                            "Only the library entry is removed. Your original file on disk is kept."
                        } else {
                            "The page will be removed from your library."
                        },
                    )
                }
                PageAction::DeleteFolder(id) => (
                    format!(
                        "Delete “{}”?",
                        self.workspace
                            .folders
                            .iter()
                            .find(|f| &f.id == id)
                            .map(|f| f.name.as_str())
                            .unwrap_or("folder")
                    ),
                    "Choose what happens to the pages in this folder.",
                ),
                PageAction::Cleanup => (
                    "Delete all empty pages?".into(),
                    "Only untitled pages with no text or citations are removed.",
                ),
                _ => (
                    "Reset all settings?".into(),
                    "Your documents will not be changed.",
                ),
            };
            let mut confirm = false;
            let mut cancel = ctx.input(|i| i.key_pressed(egui::Key::Escape));
            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .default_width(390.0)
                .show(ctx, |ui| {
                    ui.label(description);
                    ui.add_space(12.0);
                    if let PageAction::DeleteFolder(id) = &action {
                        let count = self
                            .workspace
                            .documents
                            .iter()
                            .filter(|d| d.folder.as_ref() == Some(id))
                            .count();
                        ui.radio_value(
                            &mut self.dialogs.delete_pages,
                            false,
                            "Keep pages (move to no folder)",
                        )
                        .hand();
                        ui.radio_value(
                            &mut self.dialogs.delete_pages,
                            true,
                            RichText::new(format!("Delete folder and its {count} pages"))
                                .color(Color32::from_rgb(235, 112, 112)),
                        )
                        .hand();
                    }
                    ui.horizontal(|ui| {
                        let response = ui.button("Cancel").hand();
                        if self.dialogs.cancel_focus {
                            response.request_focus();
                            self.dialogs.cancel_focus = false;
                        }
                        cancel |= response.clicked();
                        // Enter must never authorize a destructive action, even if
                        // keyboard focus was moved onto the red button.
                        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        confirm = icons::danger(
                            ui,
                            if matches!(action, PageAction::ResetSettings) {
                                "Reset"
                            } else {
                                "Delete"
                            },
                        )
                        .clicked()
                            && !enter;
                    });
                });
            if cancel {
                self.dialogs.delete = None;
            }
            if confirm && self.save_workspace() {
                match action {
                    PageAction::Delete(id) => {
                        self.workspace.delete_document(&id);
                    }
                    PageAction::DeleteFolder(id) => {
                        self.workspace.delete_folder(&id, self.dialogs.delete_pages);
                        if self.current_folder.as_ref() == Some(&id) {
                            self.current_folder = None;
                        }
                    }
                    PageAction::Cleanup => {
                        let ids: Vec<_> = self
                            .workspace
                            .documents
                            .iter()
                            .filter(|d| {
                                d.document.title.starts_with("Untitled")
                                    && d.document.style.paper_format == PaperFormat::Freeform
                                    && d.document
                                        .blocks
                                        .iter()
                                        .all(|b| b.text.is_empty() && b.kind == BlockKind::Text)
                                    && d.document.citations.is_empty()
                            })
                            .map(|d| d.id.clone())
                            .collect();
                        for id in ids {
                            self.workspace.delete_document(&id);
                        }
                    }
                    PageAction::ResetSettings => {
                        self.settings = settings::Settings::default();
                        self.apply_settings(ctx);
                    }
                    _ => {}
                }
                if !self
                    .workspace
                    .documents
                    .iter()
                    .any(|d| d.id == self.active_document)
                {
                    if let Some(id) = self.workspace.tabs.last().cloned() {
                        self.switch_document(&id);
                    } else {
                        self.active_document.clear();
                        self.workspace.active = None;
                        self.show_home = true;
                        self.dirty = false;
                    }
                }
                let _ = self.workspace.save();
                self.dialogs.delete = None;
            }
        }
    }
}
