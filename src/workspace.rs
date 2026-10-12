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
    #[serde(default = "first_id")]
    pub next_id: u64,
    #[serde(skip)]
    pub recovery_message: Option<String>,
    #[serde(skip)]
    pub read_only: bool,
}

fn first_id() -> u64 {
    1
}

#[derive(Deserialize, Serialize)]
struct LibraryIndex {
    version: u32,
    documents: Vec<DocumentIndex>,
    #[serde(default)]
    tabs: Vec<String>,
    active: Option<String>,
    #[serde(default)]
    folders: Vec<WorkspaceFolder>,
    #[serde(default = "first_id")]
    next_id: u64,
}

#[derive(Deserialize, Serialize)]
struct DocumentIndex {
    id: String,
    #[serde(default)]
    title: String,
    external_path: Option<PathBuf>,
    #[serde(default)]
    modified: u64,
    folder: Option<String>,
}

pub(super) fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub(super) fn default_directory() -> PathBuf {
    let executable = std::env::current_exe().unwrap_or_default();
    if let Some(directory) = std::env::var_os("FLOWRITE_DATA_DIR") {
        return PathBuf::from(directory);
    }
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| executable.parent().unwrap_or(Path::new(".")).to_owned())
        .join("flowrite")
}

pub(super) fn write_atomic(path: &Path, content: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_file_name(format!(
        "{}.flowrite-autosave.tmp",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    if path.exists() {
        std::fs::copy(
            path,
            path.with_file_name(format!(
                "{}.bak",
                path.file_name().unwrap_or_default().to_string_lossy()
            )),
        )?;
    }
    std::fs::write(&temporary, content)?;
    std::fs::rename(&temporary, path)
}

impl Workspace {
    pub fn load(directory: PathBuf) -> Result<Self, String> {
        let mut migrated_name = false;
        if std::env::var_os("FLOWRITE_DATA_DIR").is_none()
            && directory.file_name().is_some_and(|n| n == "flowrite")
            && !directory.join("index.json").exists()
            && !directory.join("workspace.json").exists()
        {
            if let Some(parent) = directory.parent() {
                let old = parent.join("FlowWrite").join("workspace.json");
                if old.exists() {
                    let bytes = std::fs::read(&old).map_err(|e| e.to_string())?;
                    let _: Self = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
                    std::fs::copy(&old, directory.join("workspace.json"))
                        .map_err(|e| e.to_string())?;
                    let backup = old.with_file_name("workspace.json.bak");
                    if backup.exists() {
                        std::fs::copy(backup, directory.join("workspace.json.bak"))
                            .map_err(|e| e.to_string())?;
                    }
                    migrated_name = true;
                }
            }
        }
        let index_path = directory.join("index.json");
        if index_path.exists() {
            let (index, recovery) = read_with_backup::<LibraryIndex>(&index_path)?;
            if index.version > 2 {
                return Err("This library was saved by a newer version of flowrite. It has not been changed.".into());
            }
            let mut workspace = Self {
                directory,
                next_id: index.next_id.max(1),
                tabs: index.tabs,
                active: index.active,
                folders: index.folders,
                recovery_message: recovery,
                ..Default::default()
            };
            for entry in index.documents {
                if !safe_id(&entry.id) {
                    return Err("Invalid document identifier in library index.".into());
                }
                let (document, recovery) = read_with_backup::<SavedDocument>(
                    &workspace
                        .directory
                        .join("documents")
                        .join(format!("{}.json", entry.id)),
                )?;
                if document.version > 2 {
                    return Err(
                        "A document uses a newer flowrite format. It has not been changed.".into(),
                    );
                }
                if recovery.is_some() {
                    workspace.recovery_message = recovery;
                }
                workspace.documents.push(WorkspaceDocument {
                    id: entry.id,
                    document,
                    external_path: entry.external_path,
                    modified: entry.modified,
                    folder: entry.folder,
                });
            }
            workspace.restore_id_counter();
            workspace.validate_tabs();
            return Ok(workspace);
        }
        let file = directory.join("workspace.json");
        let mut workspace = if file.exists() {
            let (mut workspace, recovery) = read_with_backup::<Self>(&file)?;
            workspace.recovery_message = recovery;
            workspace
        } else {
            Self::default()
        };
        workspace.directory = directory;
        if workspace.documents.iter().any(|d| d.document.version > 2) {
            return Err(
                "This library was saved by a newer version of flowrite. It has not been changed."
                    .into(),
            );
        }
        workspace.restore_id_counter();
        if migrated_name {
            workspace.recovery_message = Some("Your library was copied to the flowrite data folder. The earlier FlowWrite folder is still available as a backup.".into());
        }
        workspace.validate_tabs();
        if file.exists() {
            workspace.save()?;
            // Keep the original single-file library after a verified migration.
            let _ = Self::load(workspace.directory.clone())?;
            std::fs::rename(&file, file.with_file_name("workspace.json.migrated"))
                .map_err(|e| e.to_string())?;
        }
        Ok(workspace)
    }

    fn restore_id_counter(&mut self) {
        // Older indexes predate the persisted counter. Derive its lower bound
        // from existing IDs, without rewriting or renaming any library files.
        let next = self
            .documents
            .iter()
            .map(|d| d.id.as_str())
            .chain(self.folders.iter().map(|f| f.id.as_str()))
            .filter_map(|id| {
                id.strip_prefix("document-")
                    .or_else(|| id.strip_prefix("folder-"))
            })
            .filter_map(|suffix| suffix.parse::<u64>().ok())
            .map(|id| id.saturating_add(1))
            .max()
            .unwrap_or(1);
        self.next_id = self.next_id.max(next).max(1);
    }

    fn validate_tabs(&mut self) {
        self.tabs
            .retain(|id| self.documents.iter().any(|doc| &doc.id == id));
        if !self
            .active
            .as_ref()
            .map(|id| self.tabs.contains(id))
            .unwrap_or(false)
        {
            self.active = self.tabs.first().cloned();
        }
    }

    pub fn save(&self) -> Result<(), String> {
        if self.read_only {
            return Err("The library is protected until recovery succeeds.".into());
        }
        if self.directory.as_os_str().is_empty() {
            return Err(
                "The local library is unavailable. Save a copy using File > Save as.".into(),
            );
        }
        let index_path = self.directory.join("index.json");
        if index_path.exists() {
            let bytes = std::fs::read(&index_path).map_err(|e| e.to_string())?;
            check_version(&bytes)?;
            let _: LibraryIndex = serde_json::from_slice(&bytes).map_err(|_| {
                "The library index became unreadable. Recover it before saving.".to_owned()
            })?;
        }
        // Preflight every page before any writes: a newer or corrupt file must
        // not be replaced even if another process changed it after launch.
        for entry in &self.documents {
            if !safe_id(&entry.id) {
                return Err("Invalid document identifier.".into());
            }
            let path = self
                .directory
                .join("documents")
                .join(format!("{}.json", entry.id));
            if path.exists() {
                let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
                check_version(&bytes)?;
                let _: SavedDocument = serde_json::from_slice(&bytes).map_err(|_| {
                    "A document became unreadable. Recover it before saving.".to_owned()
                })?;
            }
        }
        for entry in &self.documents {
            if !safe_id(&entry.id) {
                return Err("Invalid document identifier.".into());
            }
            save_if_changed(
                &self
                    .directory
                    .join("documents")
                    .join(format!("{}.json", entry.id)),
                &entry.document,
            )?;
        }
        let index = LibraryIndex {
            version: 2,
            documents: self
                .documents
                .iter()
                .map(|d| DocumentIndex {
                    id: d.id.clone(),
                    title: d.document.title.clone(),
                    external_path: d.external_path.clone(),
                    modified: d.modified,
                    folder: d.folder.clone(),
                })
                .collect(),
            tabs: self.tabs.clone(),
            active: self.active.clone(),
            folders: self.folders.clone(),
            next_id: self.next_id,
        };
        save_if_changed(&self.directory.join("index.json"), &index)
    }

    pub fn add(&mut self, document: SavedDocument, external_path: Option<PathBuf>) -> String {
        let id = self.allocate_id("document");
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
        let id = self.allocate_id("folder");
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
        if !self
            .documents
            .iter()
            .any(|d| d.document.title.eq_ignore_ascii_case(base))
        {
            return base.to_owned();
        }
        let mut index = 2;
        loop {
            let name = format!("{base} {index}");
            if !self
                .documents
                .iter()
                .any(|d| d.document.title.eq_ignore_ascii_case(&name))
            {
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

    fn allocate_id(&mut self, prefix: &str) -> String {
        loop {
            let id = format!("{prefix}-{}", self.next_id.max(1));
            self.next_id = self.next_id.max(1) + 1;
            if !self.documents.iter().any(|d| d.id == id)
                && !self.folders.iter().any(|f| f.id == id)
                && !self
                    .directory
                    .join("documents")
                    .join(format!("{id}.json"))
                    .exists()
                && !self.directory.join("documents").join(&id).exists()
            {
                return id;
            }
        }
    }

    pub fn delete_document(&mut self, id: &str) -> bool {
        let before = self.documents.len();
        self.documents.retain(|d| d.id != id);
        self.tabs.retain(|tab| tab != id);
        if self.active.as_deref() == Some(id) {
            self.active = self.tabs.last().cloned();
        }
        before != self.documents.len()
    }

    pub fn rename_document(&mut self, id: &str, title: &str) -> Result<(), String> {
        let title = title.trim();
        if title.is_empty() {
            return Err("A page needs a name.".into());
        }
        if self
            .documents
            .iter()
            .any(|d| d.id != id && d.document.title.eq_ignore_ascii_case(title))
        {
            return Err("A page with this name already exists.".into());
        }
        let entry = self
            .documents
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or("Page not found")?;
        entry.document.title = title.into();
        entry.modified = timestamp();
        Ok(())
    }

    pub fn duplicate_document(&mut self, id: &str) -> Result<String, String> {
        let original = self
            .documents
            .iter()
            .find(|d| d.id == id)
            .cloned()
            .ok_or("Page not found")?;
        let mut document = original.document;
        document.title = self.unique_title(&format!("{} copy", document.title));
        let id = self.allocate_id("document");
        let assets = self
            .directory
            .join("documents")
            .join(&original.id)
            .join("assets");
        if assets.exists() {
            let destination = self.directory.join("documents").join(&id).join("assets");
            std::fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
            for entry in std::fs::read_dir(assets).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.file_type().map_err(|e| e.to_string())?.is_file() {
                    std::fs::copy(entry.path(), destination.join(entry.file_name()))
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        self.documents.push(WorkspaceDocument {
            id: id.clone(),
            document,
            external_path: None,
            modified: timestamp(),
            folder: original.folder,
        });
        self.open_tab(&id);
        Ok(id)
    }

    pub fn rename_folder(&mut self, id: &str, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("A folder needs a name.".into());
        }
        if self
            .folders
            .iter()
            .any(|f| f.id != id && f.name.eq_ignore_ascii_case(name))
        {
            return Err("A folder with this name already exists.".into());
        }
        self.folders
            .iter_mut()
            .find(|f| f.id == id)
            .ok_or("Folder not found")?
            .name = name.into();
        Ok(())
    }

    pub fn delete_folder(&mut self, id: &str, delete_pages: bool) {
        if delete_pages {
            let pages: Vec<_> = self
                .documents
                .iter()
                .filter(|d| d.folder.as_deref() == Some(id))
                .map(|d| d.id.clone())
                .collect();
            for page in pages {
                self.delete_document(&page);
            }
        } else {
            for d in &mut self.documents {
                if d.folder.as_deref() == Some(id) {
                    d.folder = None;
                }
            }
        }
        self.folders.retain(|f| f.id != id);
    }
}

fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn check_version(bytes: &[u8]) -> Result<(), String> {
    if serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|value| value.get("version").and_then(|v| v.as_u64()))
        .is_some_and(|version| version > 2)
    {
        return Err(
            "This library contains a newer flowrite format. Its files have not been changed."
                .into(),
        );
    }
    Ok(())
}

fn save_if_changed<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    if std::fs::read(path).ok().as_deref() != Some(bytes.as_slice()) {
        write_atomic(path, &bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn read_with_backup<T: serde::de::DeserializeOwned>(
    path: &Path,
) -> Result<(T, Option<String>), String> {
    // Check the raw version first, including formats whose newer fields cannot
    // deserialize with this executable. Never fall back over a newer primary.
    if let Ok(bytes) = std::fs::read(path) {
        check_version(&bytes)?;
    }
    let read = |path: &Path| {
        std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|e| e.to_string()))
    };
    match read(path) {
        Ok(value) => Ok((value, None)),
        Err(error) => {
            let backup = path.with_file_name(format!(
                "{}.bak",
                path.file_name().unwrap_or_default().to_string_lossy()
            ));
            if let Ok(bytes) = std::fs::read(&backup) {
                check_version(&bytes)?;
            }
            match read(&backup) {
                Ok(value) => {
                    let corrupt = path.with_file_name(format!("{}.{}.corrupt",path.file_name().unwrap_or_default().to_string_lossy(),timestamp()));
                    if path.exists() { std::fs::rename(path,&corrupt).map_err(|e|e.to_string())?; }
                    Ok((value,Some(format!("Recovered {} from its backup. The damaged file is kept at {}.",path.display(),corrupt.display()))))
                },
                Err(_) => Err(format!("{} could not be read: {error}. Its files have been kept; open a backup or start fresh from the recovery screen.",path.display())),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_library() -> Workspace {
        Workspace::load(std::env::temp_dir().join(format!(
                "flowrite-library-{}-{}",
                timestamp(),
                std::thread::current()
                    .name()
                    .unwrap_or("test")
                    .replace(':', "-")
            )))
        .unwrap()
    }
    fn sample(title: &str) -> SavedDocument {
        SavedDocument {
            version: 1,
            title: title.into(),
            blocks: vec![super::super::Block::new(
                1,
                super::super::BlockKind::Text,
                "text",
            )],
            style: super::super::DocumentStyle::default(),
            details: super::super::PaperDetails::default(),
            citations: vec!["Citation".into()],
        }
    }
    #[test]
    fn per_document_save_does_not_rewrite_other_pages() {
        let mut ws = test_library();
        let first = ws.add(sample("First"), None);
        let second = ws.add(sample("Second"), None);
        ws.save().unwrap();
        ws.documents[0].document.blocks[0].text = "changed".into();
        ws.save().unwrap();
        assert!(ws
            .directory
            .join("documents")
            .join(format!("{first}.json.bak"))
            .exists());
        assert!(!ws
            .directory
            .join("documents")
            .join(format!("{second}.json.bak"))
            .exists());
    }
    #[test]
    fn corrupt_legacy_library_recovers_backup() {
        let mut ws = test_library();
        ws.add(sample("Recovered"), None);
        std::fs::create_dir_all(&ws.directory).unwrap();
        std::fs::write(
            ws.directory.join("workspace.json.bak"),
            serde_json::to_vec(&ws).unwrap(),
        )
        .unwrap();
        std::fs::write(ws.directory.join("workspace.json"), b"broken").unwrap();
        let loaded = Workspace::load(ws.directory.clone()).unwrap();
        assert_eq!(loaded.documents[0].document.title, "Recovered");
        assert!(loaded.recovery_message.is_some());
    }
    #[test]
    fn migration_preserves_folders_tabs_citations() {
        let mut ws = test_library();
        let id = ws.add(sample("Old"), None);
        let folder = ws.create_folder("Folder").unwrap();
        ws.move_document(&id, Some(folder.clone()));
        std::fs::create_dir_all(&ws.directory).unwrap();
        std::fs::write(
            ws.directory.join("workspace.json"),
            serde_json::to_vec(&ws).unwrap(),
        )
        .unwrap();
        let loaded = Workspace::load(ws.directory.clone()).unwrap();
        assert_eq!(loaded.documents[0].folder, Some(folder));
        assert_eq!(loaded.tabs, vec![id]);
        assert_eq!(loaded.documents[0].document.citations, vec!["Citation"]);
        assert!(ws.directory.join("workspace.json.migrated").exists());
    }
    #[test]
    fn ids_and_page_folder_actions_remain_safe() {
        let mut ws = test_library();
        let first = ws.add(sample("First"), None);
        let folder = ws.create_folder("Old folder").unwrap();
        ws.move_document(&first, Some(folder.clone()));
        let copy = ws.duplicate_document(&first).unwrap();
        assert_ne!(first, copy);
        assert_eq!(
            ws.documents[1].document.citations,
            ws.documents[0].document.citations
        );
        assert!(ws.rename_document(&copy, "First").is_err());
        assert!(ws.rename_document(&copy, "").is_err());
        ws.rename_folder(&folder, "New folder").unwrap();
        ws.delete_folder(&folder, false);
        assert_eq!(ws.documents.len(), 2);
        assert!(ws.documents.iter().all(|d| d.folder.is_none()));
        ws.delete_document(&first);
        let third = ws.add(sample("Third"), None);
        assert_ne!(first, third);
        ws.save().unwrap();
        let mut loaded = Workspace::load(ws.directory.clone()).unwrap();
        loaded.delete_document(&third);
        assert_ne!(third, loaded.add(sample("Fourth"), None));
    }
    #[test]
    fn newer_library_is_not_modified() {
        let ws = test_library();
        std::fs::create_dir_all(&ws.directory).unwrap();
        let bytes=b"{\"version\":999,\"documents\":[],\"tabs\":[],\"active\":null,\"folders\":[],\"next_id\":1}";
        std::fs::write(ws.directory.join("index.json"), bytes).unwrap();
        assert!(Workspace::load(ws.directory.clone()).is_err());
        assert_eq!(
            std::fs::read(ws.directory.join("index.json")).unwrap(),
            bytes
        );
    }

    #[test]
    fn index_without_counter_loads_and_upgrades_without_data_loss() {
        let mut ws = test_library();
        ws.next_id = 41;
        let id = ws.add(sample("Existing essay"), None);
        let folder = ws.create_folder("Essays").unwrap();
        ws.move_document(&id, Some(folder.clone()));
        ws.save().unwrap();
        let index_path = ws.directory.join("index.json");
        let page_path = ws.directory.join("documents").join(format!("{id}.json"));
        let page_bytes = std::fs::read(&page_path).unwrap();
        let mut old: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&index_path).unwrap()).unwrap();
        old.as_object_mut().unwrap().remove("next_id");
        let old_bytes = serde_json::to_vec(&old).unwrap();
        std::fs::write(&index_path, &old_bytes).unwrap();

        let mut loaded = Workspace::load(ws.directory.clone()).unwrap();
        assert_eq!(loaded.documents.len(), 1);
        assert_eq!(loaded.documents[0].document, ws.documents[0].document);
        assert_eq!(loaded.documents[0].folder.as_ref(), Some(&folder));
        assert_eq!(loaded.tabs, vec![id.clone()]);
        assert_eq!(loaded.active.as_deref(), Some(id.as_str()));
        assert_eq!(loaded.next_id, 43);
        assert!(loaded.recovery_message.is_none());
        assert_eq!(std::fs::read(&index_path).unwrap(), old_bytes);
        loaded.save().unwrap();
        assert_eq!(
            std::fs::read(index_path.with_file_name("index.json.bak")).unwrap(),
            old_bytes
        );
        assert_eq!(std::fs::read(&page_path).unwrap(), page_bytes);
        assert!(!page_path.with_file_name(format!("{id}.json.bak")).exists());
        loaded.delete_document(&id);
        loaded.delete_folder(&folder, false);
        assert_eq!(loaded.add(sample("New page"), None), "document-43");
    }

    #[test]
    fn early_index_optional_metadata_uses_safe_defaults() {
        let index: LibraryIndex =
            serde_json::from_str(r#"{"version":1,"documents":[{"id":"document-7"}]}"#).unwrap();
        assert_eq!(index.next_id, 1);
        assert!(index.tabs.is_empty());
        assert!(index.folders.is_empty());
        assert_eq!(index.documents[0].modified, 0);
        assert!(index.documents[0].folder.is_none());
    }

    #[test]
    fn stale_counter_is_raised_above_existing_ids() {
        let mut ws = test_library();
        ws.next_id = 90;
        ws.add(sample("Existing page"), None);
        ws.create_folder("Existing folder").unwrap();
        ws.next_id = 1;
        ws.save().unwrap();
        let mut loaded = Workspace::load(ws.directory.clone()).unwrap();
        assert_eq!(loaded.next_id, 92);
        assert_eq!(loaded.add(sample("New page"), None), "document-92");
    }

    #[test]
    fn protected_or_changed_library_cannot_be_saved() {
        let mut ws = test_library();
        ws.add(sample("Saved page"), None);
        ws.save().unwrap();
        let index = ws.directory.join("index.json");
        let document = ws.directory.join("documents/document-1.json");
        let original = std::fs::read(&document).unwrap();
        ws.documents[0].document.title = "Changed title".into();
        ws.read_only = true;
        assert!(ws.save().is_err());
        assert_eq!(std::fs::read(&document).unwrap(), original);
        ws.read_only = false;
        for bytes in [
            b"{\"version\":999,\"future_shape\":true}".as_slice(),
            b"broken".as_slice(),
        ] {
            std::fs::write(&index, bytes).unwrap();
            assert!(ws.save().is_err());
            assert_eq!(std::fs::read(&index).unwrap(), bytes);
            assert_eq!(std::fs::read(&document).unwrap(), original);
        }
    }

    #[test]
    fn newer_document_is_not_replaced_by_an_older_backup() {
        let mut ws = test_library();
        ws.add(sample("Saved page"), None);
        ws.save().unwrap();
        let document = ws.directory.join("documents/document-1.json");
        std::fs::copy(&document, document.with_file_name("document-1.json.bak")).unwrap();
        let newer = b"{\"version\":999,\"future_shape\":true}";
        std::fs::write(&document, newer).unwrap();
        assert!(Workspace::load(ws.directory.clone())
            .unwrap_err()
            .contains("newer"));
        assert!(ws.save().unwrap_err().contains("newer"));
        assert_eq!(std::fs::read(document).unwrap(), newer);
    }

    #[test]
    fn orphaned_asset_directories_do_not_reuse_document_ids() {
        let mut ws = test_library();
        std::fs::create_dir_all(ws.directory.join("documents/document-1/assets")).unwrap();
        assert_ne!(ws.add(sample("New page"), None), "document-1");
    }

    #[test]
    fn autosave_restores_tabs_edits_citations_and_links_after_restart() {
        let directory = std::env::temp_dir().join(format!("flowwrite-test-{}", timestamp()));
        let mut app = crate::FlowriteApp::empty();
        app.workspace = Workspace::load(directory.clone()).unwrap();
        app.start_fresh();
        app.title = "Research notes".into();
        app.blocks[0].text = "A first draft".into();
        app.citations = vec!["Smith. A Book. 2026.".into()];
        app.dirty = true;
        assert!(app.save_workspace());
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
            .contains("[[Research notes|"));
    }

    #[test]
    fn closed_tabs_leave_documents_in_the_workspace() {
        let mut workspace = Workspace::default();
        let document = crate::FlowriteApp::empty().local_document();
        let id = workspace.add(document, None);
        workspace.tabs.clear();
        assert_eq!(workspace.by_title("Untitled document").unwrap().id, id);
        workspace.open_tab(&id);
        assert_eq!(workspace.tabs, vec![id]);
    }

    #[test]
    fn older_libraries_load_without_folders() {
        let mut workspace = Workspace::default();
        workspace.add(crate::FlowriteApp::empty().local_document(), None);
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
