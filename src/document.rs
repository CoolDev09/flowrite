use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(super) enum BlockKind {
    Text,
    Heading1,
    Heading2,
    Bulleted,
    Numbered,
    Todo,
    Quote,
    Code,
    Callout,
    Divider,
}

impl BlockKind {
    pub(super) const ALL: [Self; 10] = [
        Self::Text,
        Self::Heading1,
        Self::Heading2,
        Self::Bulleted,
        Self::Numbered,
        Self::Todo,
        Self::Quote,
        Self::Code,
        Self::Callout,
        Self::Divider,
    ];

    pub(super) fn icon(self) -> &'static str {
        match self {
            Self::Text => "T",
            Self::Heading1 => "H1",
            Self::Heading2 => "H2",
            Self::Bulleted => "•",
            Self::Numbered => "1.",
            Self::Todo => "☐",
            Self::Quote => "❝",
            Self::Code => "</>",
            Self::Callout => "✦",
            Self::Divider => "—",
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Heading1 => "Heading 1",
            Self::Heading2 => "Heading 2",
            Self::Bulleted => "Bulleted list",
            Self::Numbered => "Numbered list",
            Self::Todo => "To-do",
            Self::Quote => "Quote",
            Self::Code => "Code",
            Self::Callout => "Callout",
            Self::Divider => "Divider",
        }
    }

    pub(super) fn hint(self) -> &'static str {
        match self {
            Self::Text => "Just start writing",
            Self::Heading1 => "Large section heading",
            Self::Heading2 => "Medium section heading",
            Self::Bulleted => "A simple bulleted list",
            Self::Numbered => "An ordered list",
            Self::Todo => "A task with a checkbox",
            Self::Quote => "A highlighted quotation",
            Self::Code => "A monospace code snippet",
            Self::Callout => "A coloured note",
            Self::Divider => "A visual break",
        }
    }

    pub(super) fn markdown_prefix(self, number: usize) -> &'static str {
        match self {
            Self::Text => "",
            Self::Heading1 => "# ",
            Self::Heading2 => "## ",
            Self::Bulleted => "- ",
            Self::Numbered => {
                let _ = number;
                "1. "
            }
            Self::Todo => "- [ ] ",
            Self::Quote => "> ",
            Self::Code => "    ",
            Self::Callout => "> 💡 ",
            Self::Divider => "---",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub(super) struct InlineStyle {
    pub(super) bold: bool,
    pub(super) italic: bool,
    pub(super) underline: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(super) struct FormatSpan {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) style: InlineStyle,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(super) struct Block {
    pub(super) id: u64,
    pub(super) kind: BlockKind,
    pub(super) text: String,
    pub(super) checked: bool,
    #[serde(default)]
    pub(super) format: InlineStyle,
    #[serde(default)]
    pub(super) marks: Vec<FormatSpan>,
}

impl Block {
    pub(super) fn new(id: u64, kind: BlockKind, text: impl Into<String>) -> Self {
        Self {
            id,
            kind,
            text: text.into(),
            checked: false,
            format: InlineStyle::default(),
            marks: Vec::new(),
        }
    }

    pub(super) fn character_styles(&self) -> Vec<InlineStyle> {
        let mut styles = vec![self.format; self.text.chars().count()];
        for span in &self.marks {
            for index in span.start..span.end.min(styles.len()) {
                styles[index] = span.style;
            }
        }
        styles
    }

    pub(super) fn set_character_styles(&mut self, styles: &[InlineStyle]) {
        self.marks.clear();
        let mut start = 0;
        while start < styles.len() {
            let style = styles[start];
            let mut end = start + 1;
            while end < styles.len() && styles[end] == style {
                end += 1;
            }
            if style != self.format {
                self.marks.push(FormatSpan { start, end, style });
            }
            start = end;
        }
    }

    pub(super) fn rebase_formatting(&mut self, old_text: &str, old_styles: &[InlineStyle]) {
        let old: Vec<char> = old_text.chars().collect();
        let new: Vec<char> = self.text.chars().collect();
        let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
        let suffix = old[prefix..]
            .iter()
            .rev()
            .zip(new[prefix..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        let inserted = new.len() - prefix - suffix;
        let inherited = old_styles
            .get(prefix.saturating_sub(1))
            .copied()
            .unwrap_or(self.format);
        let mut styles = old_styles[..prefix.min(old_styles.len())].to_vec();
        styles.extend(std::iter::repeat_n(inherited, inserted));
        styles.extend_from_slice(&old_styles[old.len().saturating_sub(suffix)..]);
        self.set_character_styles(&styles);
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(super) enum PaperFormat {
    Freeform,
    Apa,
    Mla,
}

impl PaperFormat {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Freeform => "Freeform",
            Self::Apa => "APA 7th",
            Self::Mla => "MLA 9th",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(super) struct DocumentStyle {
    pub(super) paper_format: PaperFormat,
    pub(super) font: String,
    pub(super) font_size: f32,
    pub(super) line_spacing: f32,
    pub(super) margin_inches: f32,
    pub(super) first_line_indent_inches: f32,
}

impl Default for DocumentStyle {
    fn default() -> Self {
        Self {
            paper_format: PaperFormat::Freeform,
            font: "Segoe UI".to_owned(),
            font_size: 14.0,
            line_spacing: 1.45,
            margin_inches: 0.8,
            first_line_indent_inches: 0.0,
        }
    }
}

impl DocumentStyle {
    pub(super) fn apply_preset(&mut self, format: PaperFormat) {
        self.paper_format = format;
        match format {
            PaperFormat::Freeform => {
                self.font = "Segoe UI".to_owned();
                self.font_size = 14.0;
                self.line_spacing = 1.45;
                self.margin_inches = 0.8;
                self.first_line_indent_inches = 0.0;
            }
            PaperFormat::Apa | PaperFormat::Mla => {
                self.font = "Times New Roman".to_owned();
                self.font_size = 12.0;
                self.line_spacing = 2.0;
                self.margin_inches = 1.0;
                self.first_line_indent_inches = 0.5;
            }
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub(super) struct PaperDetails {
    pub(super) author: String,
    pub(super) instructor: String,
    pub(super) course: String,
    pub(super) due_date: String,
    pub(super) last_name: String,
}

#[derive(Clone, Copy)]
pub(super) enum ExportTarget {
    Markdown,
    Pdf,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(super) struct SavedDocument {
    pub(super) version: u8,
    pub(super) title: String,
    pub(super) blocks: Vec<Block>,
    pub(super) style: DocumentStyle,
    pub(super) details: PaperDetails,
    #[serde(default)]
    pub(super) citations: Vec<String>,
}

pub(super) fn sorted_citations(citations: &[String]) -> Vec<String> {
    let mut sorted: Vec<String> = citations
        .iter()
        .filter(|text| !text.trim().is_empty())
        .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    sorted.sort_by_key(|text| text.to_lowercase());
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_documents_open_without_losing_text() {
        let json = r#"{"id":7,"kind":"Text","text":"Older document","checked":false}"#;
        let block: Block = serde_json::from_str(json).unwrap();
        assert_eq!(block.text, "Older document");
        assert_eq!(block.format, InlineStyle::default());
        assert!(block.marks.is_empty());
    }

    #[test]
    fn editing_before_a_formatted_word_moves_its_marks() {
        let mut block = Block::new(1, BlockKind::Text, "An essay");
        block.marks = vec![FormatSpan {
            start: 3,
            end: 8,
            style: InlineStyle {
                bold: true,
                ..Default::default()
            },
        }];
        let styles = block.character_styles();
        block.text = "A short essay".to_owned();
        block.rebase_formatting("An essay", &styles);
        assert!(block.character_styles()[8].bold);
        assert_eq!(block.text, "A short essay");
    }
}
