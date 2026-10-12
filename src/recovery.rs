use super::*;
#[derive(Default)]
pub(super) struct Recovery {
    pub error: Option<String>,
    pub open: bool,
    confirm: String,
}

/// Preserve only the library manifests. Document files and their assets remain
/// untouched, including any page whose JSON could not be opened.
fn preserve_manifests(directory: &Path) -> Result<(), String> {
    for name in ["index.json", "workspace.json"] {
        let source = directory.join(name);
        if !source.exists() {
            continue;
        }
        let target = directory.join(format!("{name}.{}.corrupt", workspace::timestamp()));
        if target.exists() {
            return Err("Recovery copy already exists; try again.".into());
        }
        std::fs::rename(source, target).map_err(|e| e.to_string())?;
    }
    Ok(())
}
impl FlowriteApp {
    pub(super) fn recovery_dialog(&mut self, ctx: &egui::Context) {
        if !self.recovery.open {
            return;
        }
        let newer = self
            .recovery
            .error
            .as_ref()
            .is_some_and(|e| e.to_lowercase().contains("newer"));
        let mut retry = false;
        let mut fresh = false;
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        egui::Window::new("Your library needs attention").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER,Vec2::ZERO).default_width(480.0).show(ctx,|ui|{
            ui.label(self.recovery.error.as_deref().unwrap_or("Library could not be opened."));
            ui.label("Library autosave is unavailable until this is resolved. You can still save a separate document copy from File > Save as.");
            ui.add_space(12.0);ui.label(self.workspace.directory.display().to_string());
            if ui.button("Open backup folder").hand().clicked(){settings_ui::open_path(&self.workspace.directory);}
            if ui.button("Retry opening library and backup").hand().clicked(){retry=true;}
            if newer {ui.label("This library was created by a newer flowrite version. It will not be changed; use that version to open it.");}else {
                ui.add_space(12.0);ui.label("Start fresh keeps the old manifests as .corrupt files and leaves all existing document and image files untouched. They will not appear in the fresh library until recovered.");
                ui.add(TextEdit::singleline(&mut self.recovery.confirm).hint_text("Type START to start fresh"));
                let enter=ui.input(|i|i.key_pressed(egui::Key::Enter));
                fresh=ui.add_enabled(self.recovery.confirm=="START",egui::Button::new(RichText::new("Start fresh").color(Color32::from_rgb(235,112,112)))).hand().clicked()&&!enter;
            }
            if ui.button("Keep files unchanged").hand().clicked(){close=true;}
        });
        if retry {
            match Workspace::load(self.workspace.directory.clone()) {
                Ok(workspace) => {
                    self.workspace = workspace;
                    self.workspace_available = true;
                    // A draft created while recovery was open may have IDs that
                    // collide with the recovered library. Preserve it as a new
                    // page, never commit it over an existing recovered page.
                    self.active_document.clear();
                    self.current_path = None;
                    if self.dirty {
                        self.save_workspace();
                    } else if let Some(id) = self.workspace.active.clone() {
                        self.load_workspace_document(&id);
                    }
                    self.recovery.open = false;
                    self.recovery.error = None;
                    self.notice = Some("Library opened. Autosave is available again.".into());
                }
                Err(error) => self.recovery.error = Some(error),
            }
        }
        if fresh {
            let directory = self.workspace.directory.clone();
            match preserve_manifests(&directory).and_then(|_| Workspace::load(directory.clone())) {
                Ok(workspace) => {
                    self.workspace = workspace;
                    self.workspace_available = true;
                    self.active_document.clear();
                    self.current_path = None;
                    self.show_home = true;
                    self.recovery = Recovery::default();
                    if self.dirty {
                        self.save_workspace();
                    }
                    self.notice=Some("Started a fresh library. The previous manifests and all document files are preserved in the library folder.".into());
                }
                Err(error) => self.recovery.error = Some(error),
            }
        }
        if close {
            self.recovery.open = false;
        }
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn recovery_preserves_documents_and_broken_manifest() {
        let dir = std::env::temp_dir().join(format!(
            "flowrite-recovery-{}",
            crate::workspace::timestamp()
        ));
        std::fs::create_dir_all(dir.join("documents")).unwrap();
        std::fs::write(dir.join("index.json"), b"broken").unwrap();
        std::fs::write(dir.join("documents/document-1.json"), b"keep this page").unwrap();
        super::preserve_manifests(&dir).unwrap();
        assert!(!dir.join("index.json").exists());
        assert_eq!(
            std::fs::read(dir.join("documents/document-1.json")).unwrap(),
            b"keep this page"
        );
        assert!(std::fs::read_dir(dir)
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e.file_name().to_string_lossy().ends_with(".corrupt")));
    }
}
