//! Exact, paginated print preview without a second layout engine or runtime download.
use super::*;
impl FlowriteApp {
    pub(super) fn paper_preview(&mut self) {
        let directory = std::env::temp_dir().join("flowrite-paper-preview");
        let path = directory.join(format!("preview-{}.pdf", workspace::timestamp()));
        let result = std::fs::create_dir_all(&directory)
            .map_err(|e| e.to_string())
            .and_then(|_| self.write_pdf(&path).map_err(|e| e.to_string()));
        match result {
            Ok(()) => {
                settings_ui::open_path(&path);
                self.notice = Some("Paper preview opened in your PDF viewer.".into());
            }
            Err(error) => {
                self.notice = Some(format!(
                    "Paper preview could not open. Your document is unchanged. Details: {error}"
                ))
            }
        }
    }
}
