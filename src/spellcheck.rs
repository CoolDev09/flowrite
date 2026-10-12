use super::*;
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
#[derive(Clone, Debug)]
pub(super) struct Issue {
    pub start: usize,
    pub end: usize,
    pub spelling: bool,
    pub source: String,
    pub message: String,
    pub replacements: Vec<String>,
    pub word: String,
}
struct Request {
    document: String,
    id: u64,
    text: String,
    settings: settings::Proofing,
}
enum ResultMessage {
    Ready(Result<(), String>),
    Checked(String, u64, String, Vec<Issue>),
}
#[derive(Default)]
pub(super) struct Proofreader {
    tx: Option<Sender<Request>>,
    rx: Option<Receiver<ResultMessage>>,
    pub ready: bool,
    pub error: Option<String>,
    document: String,
    checked: HashMap<u64, String>,
    pub issues: HashMap<u64, (String, Vec<Issue>)>,
    pub manual: bool,
    pub target: Option<(u64, usize)>,
    pub dictionary_input: String,
}
impl Proofreader {
    pub fn start(&mut self, ctx: &egui::Context) {
        let (tx, requests) = mpsc::channel::<Request>();
        let (responses, rx) = mpsc::channel();
        let ctx = ctx.clone();
        self.tx = Some(tx);
        self.rx = Some(rx);
        std::thread::spawn(move || {
            let loaded = (|| -> Result<_, String> {
                let tokenizer = nlprule::Tokenizer::from_reader(
                    &include_bytes!("../assets/nlprule/en_tokenizer.bin")[..],
                )
                .map_err(|e| e.to_string())?;
                let rules = nlprule::Rules::from_reader(
                    &include_bytes!("../assets/nlprule/en_rules.bin")[..],
                )
                .map_err(|e| e.to_string())?;
                let dictionary = spellbook::Dictionary::new(
                    include_str!("../assets/spelling/en_US.aff"),
                    include_str!("../assets/spelling/en_US.dic"),
                )
                .map_err(|e| e.to_string())?;
                Ok((tokenizer, rules, dictionary))
            })();
            let (tokenizer, rules, dictionary) = match loaded {
                Ok(engine) => {
                    let _ = responses.send(ResultMessage::Ready(Ok(())));
                    ctx.request_repaint();
                    engine
                }
                Err(error) => {
                    let _ = responses.send(ResultMessage::Ready(Err(error)));
                    ctx.request_repaint();
                    return;
                }
            };
            let categories: HashMap<_, _> = rules
                .rules()
                .iter()
                .map(|r| {
                    (
                        r.id().to_string(),
                        category(r.category_type().unwrap_or(r.category_name())),
                    )
                })
                .collect();
            while let Ok(first) = requests.recv() {
                let mut latest = HashMap::new();
                latest.insert((first.document.clone(), first.id), first);
                for request in requests.try_iter() {
                    latest.insert((request.document.clone(), request.id), request);
                }
                for (_, request) in latest {
                    let mut issues = Vec::new();
                    let chars: Vec<_> = request.text.chars().collect();
                    let settings = &request.settings;
                    let skip = |word: &str| {
                        settings
                            .ignored_words
                            .iter()
                            .chain(&settings.dictionary)
                            .any(|w| w.eq_ignore_ascii_case(word))
                            || (settings.skip_caps
                                && word
                                    .chars()
                                    .filter(|c| c.is_alphabetic())
                                    .all(|c| c.is_uppercase()))
                            || (settings.skip_numbers && word.chars().any(|c| c.is_numeric()))
                    };
                    let ignored_spans = excluded(&request.text);
                    if settings.grammar {
                        for suggestion in rules.suggest(&request.text, &tokenizer) {
                            let span = suggestion.span().char();
                            let start = span.start;
                            let end = span.end;
                            if end > chars.len()
                                || start >= end
                                || ignored_spans.iter().any(|&(a, b)| start < b && end > a)
                                || settings
                                    .ignored_rules
                                    .iter()
                                    .any(|s| s == suggestion.source())
                                || !settings.categories
                                    [*categories.get(suggestion.source()).unwrap_or(&0)]
                            {
                                continue;
                            }
                            let word: String = chars[start..end].iter().collect();
                            if settings
                                .ignored_words
                                .iter()
                                .any(|w| w.eq_ignore_ascii_case(&word))
                            {
                                continue;
                            }
                            issues.push(Issue {
                                start,
                                end,
                                spelling: false,
                                source: suggestion.source().into(),
                                message: if settings.highlight_only {
                                    String::new()
                                } else {
                                    suggestion.message().into()
                                },
                                replacements: if settings.highlight_only {
                                    Vec::new()
                                } else {
                                    suggestion.replacements().iter().take(5).cloned().collect()
                                },
                                word,
                            });
                        }
                    }
                    if settings.spelling {
                        let mut start = 0;
                        while start < chars.len() {
                            if !chars[start].is_alphabetic() {
                                start += 1;
                                continue;
                            }
                            let mut end = start + 1;
                            while end < chars.len()
                                && (chars[end].is_alphanumeric()
                                    || chars[end] == '\''
                                    || chars[end] == '’')
                            {
                                end += 1;
                            }
                            let word: String = chars[start..end].iter().collect();
                            if !skip(&word)
                                && !ignored_spans.iter().any(|&(a, b)| start < b && end > a)
                                && !dictionary.check(&word)
                            {
                                let mut replacements = Vec::new();
                                if !settings.highlight_only {
                                    dictionary.suggest(&word, &mut replacements);
                                    replacements.truncate(5);
                                }
                                issues.push(Issue {
                                    start,
                                    end,
                                    spelling: true,
                                    source: "SPELLING".into(),
                                    message: if settings.highlight_only {
                                        String::new()
                                    } else {
                                        "Possible spelling mistake".into()
                                    },
                                    replacements,
                                    word,
                                });
                            }
                            start = end;
                        }
                    }
                    issues.sort_by_key(|i| i.start);
                    let _ = responses.send(ResultMessage::Checked(
                        request.document,
                        request.id,
                        request.text,
                        issues,
                    ));
                    ctx.request_repaint();
                }
            }
        });
    }
    pub fn invalidate(&mut self) {
        self.checked.clear();
        self.issues.clear();
    }
}
fn category(name: &str) -> usize {
    let name = name.to_lowercase();
    if name.contains("punctuation") {
        1
    } else if name.contains("typograph") {
        2
    } else if name.contains("redundan") {
        3
    } else if name.contains("capital") {
        4
    } else if name.contains("style") {
        5
    } else {
        0
    }
}
fn excluded(text: &str) -> Vec<(usize, usize)> {
    let mut spans: Vec<_> = wikilinks::links(text)
        .iter()
        .map(|l| (l.start, l.end))
        .collect();
    let mut offset = 0;
    for token in text.split_inclusive(char::is_whitespace) {
        let len = token.chars().count();
        if token.contains("://") || token.contains('@') || token.starts_with("www.") {
            spans.push((offset, offset + len));
        }
        offset += len;
    }
    spans
}
impl FlowriteApp {
    pub(super) fn update_proofing(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.proofreader.rx {
            for result in rx.try_iter() {
                match result {
                    ResultMessage::Ready(result) => {
                        self.proofreader.ready = result.is_ok();
                        self.proofreader.error = result.err();
                    }
                    ResultMessage::Checked(doc, id, text, issues) => {
                        if doc == self.active_document
                            && self.blocks.iter().any(|b| b.id == id && b.text == text)
                        {
                            self.proofreader.issues.insert(id, (text, issues));
                        }
                    }
                }
            }
        }
        if self.show_home || self.settings_open || !self.proofreader.ready {
            return;
        }
        if self.proofreader.document != self.active_document {
            self.proofreader.invalidate();
            self.proofreader.document = self.active_document.clone();
        }
        if !self.settings.proofing.as_you_type && !self.proofreader.manual {
            return;
        }
        if !self.proofreader.manual
            && ctx.input(|i| i.time) - self.last_document_change < self.settings.proofing.debounce
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        }
        for block in &self.blocks {
            if matches!(
                block.kind,
                BlockKind::Code | BlockKind::Divider | BlockKind::Heading1 | BlockKind::Heading2
            ) || block.text.is_empty()
            {
                continue;
            }
            if self.proofreader.checked.get(&block.id) == Some(&block.text) {
                continue;
            }
            if let Some(tx) = &self.proofreader.tx {
                let _ = tx.send(Request {
                    document: self.active_document.clone(),
                    id: block.id,
                    text: block.text.clone(),
                    settings: self.settings.proofing.clone(),
                });
                self.proofreader
                    .checked
                    .insert(block.id, block.text.clone());
            }
        }
        self.proofreader.manual = false;
    }
    pub(super) fn proofing_menu(&mut self, ui: &mut egui::Ui, index: usize) {
        let id = self.blocks[index].id;
        let issue = self
            .proofreader
            .target
            .filter(|(target, _)| *target == id)
            .and_then(|(_, n)| {
                self.proofreader.issues.get(&id).and_then(|(text, issues)| {
                    if text == &self.blocks[index].text {
                        issues.get(n).cloned()
                    } else {
                        None
                    }
                })
            });
        if let Some(issue) = issue {
            if !self.settings.proofing.highlight_only {
                ui.label(RichText::new(&issue.message).small());
                for replacement in &issue.replacements {
                    if ui.button(replacement).hand().clicked() {
                        self.insert_at(index, issue.start, issue.end, replacement);
                        self.proofreader.invalidate();
                        ui.close_menu();
                    }
                }
            }
            if ui.button("Ignore word").hand().clicked() {
                self.settings
                    .proofing
                    .ignored_words
                    .push(issue.word.clone());
                self.proofreader.invalidate();
                ui.close_menu();
            }
            if issue.spelling && ui.button("Add to dictionary").hand().clicked() {
                self.settings.proofing.dictionary.push(issue.word);
                self.proofreader.invalidate();
                ui.close_menu();
            }
            if !issue.spelling && ui.button("Ignore rule").hand().clicked() {
                self.settings.proofing.ignored_rules.push(issue.source);
                self.proofreader.invalidate();
                ui.close_menu();
            }
            let _ = self.settings.save(&self.workspace.directory);
            ui.separator();
        }
    }
    pub(super) fn paint_proofing(
        &mut self,
        ui: &egui::Ui,
        index: usize,
        output: &egui::text_edit::TextEditOutput,
    ) {
        let id = self.blocks[index].id;
        let Some((text, issues)) = self.proofreader.issues.get(&id) else {
            return;
        };
        if text != &self.blocks[index].text {
            return;
        }
        let pointer = ui.input(|i| i.pointer.interact_pos()).map(|p| {
            output
                .galley
                .cursor_from_pos(p - output.galley_pos)
                .ccursor
                .index
        });
        for (n, issue) in issues.iter().enumerate() {
            if output.response.secondary_clicked()
                && pointer.is_some_and(|p| p >= issue.start && p <= issue.end)
            {
                self.proofreader.target = Some((id, n));
            }
            let color = if issue.spelling {
                self.settings.proofing.spelling_color
            } else {
                self.settings.proofing.grammar_color
            };
            let color = Color32::from_rgb(color[0], color[1], color[2])
                .gamma_multiply(self.settings.proofing.opacity);
            let start = output
                .galley
                .from_ccursor(egui::text::CCursor::new(issue.start));
            let end = output
                .galley
                .from_ccursor(egui::text::CCursor::new(issue.end));
            for row in start.rcursor.row..=end.rcursor.row {
                if let Some(line) = output.galley.rows.get(row) {
                    let left = if row == start.rcursor.row {
                        output.galley.pos_from_cursor(&start).left()
                    } else {
                        line.rect.left()
                    };
                    let right = if row == end.rcursor.row {
                        output.galley.pos_from_cursor(&end).left()
                    } else {
                        line.rect.right()
                    };
                    let y = output.galley_pos.y + line.rect.bottom() - 1.0;
                    if self.settings.proofing.dotted {
                        let mut x = left;
                        while x < right {
                            ui.painter().circle_filled(
                                Pos2::new(output.galley_pos.x + x, y),
                                0.8,
                                color,
                            );
                            x += 3.0;
                        }
                    } else {
                        let points = (0..=((right - left) / 2.0).max(0.0) as usize)
                            .map(|step| {
                                Pos2::new(
                                    output.galley_pos.x + left + step as f32 * 2.0,
                                    y + if step % 2 == 0 { 0.0 } else { 1.5 },
                                )
                            })
                            .collect();
                        ui.painter()
                            .add(egui::Shape::line(points, Stroke::new(1.0_f32, color)));
                    }
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_engines_check_offline_and_highlight_only_hides_suggestions() {
        let ctx = egui::Context::default();
        let mut checker = Proofreader::default();
        checker.start(&ctx);
        let rx = checker.rx.as_ref().unwrap();
        match rx.recv_timeout(std::time::Duration::from_secs(20)).unwrap() {
            ResultMessage::Ready(result) => result.unwrap(),
            _ => panic!("Engine must initialize first"),
        }
        checker
            .tx
            .as_ref()
            .unwrap()
            .send(Request {
                document: "test".into(),
                id: 1,
                text: "This are a misspelled sentance.".into(),
                settings: settings::Proofing {
                    highlight_only: true,
                    ..Default::default()
                },
            })
            .unwrap();
        let ResultMessage::Checked(_, _, _, issues) =
            rx.recv_timeout(std::time::Duration::from_secs(20)).unwrap()
        else {
            panic!("Expected checked text")
        };
        assert!(issues.iter().any(|issue| issue.spelling));
        assert!(issues.iter().any(|issue| !issue.spelling));
        assert!(issues
            .iter()
            .all(|issue| issue.replacements.is_empty() && issue.message.is_empty()));
    }
    #[test]
    fn skips_links_and_urls() {
        assert_eq!(
            super::excluded("é [[Page]] https://example.com"),
            vec![(2, 10), (11, 30)]
        );
    }
}
