use super::document::SavedDocument;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct WorkspaceDocument {
    pub id: String,
    pub document: SavedDocument,
    pub external_path: Option<PathBuf>,
    pub modified: u64,
    #[serde(default)]
    pub folder: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct WorkspaceFolder {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct Workspace {
    pub documents: Vec<WorkspaceDocument>,
    pub tabs: Vec<String>,
    pub active: Option<String>,
    #[serde(default)]
    pub folders: Vec<WorkspaceFolder>,
    #[serde(skip)]
    pub directory: PathBuf,
}

pub(super) fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub(super) fn default_directory() -> PathBuf {
    let executable = std::env::current_exe().unwrap_or_default();
    if executable
        .file_stem()
        .and_then(|p| p.to_str())
        .unwrap_or("")
        .contains("review")
    {
        return executable
            .parent()
            .unwrap_or(Path::new("."))
            .join("FlowWrite-review-data");
    }
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| executable.parent().unwrap_or(Path::new(".")).to_owned())
        .join("FlowWrite")
}

pub(super) fn write_atomic(path: &Path, content: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_file_name(format!(
        "{}.flowwrite-autosave.tmp",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    std::fs::write(&temporary, content)?;
    std::fs::rename(&temporary, path)
}

impl Workspace {
    pub fn load(directory: PathBuf) -> Result<Self, String> {
        let file = directory.join("workspace.json");
        let mut workspace = if file.exists() {
            let content = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
            serde_json::from_str::<Self>(&content).map_err(|e| e.to_string())?
        } else {
            Self::default()
        };
        workspace.directory = directory;
        if workspace.documents.iter().any(|d| d.document.version != 1) {
            return Err("This library was saved by a newer version of FlowWrite".into());
        }
        workspace
            .tabs
            .retain(|id| workspace.documents.iter().any(|doc| &doc.id == id));
        if !workspace
            .active
            .as_ref()
            .map(|id| workspace.tabs.contains(id))
            .unwrap_or(false)
        {
            workspace.active = workspace.tabs.first().cloned();
        }
        Ok(workspace)
    }

    pub fn save(&self) -> Result<(), String> {
        if self.directory.as_os_str().is_empty() {
            return Err(
                "The local library is unavailable. Save a copy using File > Save as.".into(),
            );
        }
        let content = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        write_atomic(&self.directory.join("workspace.json"), &content).map_err(|e| e.to_string())
    }

    pub fn add(&mut self, document: SavedDocument, external_path: Option<PathBuf>) -> String {
        let id = format!("{:x}-{:x}", timestamp(), self.documents.len());
        self.documents.push(WorkspaceDocument {
            id: id.clone(),
            document,
            external_path,
            modified: timestamp(),
            folder: None,
        });
        self.open_tab(&id);
        id
    }

    pub fn open_tab(&mut self, id: &str) {
        if !self.tabs.iter().any(|tab| tab == id) {
            self.tabs.push(id.to_owned());
        }
        self.active = Some(id.to_owned());
    }

    pub fn create_folder(&mut self, name: &str) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Give your folder a name.".into());
        }
        if self
            .folders
            .iter()
            .any(|folder| folder.name.eq_ignore_ascii_case(name))
        {
            return Err("A folder with this name already exists.".into());
        }
        let id = format!("folder-{:x}-{}", timestamp(), self.folders.len());
        self.folders.push(WorkspaceFolder {
            id: id.clone(),
            name: name.to_owned(),
        });
        Ok(id)
    }

    pub fn move_document(&mut self, id: &str, folder: Option<String>) -> bool {
        if folder
            .as_ref()
            .is_some_and(|id| !self.folders.iter().any(|f| &f.id == id))
        {
            return false;
        }
        if let Some(document) = self.documents.iter_mut().find(|d| d.id == id) {
            document.folder = folder;
            return true;
        }
        false
    }

    pub fn unique_title(&self, base: &str) -> String {
        if !self.documents.iter().any(|d| d.document.title == base) {
            return base.to_owned();
        }
        let mut index = 2;
        loop {
            let name = format!("{base} {index}");
            if !self.documents.iter().any(|d| d.document.title == name) {
                return name;
            }
            index += 1;
        }
    }

    pub fn by_title(&self, title: &str) -> Option<&WorkspaceDocument> {
        self.documents
            .iter()
            .find(|d| d.document.title.eq_ignore_ascii_case(title.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autosave_restores_tabs_edits_citations_and_links_after_restart() {
        let directory = std::env::temp_dir().join(format!("flowwrite-test-{}", timestamp()));
        let mut app = crate::FlowWriteApp::empty();
        app.workspace = Workspace::load(directory.clone()).unwrap();
        app.start_fresh();
        app.title = "Research notes".into();
        app.blocks[0].text = "A first draft".into();
        app.citations = vec!["Smith. A Book. 2026.".into()];
        app.dirty = true;
        let first = app.active_document.clone();
        let folder = app.workspace.create_folder("Essays").unwrap();
        assert!(app.workspace.move_document(&first, Some(folder.clone())));
        assert!(app.workspace.create_folder("essays").is_err());
        app.start_fresh();
        app.title = "Essay".into();
        app.blocks[0].text = "See [[Research notes]]".into();
        app.dirty = true;
        assert!(app.save_workspace());
        app.switch_document(&first);
        assert_eq!(app.blocks[0].text, "A first draft");
        app.blocks[0].text = "A revised draft".into();
        app.dirty = true;
        assert!(app.save_workspace());
        let restored = Workspace::load(directory.clone()).unwrap();
        assert_eq!(restored.tabs.len(), 2);
        assert_eq!(restored.folders[0].name, "Essays");
        assert_eq!(
            restored.by_title("Research notes").unwrap().folder.as_ref(),
            Some(&folder)
        );
        assert_eq!(
            restored.by_title("Research notes").unwrap().document.blocks[0].text,
            "A revised draft"
        );
        assert_eq!(
            restored
                .by_title("Research notes")
                .unwrap()
                .document
                .citations
                .len(),
            1
        );
        assert!(restored.by_title("Essay").unwrap().document.blocks[0]
            .text
            .contains("[[Research notes]]"));
        std::fs::remove_file(directory.join("workspace.json")).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn closed_tabs_leave_documents_in_the_workspace() {
        let mut workspace = Workspace::default();
        let document = crate::FlowWriteApp::empty().local_document();
        let id = workspace.add(document, None);
        workspace.tabs.clear();
        assert_eq!(workspace.by_title("Untitled document").unwrap().id, id);
        workspace.open_tab(&id);
        assert_eq!(workspace.tabs, vec![id]);
    }

    #[test]
    fn older_libraries_load_without_folders() {
        let mut workspace = Workspace::default();
        workspace.add(crate::FlowWriteApp::empty().local_document(), None);
        let mut value = serde_json::to_value(&workspace).unwrap();
        value.as_object_mut().unwrap().remove("folders");
        value["documents"][0]
            .as_object_mut()
            .unwrap()
            .remove("folder");
        let restored: Workspace = serde_json::from_value(value).unwrap();
        assert!(restored.folders.is_empty());
        assert!(restored.documents[0].folder.is_none());
    }
}
