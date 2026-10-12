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
        egui::TopBottomPanel::top("settings_header")
            .exact_height(64.0)
            .frame(
                Frame::none()
                    .fill(self.theme.canvas)
                    .inner_margin(Margin::symmetric(24.0, 12.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    self.brand_button(ui);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::control(ui, Icon::Check, "Done", "Done · return to your workspace", true, false)
                            .clicked()
                        {
                            self.settings_open = false;
                        }
                    });
                });
            });
        let before = self.settings.clone();
        egui::SidePanel::left("settings_categories")
            .exact_width(220.0)
            .frame(
                Frame::none()
                    .fill(self.theme.panel)
                    .inner_margin(Margin::same(20.0)),
            )
            .show(ctx, |ui| {
                ui.heading("Settings");
                ui.add_space(18.0);
                ui.add(
                    TextEdit::singleline(&mut self.settings_query)
                        .hint_text("Search settings…")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(18.0);
                for (i, name) in SECTIONS.iter().enumerate() {
                    if ui
                        .selectable_label(i == self.settings_section, *name)
                        .hand()
                        .clicked()
                    {
                        self.settings_section = i;
                        self.settings_query.clear();
                    }
                }
                ui.add_space(24.0);
                if ui.button("Back to document").hand().clicked() {
                    self.settings_open = false;
                }
            });
        egui::CentralPanel::default().frame(Frame::none().fill(self.theme.canvas).inner_margin(Margin::same(32.0))).show(ctx,|ui| {
            egui::ScrollArea::vertical().show(ui,|ui| {
                let query=self.settings_query.to_lowercase();
                let mut shown=false;
                for (section,name) in SECTIONS.iter().enumerate() {
                    let keywords=["startup restore save folder date name author instructor course cleanup reset","dark dim accent scale size grid list recent thumbnails sidebar contrast pointer cursor","font size spacing margin markdown slash handles words characters reading goal","autosave delay saved library location","works cited references heading alphabetical hanging indent running header","toast duration export folder auto open","shortcuts keyboard ctrl save undo redo find replace new close","about privacy fonts licenses diagnostic version github local data","writing proofing spelling grammar suggestion highlight underline dictionary rules language"];
                    if query.is_empty() {if section!=self.settings_section {continue;}}else if !keywords[section].contains(&query)&&!name.to_lowercase().contains(&query) {continue;}
                    shown=true;ui.heading(*name);ui.add_space(20.0);
                    match section {
                        0=>{
                            ui.checkbox(&mut self.settings.general.restore_tabs,"Restore last open tabs on startup").hand();
                            ui.checkbox(&mut self.settings.general.ask_export,"Ask where to save every export").hand();
                            ui.horizontal(|ui| {ui.label("Default export folder");ui.label(self.settings.general.folder.display().to_string());if ui.button("Choose…").hand().clicked(){if let Some(path)=FileDialog::new().pick_folder(){self.settings.general.folder=path;}}});
                            egui::ComboBox::from_label("Date format").selected_text(["10 October 2026","October 10, 2026","10/10/2026"][self.settings.general.date_format.min(2)]).show_ui(ui,|ui|{for (i,label) in ["10 October 2026","October 10, 2026","10/10/2026"].iter().enumerate(){ui.selectable_value(&mut self.settings.general.date_format,i,*label).hand();}}).response.hand();
                            field(ui,"Name",&mut self.settings.general.author);field(ui,"Instructor",&mut self.settings.general.instructor);field(ui,"Course",&mut self.settings.general.course);
                            ui.add_space(20.0);
                            if icons::danger(ui, "Delete all empty pages…").clicked(){self.dialogs.action=Some(context_menu::PageAction::Cleanup);}
                            if ui.button(RichText::new("Reset all settings…").color(Color32::from_rgb(235,112,112))).hand().clicked(){self.dialogs.action=Some(context_menu::PageAction::ResetSettings);}
                        },
                        1=>{
                            ui.label("Dark mode");ui.checkbox(&mut self.settings.appearance.dim_page,"Dim page background").hand();
                            egui::ComboBox::from_label("Accent").selected_text(theme::ACCENTS[self.settings.appearance.accent.min(8)].0).show_ui(ui,|ui|{for (i,(name,_)) in theme::ACCENTS.iter().enumerate(){ui.selectable_value(&mut self.settings.appearance.accent,i,*name).hand();}}).response.hand();
                            egui::ComboBox::from_label("Interface size").selected_text(format!("{}%",(self.settings.appearance.scale*100.0) as i32)).show_ui(ui,|ui|{for scale in [0.9,1.0,1.1,1.25,1.5] {ui.selectable_value(&mut self.settings.appearance.scale,scale,format!("{}%",(scale*100.0) as i32)).hand();}}).response.hand();
                            ui.checkbox(&mut self.settings.appearance.list_view,"Recent documents as a list").hand();
                            ui.add(egui::Slider::new(&mut self.settings.appearance.recent_limit,1..=100).text("Recent documents")).hand();
                            ui.checkbox(&mut self.settings.appearance.thumbnails,"Show thumbnails").hand();
                            ui.add(egui::Slider::new(&mut self.settings.appearance.sidebar_width,180.0..=360.0).text("Sidebar width")).hand();
                            ui.checkbox(&mut self.settings.appearance.folders,"Show folders in sidebar").hand();
                            ui.checkbox(&mut self.settings.appearance.high_contrast,"High contrast").hand();ui.checkbox(&mut self.settings.appearance.hand_pointer,"Hand pointer on buttons").hand();
                        },
                        2=>{
                            egui::ComboBox::from_label("Default font").selected_text(&self.settings.editor.font).show_ui(ui,|ui|{for name in fonts::FAMILIES {ui.selectable_value(&mut self.settings.editor.font,name.to_owned(),name).hand();}}).response.hand();
                            ui.add(egui::Slider::new(&mut self.settings.editor.size,8.0..=32.0).text("Font size (pt)")).hand();
                            ui.add(egui::Slider::new(&mut self.settings.editor.spacing,1.0..=3.0).text("Line spacing")).hand();
                            ui.add(egui::Slider::new(&mut self.settings.editor.margin,0.25..=2.0).text("Margins (inches)")).hand();
                            ui.label("MLA and APA presets keep double spacing and a half-inch first-line indent.");
                            ui.checkbox(&mut self.settings.editor.markdown,"Markdown shortcuts").hand();ui.checkbox(&mut self.settings.editor.slash,"Slash menu").hand();ui.checkbox(&mut self.settings.editor.handles,"Show block handles").hand();
                            ui.checkbox(&mut self.settings.editor.words,"Word count").hand();ui.checkbox(&mut self.settings.editor.characters,"Character count").hand();ui.checkbox(&mut self.settings.editor.reading_time,"Reading time").hand();
                            ui.horizontal(|ui|{ui.label("Word goal (0 disables)");ui.add(egui::DragValue::new(&mut self.settings.editor.word_goal).range(0..=100_000));});
                        },
                        3=>{
                            ui.add(egui::Slider::new(&mut self.settings.saving.delay,0.3..=10.0).text("Autosave delay (seconds)")).hand();ui.checkbox(&mut self.settings.saving.indicator,"Show Saved indicator").hand();
                            ui.add_space(16.0);ui.label("Library location");ui.label(self.workspace.directory.display().to_string());
                            if ui.button("Open library folder").hand().clicked(){open_path(&self.workspace.directory);}
                            if !self.workspace_available&&ui.button("Library recovery…").hand().clicked(){self.recovery.open=true;}
                            ui.label("Each document is saved independently, with a previous-version .bak safety copy.");
                        },
                        4=>{
                            field(ui,"Citation section title (blank uses preset)",&mut self.settings.citations.heading);
                            ui.checkbox(&mut self.settings.citations.alphabetize,"Alphabetize citations").hand();
                            ui.add(egui::Slider::new(&mut self.settings.citations.hanging_inches,0.0..=1.0).text("Hanging indent (inches)")).hand();
                            ui.checkbox(&mut self.settings.citations.running_header,"Running name and page number").hand();
                        },
                        5=>{
                            ui.checkbox(&mut self.settings.export.a4,"PDF paper: A4 (otherwise Letter)").hand();
                            ui.checkbox(&mut self.settings.export.page_colors,"PDF uses page colors (otherwise black on white)").hand();
                            ui.checkbox(&mut self.settings.export.include_images,"Include images").hand();
                            ui.checkbox(&mut self.settings.export.page_numbers,"Include academic page numbers").hand();
                            ui.checkbox(&mut self.settings.export.toast,"Show export notification").hand();ui.add(egui::Slider::new(&mut self.settings.export.toast_seconds,1.0..=15.0).text("Notification duration (seconds)")).hand();ui.checkbox(&mut self.settings.export.auto_open,"Open file after export").hand();
                            ui.label("PDF embeds the same open-licensed fonts used in the editor. Markdown preserves document links, lists, and text formatting.");
                        },
                        6=>{for (action,key) in [("New page","Ctrl+N"),("New folder","Ctrl+Shift+N"),("Save","Ctrl+S"),("Save a copy","Ctrl+Shift+S"),("Find","Ctrl+F"),("Replace","Ctrl+H"),("Settings","Ctrl+,"),("Close tab","Ctrl+W"),("Undo / redo","Ctrl+Z / Ctrl+Shift+Z"),("Bold / italic / underline","Ctrl+B / I / U"),("Navigate blocks","Arrow keys")] {ui.horizontal(|ui|{ui.label(action);ui.label(RichText::new(key).color(theme::current().muted));});}},
                        8=>self.proofing_settings(ui),
                        _=>{
                            ui.heading("flowrite");ui.label(format!("Version {}",env!("CARGO_PKG_VERSION")));
                            ui.hyperlink_to("GitHub repository","https://github.com/CoolDev09/flowrite").hand();ui.hyperlink_to("Releases","https://github.com/CoolDev09/flowrite/releases").hand();
                            ui.add_space(16.0);ui.label("All documents stay on this computer. This version makes no network requests.");
                            if ui.button("Copy diagnostic info").hand().clicked(){ctx.copy_text(format!("flowrite {}\nOS: {}\nDocuments: {}\nLibrary: {}",env!("CARGO_PKG_VERSION"),std::env::consts::OS,self.workspace.documents.len(),self.workspace.directory.display()));}
                            ui.add_space(16.0);ui.heading("Font licenses");ui.label("Liberation Serif, Sans, and Mono — SIL Open Font License 1.1");
                            egui::CollapsingHeader::new("Read bundled font license").show(ui,|ui|{ui.label(include_str!("../assets/fonts/LICENSE"));});
                            ui.label("egui / eframe — MIT or Apache-2.0; printpdf — MIT; serde — MIT or Apache-2.0; rfd — MIT.");
                            if ui.button("Save all dependency license notices…").hand().clicked(){if let Some(path)=FileDialog::new().set_file_name("flowrite-third-party-licenses.txt").save_file(){match std::fs::write(&path,include_str!(concat!(env!("OUT_DIR"),"/third-party-licenses.txt"))){Ok(())=>open_path(&path),Err(error)=>self.notice=Some(format!("Could not save license notices: {error}"))}}}
                            egui::CollapsingHeader::new("LanguageTool grammar data — LGPL-2.1").show(ui,|ui|{ui.label(include_str!("../assets/nlprule/README.md"));ui.label(include_str!("../assets/nlprule/COPYING.txt"));});
                            egui::CollapsingHeader::new("US English dictionary license and credits").show(ui,|ui|{ui.label(include_str!("../assets/spelling/README_en_US.txt"));});
                            ui.add_space(16.0);ui.heading("Icon credits");ui.hyperlink_to("Uicons by Flaticon — Regular Rounded", "https://www.flaticon.com/uicons/interface-icons").hand();ui.label("Official package 3.3.1 • SVG derivatives embedded for offline use");egui::CollapsingHeader::new("Read icon license").show(ui,|ui|{ui.label(icons::LICENSE);});
                        }
                    }
                    ui.add_space(24.0);
                    if section<6 && ui.button("Reset section to default").hand().clicked(){let defaults=settings::Settings::default();match section {0=>self.settings.general=defaults.general,1=>self.settings.appearance=defaults.appearance,2=>self.settings.editor=defaults.editor,3=>self.settings.saving=defaults.saving,4=>self.settings.citations=defaults.citations,5=>self.settings.export=defaults.export,_=>{}}}
                    ui.add_space(30.0);
                }
                if !shown {ui.label("No settings match your search.");}
            });
        });
        if before != self.settings {
            if before.proofing != self.settings.proofing {
                self.proofreader.invalidate();
            }
            self.apply_settings(ctx);
            if let Err(error) = self.settings.save(&self.workspace.directory) {
                self.notice = Some(format!("Could not save settings: {error}"));
            }
        }
    }
}
impl FlowriteApp {
    fn proofing_settings(&mut self, ui: &mut egui::Ui) {
        let settings = &mut self.settings.proofing;
        ui.checkbox(&mut settings.spelling, "Check spelling").hand();
        ui.checkbox(&mut settings.grammar, "Check grammar").hand();
        ui.radio_value(&mut settings.highlight_only, false, "Show suggestions")
            .hand();
        ui.radio_value(
            &mut settings.highlight_only,
            true,
            "Highlight only, no suggestions",
        )
        .hand();
        ui.label("Useful for homework where you want to find and fix mistakes yourself.");
        ui.checkbox(&mut settings.as_you_type, "Check as you type")
            .hand();
        ui.add(egui::Slider::new(&mut settings.debounce, 0.2..=2.0).text("Debounce (seconds)"))
            .hand();
        ui.checkbox(&mut settings.dotted, "Dotted underlines (otherwise wavy)")
            .hand();
        ui.add(egui::Slider::new(&mut settings.opacity, 0.3..=1.0).text("Underline opacity"))
            .hand();
        ui.horizontal(|ui| {
            ui.label("Spelling color");
            ui.color_edit_button_srgb(&mut settings.spelling_color)
                .hand();
            ui.label("Grammar color");
            ui.color_edit_button_srgb(&mut settings.grammar_color)
                .hand();
        });
        ui.checkbox(&mut settings.skip_caps, "Skip ALL-CAPS words")
            .hand();
        ui.checkbox(&mut settings.skip_numbers, "Skip words with numbers")
            .hand();
        ui.label("URLs, document links, code blocks, headings, and citations are skipped.");
        ui.add_space(12.0);
        ui.label("Rule categories");
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
            ui.checkbox(&mut settings.categories[i], *label).hand();
        }
        ui.add_space(12.0);
        ui.label("English (US) · more languages are planned");
        ui.add_space(18.0);
        ui.heading("Custom dictionary");
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut self.proofreader.dictionary_input)
                    .hint_text("Add a word"),
            );
            if ui.button("Add").hand().clicked() {
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
        });
        let mut remove = None;
        for (i, word) in settings.dictionary.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(word);
                if icons::danger(ui, "Remove").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            settings.dictionary.remove(i);
        }
        ui.horizontal(|ui| {
            if ui.button("Import .txt").hand().clicked() {
                if let Some(path) = FileDialog::new()
                    .add_filter("Word list", &["txt"])
                    .pick_file()
                {
                    if let Ok(text) = std::fs::read_to_string(path) {
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
                }
            }
            if ui.button("Export .txt").hand().clicked() {
                if let Some(path) = FileDialog::new()
                    .set_file_name("flowrite-dictionary.txt")
                    .save_file()
                {
                    let _ = std::fs::write(path, settings.dictionary.join("\n"));
                }
            }
        });
        for (label, list) in [
            ("Ignored words", &mut settings.ignored_words),
            ("Ignored rules", &mut settings.ignored_rules),
        ] {
            ui.add_space(12.0);
            ui.label(label);
            let mut remove = None;
            for (i, word) in list.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(word);
                    if icons::danger(ui, "Remove").clicked() {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                list.remove(i);
            }
        }
        ui.add_space(12.0);
        if ui.button("Reset proofing to default").hand().clicked() {
            *settings = settings::Proofing::default();
        }
        if let Some(error) = &self.proofreader.error {
            ui.label(format!("Proofing could not start: {error}"));
        } else {
            ui.label(if self.proofreader.ready {
                "Offline proofing is ready."
            } else {
                "Loading offline proofing in the background…"
            });
        }
    }
}
fn field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(TextEdit::singleline(value).desired_width(300.0));
    });
}
pub(super) fn open_path(path: &Path) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer.exe").arg(path).spawn();
    }
}
