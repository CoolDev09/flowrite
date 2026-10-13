use super::*;
impl FlowriteApp {
    pub(super) fn proofing_review(&mut self, ctx: &egui::Context) {
        if !self.proofing_open || self.show_home || self.settings_open {
            return;
        }
        let issues: Vec<_> = self
            .blocks
            .iter()
            .enumerate()
            .flat_map(|(index, block)| {
                self.proofreader
                    .issues
                    .get(&block.id)
                    .filter(|(text, _)| text == &block.text)
                    .into_iter()
                    .flat_map(move |(_, issues)| {
                        issues
                            .iter()
                            .enumerate()
                            .map(move |(number, issue)| (index, number, issue.clone()))
                    })
            })
            .collect();
        let mut open = self.proofing_open;
        let mut replace = None;
        let mut ignore = None;
        let mut dictionary = None;
        let mut focus = None;
        egui::Window::new("Writing review").open(&mut open).collapsible(false).default_width(340.0).default_height(380.0).max_height((ctx.screen_rect().height() - 100.0).max(200.0))
            .anchor(egui::Align2::RIGHT_TOP, Vec2::new(-16.0, 64.0)).show(ctx, |ui| {
                ui.label(RichText::new(format!("English (US) · {} potential issues · offline", issues.len())).color(theme::current().muted));
                ui.label("Suggestions are optional. Keep intentional words with Ignore or Add to dictionary.");
                if ui.button("Proofing settings").hand().clicked() { self.settings_section = 8; self.settings_open = true; }
                ui.separator();
                egui::ScrollArea::vertical().max_height((ctx.screen_rect().height() - 240.0).max(100.0)).show(ui, |ui| {
                    if issues.is_empty() { ui.label(if self.proofreader.ready { "No issues in the checked paragraphs." } else { "Proofing is loading…" }); }
                    for (index, number, issue) in &issues {
                        ui.push_id((*index, *number), |ui| {
                            ui.label(RichText::new(if issue.spelling { "Spelling · possible mistake" } else { "Grammar · suggestion" }).color(if issue.spelling { Color32::from_rgb(235,107,107) } else { Color32::from_rgb(113,174,232) }));
                            if ui.link(&issue.word).hand().clicked() { focus = Some((*index, *number, issue.start)); }
                            if !self.settings.proofing.highlight_only {
                                ui.label(&issue.message);
                                ui.horizontal_wrapped(|ui| { for value in &issue.replacements { if ui.button(value).hand().clicked() { replace = Some((*index, issue.start, issue.end, value.clone())); } } });
                            }
                            ui.horizontal_wrapped(|ui| {
                                if ui.button("Ignore").hand().clicked() { ignore = Some(issue.word.clone()); }
                                if issue.spelling && ui.button("Add to dictionary").hand().clicked() { dictionary = Some(issue.word.clone()); }
                            });
                            ui.add_space(12.0); ui.separator();
                        });
                    }
                });
            });
        self.proofing_open = open;
        if let Some((index, number, cursor)) = focus {
            let id = self.blocks[index].id;
            self.scroll_target = Some(id);
            self.focus_request = Some((id, cursor));
            self.proofreader.target = Some((id, number));
        }
        if let Some((index, start, end, value)) = replace {
            self.insert_at(index, start, end, &value);
            self.proofreader.invalidate();
        }
        if let Some(word) = ignore {
            self.settings.proofing.ignored_words.push(word);
            self.proofreader.invalidate();
            let _ = self.settings.save(&self.workspace.directory);
        }
        if let Some(word) = dictionary {
            self.settings.proofing.dictionary.push(word);
            self.proofreader.invalidate();
            let _ = self.settings.save(&self.workspace.directory);
        }
    }
}
