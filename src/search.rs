use super::*;

#[derive(Default)]
pub(super) struct Search {
    pub replacement: String,
    pub replace: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub current: usize,
}

// Offsets are Unicode character offsets, not UTF-8 bytes. Case folding may
// change byte length, so compare character slices without translating offsets.
pub(super) fn ranges(text: &str, query: &str, case: bool, whole: bool) -> Vec<(usize, usize)> {
    let chars: Vec<_> = text.chars().collect();
    let needle: Vec<_> = query.chars().collect();
    if needle.is_empty() {
        return Vec::new();
    }
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut found = Vec::new();
    let mut i = 0;
    while i + needle.len() <= chars.len() {
        let end = i + needle.len();
        let equal = chars[i..end].iter().zip(&needle).all(|(a, b)| {
            if case {
                a == b
            } else {
                a.to_lowercase().eq(b.to_lowercase())
            }
        });
        if equal
            && (!whole
                || ((i == 0 || !word(chars[i - 1])) && (end == chars.len() || !word(chars[end]))))
        {
            found.push((i, end));
            i = end;
        } else {
            i += 1;
        }
    }
    found
}

impl FlowriteApp {
    pub(super) fn search_matches(&self) -> Vec<(usize, usize, usize)> {
        self.blocks
            .iter()
            .enumerate()
            .flat_map(|(i, b)| {
                ranges(
                    &b.text,
                    &self.find_query,
                    self.search.case_sensitive,
                    self.search.whole_word,
                )
                .into_iter()
                .map(move |(start, end)| (i, start, end))
            })
            .collect()
    }
    pub(super) fn next_match(&mut self, backwards: bool) {
        let matches = self.search_matches();
        if matches.is_empty() {
            return;
        }
        self.search.current = if backwards {
            (self.search.current + matches.len() - 1) % matches.len()
        } else {
            (self.search.current + 1) % matches.len()
        };
        let (i, start, end) = matches[self.search.current];
        let id = self.blocks[i].id;
        self.scroll_target = Some(id);
        self.selection = Some((id, start, end));
    }
    fn replace_matches(&mut self, all: bool) {
        let matches = self.search_matches();
        let chosen = if all {
            matches
        } else {
            matches
                .get(self.search.current)
                .copied()
                .into_iter()
                .collect()
        };
        for (i, start, end) in chosen.into_iter().rev() {
            let block = &mut self.blocks[i];
            let chars: Vec<_> = block.text.chars().collect();
            let mut styles = block.character_styles();
            let inherited = styles.get(start).copied().unwrap_or(block.format);
            block.text = chars[..start].iter().collect::<String>()
                + &self.search.replacement
                + &chars[end..].iter().collect::<String>();
            styles.splice(
                start..end,
                std::iter::repeat_n(inherited, self.search.replacement.chars().count()),
            );
            block.set_character_styles(&styles);
            self.dirty = true;
            self.force_history = true;
        }
        self.search.current = self
            .search
            .current
            .min(self.search_matches().len().saturating_sub(1));
    }
    pub(super) fn find_bar(&mut self, ctx: &egui::Context) {
        if !self.find_open {
            return;
        }
        egui::TopBottomPanel::top("find_bar")
            .frame(
                Frame::none()
                    .fill(theme::current().panel)
                    .inner_margin(Margin::symmetric(22.0, 7.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Find");
                    let edit = ui.add(
                        TextEdit::singleline(&mut self.find_query)
                            .id(egui::Id::new("find_query"))
                            .hint_text("Find in this document…")
                            .desired_width(230.0),
                    );
                    if edit.changed() {
                        self.search.current = 0;
                    }
                    ui.checkbox(&mut self.search.case_sensitive, "Match case")
                        .hand();
                    ui.checkbox(&mut self.search.whole_word, "Whole word")
                        .hand();
                    let count = self.search_matches().len();
                    ui.label(format!(
                        "{} of {count}",
                        if count == 0 {
                            0
                        } else {
                            self.search.current.min(count - 1) + 1
                        }
                    ));
                    if ui
                        .add_enabled(count > 0, egui::Button::new("Previous"))
                        .hand()
                        .clicked()
                    {
                        self.next_match(true);
                    }
                    if ui
                        .add_enabled(count > 0, egui::Button::new("Next"))
                        .hand()
                        .clicked()
                        || (edit.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        self.next_match(ui.input(|i| i.modifiers.shift));
                    }
                    if ui
                        .selectable_label(self.search.replace, "Replace")
                        .hand()
                        .clicked()
                    {
                        self.search.replace = !self.search.replace;
                    }
                    if ui.small_button("Close").hand().clicked() {
                        self.find_open = false;
                    }
                });
                if self.search.replace {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Replace with");
                        ui.add(
                            TextEdit::singleline(&mut self.search.replacement).desired_width(230.0),
                        );
                        if ui.button("Replace").hand().clicked() {
                            self.replace_matches(false);
                        }
                        if ui.button("Replace all").hand().clicked() {
                            self.replace_matches(true);
                        }
                    });
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_unicode_matches_and_word_boundaries() {
        assert_eq!(
            ranges("é Essay essay essays", "essay", false, true),
            vec![(2, 7), (8, 13)]
        );
        assert_eq!(ranges("é Essay essay", "essay", true, false), vec![(8, 13)]);
        assert!(ranges("text", "", false, false).is_empty());
    }
    #[test]
    fn replacement_preserves_surrounding_styles() {
        let mut app = FlowriteApp::empty();
        app.blocks[0].text = "é cat cat".into();
        app.blocks[0].format.bold = true;
        app.find_query = "cat".into();
        app.search.replacement = "dog 🐕".into();
        app.replace_matches(true);
        assert_eq!(app.blocks[0].text, "é dog 🐕 dog 🐕");
        assert!(app.blocks[0].character_styles().iter().all(|s| s.bold));
        assert!(app.dirty);
    }
}
