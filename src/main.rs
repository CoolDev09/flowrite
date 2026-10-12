#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use eframe::egui::{
    self, Align, Color32, FontFamily, FontId, Frame, Layout, Margin, Pos2, RichText, Rounding,
    Sense, Stroke, TextEdit, Vec2,
};
use printpdf::{Mm, PdfDocument};
use rfd::FileDialog;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("flowrite")
            .with_icon(brand::window_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "flowrite",
        options,
        Box::new(|cc| Ok(Box::new(FlowriteApp::new(cc)))),
    )
}

mod document;
use document::*;
mod workspace;
use workspace::Workspace;
mod brand;
mod bundle;
mod colors;
mod command_palette;
mod containers;
mod context_menu;
mod date;
mod fonts;
mod history;
mod icons;
mod images;
use icons::Icon;
mod recovery;
mod search;
mod settings;
mod settings_ui;
mod spellcheck;
mod text_layout;
mod theme;
mod widgets;
use widgets::HandCursor;
mod navigation;
use history::UndoSnapshot;
mod home_ui;
mod wikilinks;
mod workspace_ui;

#[derive(Clone, Copy)]
enum PendingAction {
    New,
    Open,
    Close,
}

#[derive(Clone, Copy)]
enum Decoration {
    Bold,
    Italic,
    Underline,
}

pub struct FlowriteApp {
    title: String,
    blocks: Vec<Block>,
    next_id: u64,
    style: DocumentStyle,
    details: PaperDetails,
    current_path: Option<PathBuf>,
    dirty: bool,
    slash_target: Option<usize>,
    block_picker_target: Option<usize>,
    notice: Option<String>,
    notice_start: Option<f64>,
    notice_previous: Option<String>,
    exported_path: Option<PathBuf>,
    focused_block: Option<u64>,
    selection: Option<(u64, usize, usize)>,
    focus_request: Option<(u64, usize)>,
    show_inspector: bool,
    find_open: bool,
    find_query: String,
    search: search::Search,
    scroll_target: Option<u64>,
    command_index: usize,
    command_query: String,
    command_position: Pos2,
    dismissed_command: Option<u64>,
    undo_stack: Vec<UndoSnapshot>,
    redo_stack: Vec<UndoSnapshot>,
    history_baseline: Option<UndoSnapshot>,
    last_history_time: f64,
    force_history: bool,
    pending_action: Option<PendingAction>,
    allow_close: bool,
    history_navigation: bool,
    command_scrolling: bool,
    workspace: Workspace,
    active_document: String,
    workspace_available: bool,
    show_sidebar: bool,
    sidebar_query: String,
    citations: Vec<String>,
    show_citations: bool,
    last_document_change: f64,
    link_request: Option<String>,
    show_home: bool,
    home_query: String,
    current_folder: Option<String>,
    folder_dialog: bool,
    folder_name: String,
    folder_error: Option<String>,
    folder_focus: bool,
    settings: settings::Settings,
    theme: theme::Theme,
    settings_open: bool,
    settings_section: usize,
    settings_query: String,
    dialogs: context_menu::Dialogs,
    proofreader: spellcheck::Proofreader,
    palette: command_palette::Palette,
    closed_tabs: Vec<String>,
    focus_mode: bool,
    image_cache: std::collections::HashMap<String, egui::TextureHandle>,
    recovery: recovery::Recovery,
}

impl FlowriteApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_fonts(&cc.egui_ctx);
        let mut app = Self::empty();
        let (settings, notice) = settings::Settings::load(&workspace::default_directory());
        app.settings = settings;
        app.initialize_workspace();
        if notice.is_some() {
            app.notice = notice;
        }
        app.show_home = !app.settings.general.restore_tabs || app.active_document.is_empty();
        app.apply_settings(&cc.egui_ctx);
        app.proofreader.start(&cc.egui_ctx);
        app
    }

    fn empty() -> Self {
        Self {
            title: "Untitled document".to_owned(),
            blocks: vec![Block::new(1, BlockKind::Text, "")],
            next_id: 2,
            style: DocumentStyle::default(),
            details: PaperDetails::default(),
            current_path: None,
            dirty: false,
            slash_target: None,
            block_picker_target: None,
            notice: None,
            notice_start: None,
            notice_previous: None,
            exported_path: None,
            focused_block: None,
            selection: None,
            focus_request: None,
            show_inspector: false,
            find_open: false,
            find_query: String::new(),
            search: search::Search::default(),
            scroll_target: None,
            command_index: 0,
            command_query: String::new(),
            command_position: Pos2::ZERO,
            dismissed_command: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            history_baseline: None,
            last_history_time: -10.0,
            force_history: false,
            pending_action: None,
            allow_close: false,
            history_navigation: false,
            command_scrolling: false,
            workspace: Workspace::default(),
            active_document: String::new(),
            workspace_available: true,
            show_sidebar: true,
            sidebar_query: String::new(),
            citations: Vec::new(),
            show_citations: false,
            last_document_change: 0.0,
            link_request: None,
            show_home: true,
            home_query: String::new(),
            current_folder: None,
            folder_dialog: false,
            folder_name: String::new(),
            folder_error: None,
            folder_focus: false,
            settings: settings::Settings::default(),
            theme: theme::Theme::default(),
            settings_open: false,
            settings_section: 0,
            settings_query: String::new(),
            dialogs: context_menu::Dialogs::default(),
            proofreader: spellcheck::Proofreader::default(),
            palette: command_palette::Palette::default(),
            closed_tabs: Vec::new(),
            focus_mode: false,
            image_cache: std::collections::HashMap::new(),
            recovery: recovery::Recovery::default(),
        }
    }

    fn make_block(&mut self, kind: BlockKind, text: impl Into<String>) -> Block {
        let block = Block::new(self.next_id, kind, text);
        self.next_id += 1;
        block
    }

    fn start_fresh(&mut self) {
        if !self.active_document.is_empty() && !self.save_workspace() {
            return;
        }
        self.title = "Untitled document".to_owned();
        self.title = self.workspace.unique_title(&self.title);
        self.blocks = vec![self.make_block(BlockKind::Text, "")];
        self.style.apply_preset(PaperFormat::Freeform);
        self.style.font = self.settings.editor.font.clone();
        self.style.font_size = self.settings.editor.size;
        self.style.line_spacing = self.settings.editor.spacing;
        self.style.margin_inches = self.settings.editor.margin;
        self.details = PaperDetails {
            author: self.settings.general.author.clone(),
            instructor: self.settings.general.instructor.clone(),
            course: self.settings.general.course.clone(),
            due_date: date::today(self.settings.general.date_format),
            ..Default::default()
        };
        self.citations.clear();
        self.current_path = None;
        self.dirty = false;
        self.slash_target = None;
        self.focused_block = Some(self.blocks[0].id);
        self.focus_request = Some((self.blocks[0].id, 0));
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.notice = None;
        self.active_document.clear();
        self.show_home = false;
        self.show_sidebar = true;
        self.show_inspector = false;
        self.show_citations = false;
        self.history_navigation = true;
        self.reset_history_baseline();
    }

    fn local_document(&self) -> SavedDocument {
        SavedDocument {
            version: 2,
            title: self.title.clone(),
            blocks: self.blocks.clone(),
            style: self.style.clone(),
            details: self.details.clone(),
            citations: self.citations.clone(),
        }
    }

    fn save_document(&mut self, save_as: bool) -> bool {
        if !save_as {
            return self.save_workspace();
        }
        let destination = if save_as || self.current_path.is_none() {
            let default_name = format!("{}.flowrite", safe_filename(&self.title));
            FileDialog::new()
                .set_title("Save flowrite document")
                .set_file_name(default_name)
                .add_filter("flowrite document", &["flowrite"])
                .save_file()
        } else {
            self.current_path.clone()
        };

        let Some(path) = destination else {
            self.notice = Some("Save cancelled".to_owned());
            return false;
        };

        self.commit_current();
        let result = self.write_bundle(&path);

        let success = result.is_ok();
        self.notice = Some(match result {
            Ok(()) => {
                self.current_path = Some(path.clone());
                self.dirty = false;
                self.commit_current();
                if self.workspace_available {
                    let _ = self.workspace.save();
                }
                format!("Saved {}", path.display())
            }
            Err(error) => format!("Could not save: {error}"),
        });
        success
    }

    fn open_document(&mut self) {
        let Some(path) = FileDialog::new()
            .set_title("Open flowrite document")
            .add_filter("flowrite document", &["flowrite", "flowwrite"])
            .pick_file()
        else {
            self.notice = Some("Open cancelled".to_owned());
            return;
        };

        let result = bundle::read(&path);

        match result {
            Ok((document, assets)) if document.version <= 2 => {
                if !self.workspace_available && !assets.is_empty() {
                    self.notice=Some("Recover your library before opening an image-containing document. Its files have not been changed.".into());
                    return;
                }
                if !self.active_document.is_empty() && !self.save_workspace() {
                    return;
                }
                self.title = document.title;
                self.blocks = if document.blocks.is_empty() {
                    vec![self.make_block(BlockKind::Text, "")]
                } else {
                    document.blocks
                };
                self.next_id = self.blocks.iter().map(|block| block.id).max().unwrap_or(0) + 1;
                self.style = document.style;
                self.details = document.details;
                self.citations = document.citations;
                self.current_path = Some(path.clone());
                self.dirty = false;
                self.slash_target = None;
                self.focused_block = None;
                self.selection = None;
                self.undo_stack.clear();
                self.redo_stack.clear();
                let existing = self
                    .workspace
                    .documents
                    .iter()
                    .find(|doc| doc.external_path.as_ref() == Some(&path))
                    .map(|doc| doc.id.clone());
                self.active_document = existing.unwrap_or_else(|| {
                    self.workspace
                        .add(self.local_document(), Some(path.clone()))
                });
                self.workspace.open_tab(&self.active_document.clone());
                let folder = images::asset_directory(&self.workspace, &self.active_document);
                let mut import_error = None;
                for (name, bytes) in assets {
                    if let Err(error) = workspace::write_atomic(&folder.join(name), &bytes) {
                        import_error = Some(format!("Could not restore an image: {error}"));
                    }
                }
                for block in &mut self.blocks {
                    wikilinks::display(block, &self.workspace);
                }
                self.history_navigation = true;
                self.show_home = false;
                self.show_sidebar = true;
                self.commit_current();
                if self.workspace_available {
                    let _ = self.workspace.save();
                }
                self.notice =
                    Some(import_error.unwrap_or_else(|| format!("Opened {}", path.display())));
                self.resolve_document_font();
                self.reset_history_baseline();
            }
            Ok(_) => self.notice = Some("That document uses a newer flowrite format.".to_owned()),
            Err(error) => self.notice = Some(format!("Could not open: {error}")),
        }
    }

    fn apply_command(&mut self, index: usize, kind: BlockKind) {
        if let Some(block) = self.blocks.get_mut(index) {
            let command = block.text.trim().trim_start_matches('/').to_lowercase();
            block.kind = kind;
            if kind == BlockKind::Callout {
                block.callout_kind = match command.as_str() {
                    "tip" => CalloutKind::Tip,
                    "warning" => CalloutKind::Warning,
                    "important" => CalloutKind::Important,
                    _ => CalloutKind::Note,
                };
            }
            if self.slash_target == Some(index) {
                block.text.clear();
            }
        }
        self.slash_target = None;
        self.block_picker_target = None;
        self.dirty = true;
        self.force_history = true;
        if let Some(block) = self.blocks.get(index) {
            self.focused_block = Some(block.id);
            self.focus_request = Some((block.id, block.text.chars().count()));
        }
    }
}

mod editor_ui;
use editor_ui::{configure_fonts, configure_visuals};

mod export;
use export::safe_filename;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_preserves_block_types() {
        let mut app = FlowriteApp::empty();
        app.title = "Sample".to_owned();
        app.blocks = vec![
            Block::new(1, BlockKind::Heading1, "Ideas"),
            Block::new(2, BlockKind::Todo, "Ship it"),
        ];
        app.blocks[1].checked = true;
        let markdown = app.markdown();
        assert!(markdown.contains("# Sample"));
        assert!(markdown.contains("# Ideas"));
        assert!(markdown.contains("- [x] Ship it"));
    }

    #[test]
    fn academic_presets_apply_paper_rules() {
        let mut style = DocumentStyle::default();
        style.apply_preset(PaperFormat::Apa);
        assert_eq!(style.font, "Liberation Serif");
        assert_eq!(style.font_size, 12.0);
        assert_eq!(style.line_spacing, 2.0);
        assert_eq!(style.margin_inches, 1.0);
    }

    #[test]
    fn saved_documents_round_trip() {
        let document = SavedDocument {
            version: 1,
            title: "Offline draft".to_owned(),
            blocks: vec![Block::new(1, BlockKind::Text, "Kept locally")],
            style: DocumentStyle::default(),
            details: PaperDetails::default(),
            citations: Vec::new(),
        };
        let encoded = serde_json::to_string(&document).expect("document should serialize");
        let decoded: SavedDocument =
            serde_json::from_str(&encoded).expect("document should deserialize");
        assert_eq!(decoded.title, "Offline draft");
        assert_eq!(decoded.blocks[0].text, "Kept locally");
    }
}
