use super::*;
const SECTIONS: [&str; 9] = [
    "General",
    "Appearance",
    "Editor",
    "Saving",
    "Citations & formatting",
    "Export",
    "Keyboard shortcuts",
    "About & privacy",
    "Writing & proofing",
];

pub(super) fn setting_matches(query: &str, section: &str, label: &str, description: &str) -> bool {
    let fields = format!("{section} {label} {description}").to_lowercase();
    query
        .split_whitespace()
        .all(|word| fields.contains(&word.to_lowercase()))
}

struct Form<'a> {
    ui: &'a mut egui::Ui,
    section: &'static str,
    query: &'a str,
    shown: usize,
}
impl<'a> Form<'a> {
    fn new(ui: &'a mut egui::Ui, section: &'static str, query: &'a str) -> Self {
        Self {
            ui,
            section,
            query,
            shown: 0,
        }
    }
    fn row(
        &mut self,
        label: &str,
        description: &str,
        kind: egui::WidgetType,
        render: impl FnOnce(&mut egui::Ui) -> egui::Response,
    ) -> Option<egui::Response> {
        self.ui.ctx().data_mut(|data| {
            data.get_temp_mut_or_default::<Vec<(String, String, String)>>(egui::Id::new(
                "settings_catalog",
            ))
            .push((self.section.into(), label.into(), description.into()))
        });
        if !setting_matches(self.query, self.section, label, description) {
            return None;
        }
        if self.shown == 0 {
            self.ui.heading(self.section);
            self.ui.add_space(16.0);
        }
        self.shown += 1;
        let mut label_id = egui::Id::NULL;
        let response = self
            .ui
            .push_id(label, |ui| {
                Frame::none()
                    .inner_margin(Margin::symmetric(4.0, 6.0))
                    .show(ui, |ui| {
                        let width = ui.available_width().min(760.0);
                        ui.spacing_mut().slider_width =
                            (width - if width < 480.0 { 64.0 } else { 290.0 }).clamp(60.0, 220.0);
                        if width < 480.0 {
                            ui.vertical(|ui| {
                                label_id = ui.add(egui::Label::new(label).wrap()).id;
                                ui.add_space(4.0);
                                render(ui)
                            })
                            .inner
                        } else {
                            ui.horizontal_top(|ui| {
                                ui.allocate_ui_with_layout(
                                    Vec2::new(210.0, 32.0),
                                    Layout::top_down(Align::LEFT),
                                    |ui| {
                                        label_id = ui.add(egui::Label::new(label).wrap()).id;
                                    },
                                );
                                ui.allocate_ui_with_layout(
                                    Vec2::new(width - 226.0, 32.0),
                                    Layout::top_down(Align::LEFT),
                                    render,
                                )
                                .inner
                            })
                            .inner
                        }
                    })
                    .inner
            })
            .inner
            .labelled_by(label_id);
        let response = design::named(response, kind, label).on_hover_text(description);
        #[cfg(test)]
        self.ui.ctx().data_mut(|data| {
            data.get_temp_mut_or_default::<Vec<(String, egui::Rect)>>(egui::Id::new(
                "settings_bounds",
            ))
            .push((label.into(), response.rect))
        });
        design::focus_ring(self.ui, &response);
        Some(response)
    }
    fn checkbox(&mut self, label: &str, description: &str, value: &mut bool) {
        let state = if *value { "On" } else { "Off" };
        let response = self.row(label, description, egui::WidgetType::Checkbox, |ui| {
            ui.checkbox(value, state).hand()
        });
        if let Some(response) = response {
            response.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::Checkbox,
                    response.enabled(),
                    *value,
                    label,
                )
            });
        }
    }
    fn text(&mut self, label: &str, description: &str, value: &mut String) {
        self.row(label, description, egui::WidgetType::TextEdit, |ui| {
            ui.add(
                TextEdit::singleline(value)
                    .desired_width(f32::INFINITY)
                    .margin(8.0),
            )
        });
    }
    fn slider(
        &mut self,
        label: &str,
        description: &str,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
    ) {
        self.row(label, description, egui::WidgetType::Slider, |ui| {
            ui.spacing_mut().slider_width = (ui.available_width() - 60.0).clamp(60.0, 220.0);
            ui.add(egui::Slider::new(value, range)).hand()
        });
    }
    fn button(&mut self, label: &str, description: &str) -> bool {
        self.row(label, description, egui::WidgetType::Button, |ui| {
            ui.button(label).hand()
        })
        .is_some_and(|r| r.clicked())
    }
}

impl FlowriteApp {
    pub(super) fn apply_settings(&mut self, ctx: &egui::Context) {
        self.theme = theme::Theme::from_settings(&self.settings.appearance);
        theme::apply(self.theme);
        configure_visuals(ctx, &self.theme);
        ctx.set_zoom_factor(self.settings.appearance.scale.clamp(0.9, 1.5));
        ctx.data_mut(|d| {
            d.insert_temp(
                egui::Id::new("pointer_enabled"),
                self.settings.appearance.hand_pointer,
            )
        });
    }
    pub(super) fn settings_page(&mut self, ctx: &egui::Context) {
        self.header(ctx);
        let before = self.settings.clone();
        let width = design::panel_widths(
            ctx.screen_rect().width(),
            self.settings.appearance.sidebar_width,
            false,
        )
        .0;
        egui::SidePanel::left("settings_categories")
            .exact_width(width)
            .resizable(false)
            .frame(Frame::none().fill(self.theme.panel).inner_margin(12.0))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                let response = ui.add(
                    TextEdit::singleline(&mut self.settings_query)
                        .hint_text("Search settings…")
                        .desired_width(f32::INFINITY)
                        .margin(8.0),
                );
                design::named(
                    response,
                    egui::WidgetType::TextEdit,
                    "Search settings by control label",
                );
                ui.add_space(12.0);
                egui::ScrollArea::vertical()
                    .id_salt("settings_navigation")
                    .max_height((ui.available_height() - 72.0).max(64.0))
                    .show(ui, |ui| {
                        for (section, label) in SECTIONS.iter().enumerate() {
                            if design::nav_row(
                                ui,
                                Icon::Settings,
                                label,
                                self.settings_section == section && self.settings_query.is_empty(),
                            )
                            .clicked()
                            {
                                self.settings_section = section;
                                self.settings_query.clear();
                            }
                        }
                    });
                ui.add_space(16.0);
                ui.separator();
                if design::nav_row(ui, Icon::Undo, "Back to workspace", false).clicked() {
                    self.settings_open = false;
                }
            });
        ctx.data_mut(|data| {
            data.insert_temp(
                egui::Id::new("settings_catalog"),
                Vec::<(String, String, String)>::new(),
            )
        });
        let query = self.settings_query.clone();
        egui::CentralPanel::default().frame(Frame::none().fill(self.theme.canvas).inner_margin(24.0)).show(ctx, |ui| {
            egui::ScrollArea::vertical().id_salt("settings_content").show(ui, |ui| {
                ui.set_max_width(ui.available_width().min(800.0)); let mut shown = 0;
                for (section, title) in SECTIONS.iter().enumerate() {
                    let section_query = if query.is_empty() && section != self.settings_section { "\0no category selected" } else { &query };
                    let mut form = Form::new(ui, title, section_query);
                    match section {
                        0 => {
                            form.checkbox("Restore last open tabs on startup", "Continue with the last saved workspace", &mut self.settings.general.restore_tabs);
                            form.checkbox("Ask where to save every export", "Show the save dialog instead of using the export folder", &mut self.settings.general.ask_export);
                            let path = self.settings.general.folder.display().to_string();
                            if form.row("Default export folder", "Folder for PDF and Markdown exports", egui::WidgetType::Button, |ui| ui.button(sidebar::short_name(&path, 38)).on_hover_text(&path).hand()).is_some_and(|r| r.clicked()) { if let Some(path) = FileDialog::new().pick_folder() { self.settings.general.folder = path; } }
                            form.row("Date format", "Today's date in new academic headings", egui::WidgetType::ComboBox, |ui| {
                                let labels = ["10 October 2026", "October 10, 2026", "10/10/2026"];
                                egui::ComboBox::from_id_salt("date_format").selected_text(labels[self.settings.general.date_format.min(2)]).show_ui(ui, |ui| { for (i, label) in labels.iter().enumerate() { ui.selectable_value(&mut self.settings.general.date_format, i, *label).hand(); } }).response.hand()
                            });
                            form.text("Name", "Default author for academic papers", &mut self.settings.general.author);
                            form.text("Instructor", "Default instructor for academic papers", &mut self.settings.general.instructor);
                            form.text("Course", "Default course for academic papers", &mut self.settings.general.course);
                            if form.shown > 0 { form.ui.add_space(24.0); form.ui.separator(); form.ui.label("Maintenance · confirmation required"); }
                            if form.row("Delete all empty pages…", "Cleanup; preserves document files and requires confirmation", egui::WidgetType::Button, |ui| icons::danger(ui, "Delete…")).is_some_and(|r| r.clicked()) { self.dialogs.action = Some(context_menu::PageAction::Cleanup); }
                            if form.row("Reset all settings…", "Reset preferences with confirmation", egui::WidgetType::Button, |ui| icons::danger(ui, "Reset…")).is_some_and(|r| r.clicked()) { self.dialogs.action = Some(context_menu::PageAction::ResetSettings); }
                        }
                        1 => {
                            form.checkbox("Dim page background", "Neutral dark mode remains enabled", &mut self.settings.appearance.dim_page);
                            form.row("Accent", "Primary actions and keyboard focus", egui::WidgetType::ComboBox, |ui| egui::ComboBox::from_id_salt("accent").selected_text(theme::ACCENTS[self.settings.appearance.accent.min(8)].0).show_ui(ui, |ui| { for (i, (label, _)) in theme::ACCENTS.iter().enumerate() { ui.selectable_value(&mut self.settings.appearance.accent, i, *label).hand(); } }).response.hand());
                            form.row("Interface size", "Application scale does not change document font size", egui::WidgetType::ComboBox, |ui| egui::ComboBox::from_id_salt("interface_size").selected_text(format!("{}%", (self.settings.appearance.scale * 100.0) as i32)).show_ui(ui, |ui| { for scale in [0.9, 1.0, 1.1, 1.25, 1.5] { ui.selectable_value(&mut self.settings.appearance.scale, scale, format!("{}%", (scale * 100.0) as i32)).hand(); } }).response.hand());
                            form.checkbox("Recent documents as a list", "Home also has a visible view switch", &mut self.settings.appearance.list_view);
                            form.row("Recent documents", "Maximum pages shown on Home", egui::WidgetType::Slider, |ui| ui.add(egui::Slider::new(&mut self.settings.appearance.recent_limit, 1..=100)).hand());
                            form.checkbox("Show thumbnails", "Readable content previews in the gallery", &mut self.settings.appearance.thumbnails);
                            form.slider("Sidebar width", "Navigation narrows to protect writing width", &mut self.settings.appearance.sidebar_width, 164.0..=280.0);
                            form.checkbox("Show folders in sidebar", "Group pages by folder", &mut self.settings.appearance.folders);
                            form.checkbox("High contrast", "Stronger text and keyboard focus", &mut self.settings.appearance.high_contrast);
                            form.checkbox("Hand pointer on buttons", "Use a pointing hand for actions", &mut self.settings.appearance.hand_pointer);
                        }
                        2 => {
                            form.row("Default font", "New notes; academic presets use their own typography", egui::WidgetType::ComboBox, |ui| egui::ComboBox::from_id_salt("default_font").selected_text(&self.settings.editor.font).show_ui(ui, |ui| { for name in fonts::FAMILIES { ui.selectable_value(&mut self.settings.editor.font, name.to_owned(), name).hand(); } }).response.hand());
                            form.slider("Font size (pt)", "New notes default to 12 pt / 16 px", &mut self.settings.editor.size, 8.0..=32.0);
                            form.slider("Line spacing", "Line height for new notes", &mut self.settings.editor.spacing, 1.0..=3.0);
                            form.slider("Margins (inches)", "Default freeform print margins", &mut self.settings.editor.margin, 0.25..=2.0);
                            form.checkbox("Always show formatting toolbar", "Otherwise selected text shows contextual formatting", &mut self.settings.editor.toolbar);
                            form.checkbox("Markdown shortcuts", "Typed block shortcuts, such as three backticks", &mut self.settings.editor.markdown);
                            form.checkbox("Slash menu", "Type / to insert blocks", &mut self.settings.editor.slash);
                            form.checkbox("Show block handles", "Handles appear on hover and editing focus", &mut self.settings.editor.handles);
                            form.checkbox("Word count", "Single statistics area", &mut self.settings.editor.words);
                            form.checkbox("Character count", "Unicode character count", &mut self.settings.editor.characters);
                            form.checkbox("Reading time", "Estimate reading time", &mut self.settings.editor.reading_time);
                            form.row("Word goal (0 disables)", "Word-goal progress", egui::WidgetType::DragValue, |ui| ui.add(egui::DragValue::new(&mut self.settings.editor.word_goal).range(0..=100_000)));
                        }
                        3 => {
                            form.row("Autosave delay (seconds)", "Save after a pause, on page changes and close", egui::WidgetType::Slider, |ui| ui.add(egui::Slider::new(&mut self.settings.saving.delay, 0.3..=10.0)));
                            form.checkbox("Show Saved indicator", "Subtle save status in File and the footer", &mut self.settings.saving.indicator);
                            if form.button("Open library folder", "Independent documents and previous-version .bak copies") { open_path(&self.workspace.directory); }
                            if !self.workspace_available && form.button("Library recovery…", "Retry without discarding documents") { self.recovery.open = true; }
                        }
                        4 => {
                            form.text("Citation section title (blank uses preset)", "MLA: Works Cited; APA: References", &mut self.settings.citations.heading);
                            form.checkbox("Alphabetize citations", "Sort ready-made entries by author", &mut self.settings.citations.alphabetize);
                            form.slider("Hanging indent (inches)", "Indent subsequent citation lines", &mut self.settings.citations.hanging_inches, 0.0..=1.0);
                            form.checkbox("Running name and page number", "Academic PDF running header", &mut self.settings.citations.running_header);
                        }
                        5 => {
                            form.checkbox("PDF paper: A4 (otherwise Letter)", "Size used by export and paper preview", &mut self.settings.export.a4);
                            form.checkbox("PDF uses page colors (otherwise black on white)", "Default black-on-white print styling", &mut self.settings.export.page_colors);
                            form.checkbox("Include images", "Image frames and captions in exports", &mut self.settings.export.include_images);
                            form.checkbox("Include academic page numbers", "MLA and APA page numbers", &mut self.settings.export.page_numbers);
                            form.checkbox("Show export notification", "Short confirmations with optional details", &mut self.settings.export.toast);
                            form.row("Notification duration (seconds)", "Export confirmation lifetime", egui::WidgetType::Slider, |ui| ui.add(egui::Slider::new(&mut self.settings.export.toast_seconds, 1.0..=15.0)));
                            form.checkbox("Open file after export", "Default system application", &mut self.settings.export.auto_open);
                        }
                        6 => for (action, key) in [("New page", "Ctrl+N"), ("New folder", "Ctrl+Shift+N"), ("Save", "Ctrl+S"), ("Save a copy", "Ctrl+Shift+S"), ("Find", "Ctrl+F"), ("Replace", "Ctrl+H"), ("Settings", "Ctrl+,"), ("Close tab", "Ctrl+W"), ("Undo / redo", "Ctrl+Z / Ctrl+Shift+Z"), ("Bold / italic / underline", "Ctrl+B / I / U"), ("Navigate blocks", "Arrow keys"), ("Indent / outdent", "Tab / Shift+Tab"), ("Expand writing", "F11")] { form.row(action, key, egui::WidgetType::Label, |ui| ui.label(key)); },
                        7 => self.about_settings(&mut form, ctx),
                        8 => self.proofing_settings(&mut form),
                        _ => {}
                    }
                    if form.shown > 0 && query.is_empty() && section < 6 { form.ui.add_space(24.0); form.ui.separator(); }
                    if section < 6 && form.button("Reset this category", "Reset preferences for this category") { self.reset_settings_section(section); }
                    shown += form.shown;
                    if form.shown > 0 { form.ui.add_space(32.0); }
                }
                if shown == 0 { ui.label("No settings match. Try a control name, such as Spelling color."); }
            });
        });
        if before != self.settings {
            if before.proofing != self.settings.proofing {
                self.proofreader.invalidate();
            }
            self.apply_settings(ctx);
            if let Err(error) = self.settings.save(&self.workspace.directory) {
                self.notice = Some(format!("Could not save settings. Details: {error}"));
            }
        }
    }
    fn reset_settings_section(&mut self, section: usize) {
        let defaults = settings::Settings::default();
        match section {
            0 => self.settings.general = defaults.general,
            1 => self.settings.appearance = defaults.appearance,
            2 => self.settings.editor = defaults.editor,
            3 => self.settings.saving = defaults.saving,
            4 => self.settings.citations = defaults.citations,
            5 => self.settings.export = defaults.export,
            _ => {}
        }
    }
    fn proofing_settings(&mut self, form: &mut Form<'_>) {
        let settings = &mut self.settings.proofing;
        form.checkbox(
            "Check spelling",
            "English (US) · potential spelling issues",
            &mut settings.spelling,
        );
        form.checkbox(
            "Check grammar",
            "English (US) · suggestions, not automatic corrections",
            &mut settings.grammar,
        );
        form.checkbox(
            "Highlight only, no suggestions",
            "Find mistakes without receiving corrections",
            &mut settings.highlight_only,
        );
        form.checkbox(
            "Check as you type",
            "Background checking of changed text",
            &mut settings.as_you_type,
        );
        form.row(
            "Debounce (seconds)",
            "Delay before checking changed text",
            egui::WidgetType::Slider,
            |ui| ui.add(egui::Slider::new(&mut settings.debounce, 0.2..=2.0)),
        );
        form.checkbox(
            "Dotted underlines (otherwise wavy)",
            "Quiet proofing marks",
            &mut settings.dotted,
        );
        form.slider(
            "Underline opacity",
            "Subtle but readable marks",
            &mut settings.opacity,
            0.3..=1.0,
        );
        form.row(
            "Spelling color",
            "Color of spelling underlines",
            egui::WidgetType::ColorButton,
            |ui| {
                ui.color_edit_button_srgb(&mut settings.spelling_color)
                    .hand()
            },
        );
        form.row(
            "Grammar color",
            "Color of grammar underlines",
            egui::WidgetType::ColorButton,
            |ui| {
                ui.color_edit_button_srgb(&mut settings.grammar_color)
                    .hand()
            },
        );
        form.checkbox(
            "Skip ALL-CAPS words",
            "Do not flag abbreviations",
            &mut settings.skip_caps,
        );
        form.checkbox(
            "Skip words with numbers",
            "Skip mixed word and number tokens",
            &mut settings.skip_numbers,
        );
        for (i, label) in [
            "Grammar",
            "Punctuation",
            "Typography",
            "Redundancy",
            "Capitalization",
            "Style",
        ]
        .iter()
        .enumerate()
        {
            form.checkbox(
                label,
                "Grammar rule categories",
                &mut settings.categories[i],
            );
        }
        form.row(
            "Proofing language",
            "Links, code, headings and citations are skipped",
            egui::WidgetType::Label,
            |ui| ui.label("English (US) · offline"),
        );
        form.text(
            "Add a dictionary word",
            "Keep intentional names and accented words",
            &mut self.proofreader.dictionary_input,
        );
        if form.button(
            "Add to dictionary",
            "Keep intentional words without turning checking off",
        ) {
            let word = self.proofreader.dictionary_input.trim();
            if !word.is_empty()
                && !settings
                    .dictionary
                    .iter()
                    .any(|w| w.eq_ignore_ascii_case(word))
            {
                settings.dictionary.push(word.into());
            }
            self.proofreader.dictionary_input.clear();
        }
        let mut remove = None;
        for (i, word) in settings.dictionary.iter().enumerate() {
            if form
                .row(
                    &format!("Dictionary: {word}"),
                    "Remove from the custom dictionary",
                    egui::WidgetType::Button,
                    |ui| icons::danger(ui, "Remove"),
                )
                .is_some_and(|r| r.clicked())
            {
                remove = Some(i);
            }
        }
        if let Some(i) = remove {
            settings.dictionary.remove(i);
        }
        if form.button("Import dictionary .txt", "Import one word per line") {
            if let Some(path) = FileDialog::new()
                .add_filter("Word list", &["txt"])
                .pick_file()
            {
                match std::fs::read_to_string(&path) {
                    Ok(text) => {
                        for word in text.lines().map(str::trim).filter(|w| !w.is_empty()) {
                            if !settings
                                .dictionary
                                .iter()
                                .any(|w| w.eq_ignore_ascii_case(word))
                            {
                                settings.dictionary.push(word.into());
                            }
                        }
                    }
                    Err(error) => {
                        self.notice = Some(format!(
                            "The dictionary could not be read. Details: {error}"
                        ))
                    }
                }
            }
        }
        if form.button("Export dictionary .txt", "Save the custom word list") {
            if let Some(path) = FileDialog::new()
                .set_file_name("flowrite-dictionary.txt")
                .save_file()
            {
                if let Err(error) = std::fs::write(path, settings.dictionary.join("\n")) {
                    self.notice = Some(format!(
                        "The dictionary could not be saved. Details: {error}"
                    ));
                }
            }
        }
        for (label, list) in [
            ("Ignored words", &mut settings.ignored_words),
            ("Ignored rules", &mut settings.ignored_rules),
        ] {
            let mut remove = None;
            for (i, word) in list.iter().enumerate() {
                if form
                    .row(
                        &format!("{label}: {word}"),
                        "Remove this ignored entry",
                        egui::WidgetType::Button,
                        |ui| icons::danger(ui, "Remove"),
                    )
                    .is_some_and(|r| r.clicked())
                {
                    remove = Some(i);
                }
            }
            if let Some(i) = remove {
                list.remove(i);
            }
        }
        if form.button(
            "Reset proofing to default",
            "Reset proofing preferences and custom lists",
        ) {
            *settings = settings::Proofing::default();
        }
        let status = if let Some(error) = &self.proofreader.error {
            format!("Could not start: {error}")
        } else if self.proofreader.ready {
            "Ready · checking stays on this computer".into()
        } else {
            "Loading in the background…".into()
        };
        form.row(
            "Offline proofing status",
            "No writing is sent to a service",
            egui::WidgetType::Label,
            |ui| ui.label(status),
        );
    }
    fn about_settings(&mut self, form: &mut Form<'_>, ctx: &egui::Context) {
        form.row(
            "Version",
            "Flowrite release version",
            egui::WidgetType::Label,
            |ui| ui.label(env!("CARGO_PKG_VERSION")),
        );
        form.row(
            "GitHub repository",
            "Source code",
            egui::WidgetType::Link,
            |ui| {
                ui.hyperlink_to("View repository", "https://github.com/CoolDev09/flowrite")
                    .hand()
            },
        );
        form.row(
            "Releases",
            "Windows downloads",
            egui::WidgetType::Link,
            |ui| {
                ui.hyperlink_to(
                    "View releases",
                    "https://github.com/CoolDev09/flowrite/releases",
                )
                .hand()
            },
        );
        form.row(
            "Privacy",
            "Local documents and offline proofing",
            egui::WidgetType::Label,
            |ui| ui.label("No cloud sync or runtime network requests."),
        );
        if form.button(
            "Copy diagnostic info",
            "Version, OS, page count and library path",
        ) {
            ctx.copy_text(format!(
                "flowrite {}\nOS: {}\nDocuments: {}\nLibrary: {}",
                env!("CARGO_PKG_VERSION"),
                std::env::consts::OS,
                self.workspace.documents.len(),
                self.workspace.directory.display()
            ));
        }
        if form.button(
            "Save all dependency license notices…",
            "Export third-party notices",
        ) {
            if let Some(path) = FileDialog::new()
                .set_file_name("flowrite-third-party-licenses.txt")
                .save_file()
            {
                match std::fs::write(
                    &path,
                    include_str!(concat!(env!("OUT_DIR"), "/third-party-licenses.txt")),
                ) {
                    Ok(()) => open_path(&path),
                    Err(error) => {
                        self.notice = Some(format!(
                            "License notices could not be saved. Details: {error}"
                        ))
                    }
                }
            }
        }
        for (label, text) in [
            (
                "Font licenses · SIL Open Font License 1.1",
                include_str!("../assets/fonts/LICENSE"),
            ),
            (
                "Grammar data · LGPL-2.1 and source",
                include_str!("../assets/nlprule/README.md"),
            ),
            (
                "Grammar data · full LGPL-2.1 license",
                include_str!("../assets/nlprule/COPYING.txt"),
            ),
            (
                "US English dictionary license and credits",
                include_str!("../assets/spelling/README_en_US.txt"),
            ),
            ("Icon license · Uicons by Flaticon", icons::LICENSE),
        ] {
            form.row(
                label,
                "Bundled licenses, credits and source information",
                egui::WidgetType::CollapsingHeader,
                |ui| {
                    egui::CollapsingHeader::new("Read license and credits")
                        .show(ui, |ui| {
                            ui.label(text);
                        })
                        .header_response
                },
            );
        }
        form.row(
            "Icon source",
            "Flaticon Uicons Regular Rounded 3.3.1",
            egui::WidgetType::Link,
            |ui| {
                ui.hyperlink_to(
                    "Uicons by Flaticon",
                    "https://www.flaticon.com/uicons/interface-icons",
                )
                .hand()
            },
        );
    }
}
pub(super) fn open_path(path: &Path) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer.exe").arg(path).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn form_controls_stay_inside_the_minimum_window_at_every_scale() {
        for scale in [1.0, 1.25, 1.5] {
            for section in 0..9 {
                let ctx = egui::Context::default();
                configure_fonts(&ctx);
                let mut app = FlowriteApp::empty();
                app.settings_section = section;
                app.settings.appearance.scale = scale;
                app.apply_settings(&ctx);
                for _ in 0..3 {
                    ctx.data_mut(|d| {
                        d.remove::<Vec<(String, egui::Rect)>>(egui::Id::new("settings_bounds"))
                    });
                    let _ = ctx.run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                Pos2::ZERO,
                                Vec2::new(900.0, 600.0) / scale,
                            )),
                            ..Default::default()
                        },
                        |ctx| app.settings_page(ctx),
                    );
                }
                for (label, rect) in ctx
                    .data(|d| {
                        d.get_temp::<Vec<(String, egui::Rect)>>(egui::Id::new("settings_bounds"))
                    })
                    .unwrap_or_default()
                {
                    assert!(
                        rect.right() <= ctx.screen_rect().right() + 1.0,
                        "{label} clips horizontally at {scale}: {rect:?}"
                    );
                }
            }
        }
    }
    #[test]
    fn search_indexes_actual_labels_and_matches_multiple_words() {
        let ctx = egui::Context::default();
        configure_fonts(&ctx);
        let mut app = FlowriteApp::empty();
        app.settings_open = true;
        app.settings_query = "spelling color".into();
        let output = ctx.run(egui::RawInput::default(), |ctx| app.settings_page(ctx));
        let painted: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| {
                if let egui::Shape::Text(t) = &s.shape {
                    Some(t.galley.job.text.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert!(painted.iter().any(|&text| text == "Spelling color"));
        assert!(!painted
            .iter()
            .any(|text| text.contains("No settings match")));
        let catalog = ctx
            .data(|data| {
                data.get_temp::<Vec<(String, String, String)>>(egui::Id::new("settings_catalog"))
            })
            .unwrap();
        assert!(catalog.len() > 65);
        for (section, label, description) in &catalog {
            assert!(
                setting_matches(label, section, label, description),
                "Unsearchable setting: {label}"
            );
        }
        assert!(setting_matches(
            "opacity underline",
            "Writing & proofing",
            "Underline opacity",
            ""
        ));
    }
}
