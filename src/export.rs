use super::*;

impl FlowWriteApp {
    pub(super) fn markdown(&self) -> String {
        let mut output = format!("# {}\n\n", self.title.trim());
        if self.style.paper_format == PaperFormat::Mla {
            for value in [
                &self.details.author,
                &self.details.instructor,
                &self.details.course,
                &self.details.due_date,
            ] {
                if !value.is_empty() {
                    output.push_str(&format!("{value}  \n"));
                }
            }
            output.push('\n');
        }
        let mut number = 0;
        for block in &self.blocks {
            if block.kind == BlockKind::Divider {
                output.push_str("---\n\n");
                continue;
            }
            if block.kind == BlockKind::Numbered {
                number += 1;
            } else {
                number = 0;
            }
            if block.kind == BlockKind::Code {
                output.push_str(&format!("~~~\n{}\n~~~\n\n", block.text));
                continue;
            }
            let prefix = match block.kind {
                BlockKind::Todo => {
                    if block.checked {
                        "- [x] ".to_owned()
                    } else {
                        "- [ ] ".to_owned()
                    }
                }
                BlockKind::Numbered => format!("{number}. "),
                _ => block.kind.markdown_prefix(1).to_owned(),
            };
            output.push_str(&prefix);
            output.push_str(&markdown_inline(block));
            output.push_str("\n\n");
        }
        let citations = sorted_citations(&self.citations);
        if !citations.is_empty() {
            output.push_str(&format!("## {}\n\n", self.citations_heading()));
            for citation in citations {
                output.push_str(&citation);
                output.push_str("\n\n");
            }
        }
        output
    }

    pub(super) fn export(&mut self, target: ExportTarget) {
        let (title, extension, filter) = match target {
            ExportTarget::Markdown => ("Export Markdown", "md", "Markdown"),
            ExportTarget::Pdf => ("Export PDF", "pdf", "PDF"),
        };
        let Some(path) = FileDialog::new()
            .set_title(title)
            .set_file_name(format!("{}.{}", safe_filename(&self.title), extension))
            .add_filter(filter, &[extension])
            .save_file()
        else {
            return;
        };
        let result = match target {
            ExportTarget::Markdown => {
                std::fs::write(&path, self.markdown()).map_err(|e| e.to_string())
            }
            ExportTarget::Pdf => self.write_pdf(&path).map_err(|e| e.to_string()),
        };
        self.notice = Some(match result {
            Ok(()) => format!("Exported {}", path.display()),
            Err(error) => format!("Could not export: {error}"),
        });
    }

    pub(super) fn write_pdf(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let mut pdf = Paper::new(self)?;
        let plain = InlineStyle::default();
        let bold = InlineStyle {
            bold: true,
            ..plain
        };
        match self.style.paper_format {
            PaperFormat::Mla => {
                for value in [
                    &self.details.author,
                    &self.details.instructor,
                    &self.details.course,
                    &self.details.due_date,
                ] {
                    pdf.paragraph(value, &[], 12.0, 0.0, 0.0, false);
                }
                pdf.paragraph(&self.title, &[], 12.0, 0.0, 0.0, true);
            }
            PaperFormat::Apa => {
                pdf.y -= 4.0 * pdf.line_height;
                pdf.paragraph(
                    &self.title,
                    &vec![bold; self.title.chars().count()],
                    12.0,
                    0.0,
                    0.0,
                    true,
                );
                pdf.y -= pdf.line_height;
                for value in [
                    &self.details.author,
                    &self.details.course,
                    &self.details.instructor,
                    &self.details.due_date,
                ] {
                    if !value.is_empty() {
                        pdf.paragraph(value, &[], 12.0, 0.0, 0.0, true);
                    }
                }
                pdf.new_page();
                pdf.paragraph(
                    &self.title,
                    &vec![bold; self.title.chars().count()],
                    12.0,
                    0.0,
                    0.0,
                    true,
                );
            }
            PaperFormat::Freeform => {
                pdf.paragraph(
                    &self.title,
                    &vec![bold; self.title.chars().count()],
                    22.0,
                    0.0,
                    0.0,
                    false,
                );
                pdf.y -= pdf.line_height;
            }
        }
        let mut number = 0;
        for block in &self.blocks {
            if block.kind == BlockKind::Divider {
                pdf.ensure_space();
                pdf.rule(pdf.margin, 215.9 - pdf.margin, pdf.y);
                pdf.y -= pdf.line_height;
                continue;
            }
            if block.kind == BlockKind::Numbered {
                number += 1;
            } else {
                number = 0;
            }
            let prefix = match block.kind {
                BlockKind::Bulleted => "• ".to_owned(),
                BlockKind::Numbered => format!("{number}. "),
                BlockKind::Todo => {
                    if block.checked {
                        "[x] ".to_owned()
                    } else {
                        "[ ] ".to_owned()
                    }
                }
                BlockKind::Callout => "NOTE: ".to_owned(),
                _ => String::new(),
            };
            let mut styles = vec![plain; prefix.chars().count()];
            styles.extend(block.character_styles());
            if matches!(block.kind, BlockKind::Heading1 | BlockKind::Heading2) {
                for s in &mut styles {
                    s.bold = true;
                }
            }
            let size = match block.kind {
                BlockKind::Heading1 => 18.0,
                BlockKind::Heading2 => 14.0,
                _ => self.style.font_size,
            };
            let indent = if block.kind == BlockKind::Text {
                self.style.first_line_indent_inches * 25.4
            } else {
                0.0
            };
            pdf.paragraph(
                &format!("{prefix}{}", block.text),
                &styles,
                size,
                indent,
                0.0,
                false,
            );
            if self.style.paper_format == PaperFormat::Freeform {
                pdf.y -= pdf.line_height * 0.35;
            }
        }
        let citations = sorted_citations(&self.citations);
        if !citations.is_empty() {
            pdf.new_page();
            pdf.line_height = self.style.font_size * 2.0 * 25.4 / 72.0;
            let heading = self.citations_heading();
            let style = if self.style.paper_format == PaperFormat::Apa {
                bold
            } else {
                plain
            };
            pdf.paragraph(
                heading,
                &vec![style; heading.chars().count()],
                self.style.font_size,
                0.0,
                0.0,
                true,
            );
            for citation in citations {
                pdf.paragraph(&citation, &[], self.style.font_size, 0.0, 12.7, false);
            }
        }
        pdf.document
            .save(&mut BufWriter::new(File::create(path)?))?;
        Ok(())
    }
}

struct Paper {
    document: printpdf::PdfDocumentReference,
    layer: printpdf::PdfLayerReference,
    fonts: Vec<printpdf::IndirectFontRef>,
    metrics: egui::Context,
    style: DocumentStyle,
    surname: String,
    number: usize,
    margin: f32,
    y: f32,
    line_height: f32,
}

impl Paper {
    fn new(app: &FlowWriteApp) -> Result<Self, Box<dyn std::error::Error>> {
        let (document, page, layer) =
            PdfDocument::new(&app.title, Mm(215.9), Mm(279.4), "Document");
        let files = match app.style.font.as_str() {
            "Times New Roman" => ["times.ttf", "timesbd.ttf", "timesi.ttf", "timesbi.ttf"],
            "Consolas" => [
                "consola.ttf",
                "consolab.ttf",
                "consolai.ttf",
                "consolaz.ttf",
            ],
            "Arial" => ["arial.ttf", "arialbd.ttf", "ariali.ttf", "arialbi.ttf"],
            _ => [
                "segoeui.ttf",
                "segoeuib.ttf",
                "segoeuii.ttf",
                "segoeuiz.ttf",
            ],
        };
        let fallbacks = [
            BuiltinFont::TimesRoman,
            BuiltinFont::TimesBold,
            BuiltinFont::TimesItalic,
            BuiltinFont::TimesBoldItalic,
        ];
        let mut fonts = Vec::new();
        for (index, file) in files.iter().enumerate() {
            fonts.push(
                match File::open(Path::new("C:\\Windows\\Fonts").join(file)) {
                    Ok(file) => document.add_external_font(file)?,
                    Err(_) => document.add_builtin_font(fallbacks[index])?,
                },
            );
        }
        let metrics = egui::Context::default();
        configure_fonts(&metrics);
        metrics.set_pixels_per_point(4.0);
        let _ = metrics.run(egui::RawInput::default(), |_| {});
        let margin = app.style.margin_inches * 25.4;
        let layer = document.get_page(page).get_layer(layer);
        let mut result = Self {
            document,
            layer,
            fonts,
            metrics,
            style: app.style.clone(),
            surname: app.details.last_name.clone(),
            number: 1,
            margin,
            y: 279.4 - margin - app.style.font_size * 25.4 / 72.0,
            line_height: app.style.font_size * app.style.line_spacing * 25.4 / 72.0,
        };
        result.header();
        Ok(result)
    }
    fn width(&self, text: &str, size: f32, style: InlineStyle) -> f32 {
        self.metrics.fonts(|fonts| {
            fonts
                .layout_no_wrap(
                    text.to_owned(),
                    editor_ui::document_font(&self.style, style, size),
                    Color32::BLACK,
                )
                .size()
                .x
        }) * 25.4
            / 72.0
    }
    fn draw(&self, text: &str, size: f32, x: f32, y: f32, style: InlineStyle) {
        let index = usize::from(style.bold) + 2 * usize::from(style.italic);
        self.layer
            .use_text(text, size, Mm(x), Mm(y), &self.fonts[index]);
        if style.underline {
            self.rule(x, x + self.width(text, size, style), y - 0.7);
        }
    }
    fn rule(&self, x1: f32, x2: f32, y: f32) {
        self.layer.set_outline_thickness(0.5);
        self.layer.add_line(printpdf::Line {
            points: vec![
                (printpdf::Point::new(Mm(x1), Mm(y)), false),
                (printpdf::Point::new(Mm(x2), Mm(y)), false),
            ],
            is_closed: false,
        });
    }
    fn header(&mut self) {
        if self.style.paper_format == PaperFormat::Freeform {
            return;
        }
        let text = if self.style.paper_format == PaperFormat::Mla && !self.surname.is_empty() {
            format!("{} {}", self.surname, self.number)
        } else {
            self.number.to_string()
        };
        let style = InlineStyle::default();
        let width = self.width(&text, 12.0, style);
        self.draw(
            &text,
            12.0,
            215.9 - self.margin - width,
            279.4 - 12.7,
            style,
        );
    }
    fn new_page(&mut self) {
        let (page, layer) = self.document.add_page(Mm(215.9), Mm(279.4), "Document");
        self.layer = self.document.get_page(page).get_layer(layer);
        self.number += 1;
        self.y = 279.4 - self.margin - self.style.font_size * 25.4 / 72.0;
        self.header();
    }
    fn ensure_space(&mut self) {
        if self.y < self.margin {
            self.new_page();
        }
    }
    fn paragraph(
        &mut self,
        text: &str,
        styles: &[InlineStyle],
        size: f32,
        first_indent: f32,
        rest_indent: f32,
        center: bool,
    ) {
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() {
            self.y -= self.line_height;
            return;
        }
        let mut start = 0;
        let mut first = true;
        while start < chars.len() {
            let indent = if first { first_indent } else { rest_indent };
            let limit = 215.9 - 2.0 * self.margin - indent;
            let mut end = start;
            let mut last_space = None;
            let mut width = 0.0;
            while end < chars.len() && chars[end] != '\n' {
                let style = styles.get(end).copied().unwrap_or_default();
                let next = self.width(&chars[end].to_string(), size, style);
                if width + next > limit && end > start {
                    break;
                }
                width += next;
                if chars[end].is_whitespace() {
                    last_space = Some(end);
                }
                end += 1;
            }
            if end < chars.len() && chars[end] != '\n' {
                if let Some(space) = last_space.filter(|space| *space > start) {
                    end = space;
                }
            }
            let mut line_end = end;
            while line_end > start && chars[line_end - 1].is_whitespace() {
                line_end -= 1;
            }
            let mut runs: Vec<(String, InlineStyle)> = Vec::new();
            for i in start..line_end {
                let style = styles.get(i).copied().unwrap_or_default();
                if let Some((text, last_style)) = runs.last_mut().filter(|(_, s)| *s == style) {
                    let _ = last_style;
                    text.push(chars[i]);
                } else {
                    runs.push((chars[i].to_string(), style));
                }
            }
            let width: f32 = runs.iter().map(|(t, s)| self.width(t, size, *s)).sum();
            let mut x = if center {
                (215.9 - width) / 2.0
            } else {
                self.margin + indent
            };
            self.ensure_space();
            for (text, style) in runs {
                self.draw(&text, size, x, self.y, style);
                x += self.width(&text, size, style);
            }
            self.y -= self.line_height.max(size * 25.4 / 72.0 * 1.2);
            start = end.max(start + 1);
            if start < chars.len() && chars[start] == '\n' {
                start += 1;
            } else {
                while start < chars.len() && chars[start] == ' ' {
                    start += 1;
                }
            }
            first = false;
        }
    }
}

fn markdown_inline(block: &Block) -> String {
    let chars: Vec<_> = block.text.chars().collect();
    let styles = block.character_styles();
    let links = wikilinks::links(&block.text);
    let mut output = String::new();
    let mut index = 0;
    while index < chars.len() {
        if let Some(link) = links.iter().find(|l| l.start == index) {
            output.push_str(&format!(
                "[{}](<{}.md>)",
                link.title,
                safe_filename(&link.title)
            ));
            index = link.end;
            continue;
        }
        let style = styles[index];
        let mut end = index + 1;
        while end < chars.len() && styles[end] == style && !links.iter().any(|l| l.start == end) {
            end += 1;
        }
        let text: String = chars[index..end].iter().collect();
        if style.underline {
            output.push_str("<u>");
        }
        if style.bold {
            output.push_str("**");
        }
        if style.italic {
            output.push('*');
        }
        output.push_str(&text);
        if style.italic {
            output.push('*');
        }
        if style.bold {
            output.push_str("**");
        }
        if style.underline {
            output.push_str("</u>");
        }
        index = end;
    }
    output
}

pub(super) fn safe_filename(title: &str) -> String {
    let cleaned: String = title
        .trim()
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            _ => c,
        })
        .collect();
    let cleaned = cleaned.trim_end_matches(['.', ' ']);
    if cleaned.is_empty() {
        "untitled-document".to_owned()
    } else {
        cleaned.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mla_export_includes_a_separate_citation_page() {
        let mut app = FlowWriteApp::empty();
        app.title = "An Example MLA Essay".into();
        app.style.apply_preset(PaperFormat::Mla);
        app.details = PaperDetails {
            author: "Alex Morgan".into(),
            last_name: "Morgan".into(),
            instructor: "Professor Taylor".into(),
            course: "English 101".into(),
            due_date: "9 October 2026".into(),
        };
        let paragraph="Lorem ipsum dolor sit amet, consectetur adipiscing elit. Integer vitae nulla sed arcu tincidunt pretium. Curabitur quis ligula a justo consequat fermentum. Sed dignissim, augue vitae egestas posuere, lectus neque tempor sem, eget sollicitudin arcu mauris at sapien. Nulla facilisi. Vivamus euismod lorem non eros mattis, eget convallis mi luctus. Praesent vitae justo ut risus placerat interdum.";
        app.blocks = (1..=4)
            .map(|id| Block::new(id, BlockKind::Text, paragraph))
            .collect();
        app.citations=vec!["Williams, Clara. The Art of Thoughtful Writing: An Introduction to Essays, Revision, and Research. Example Press, 2024.".into(),"Adams, Jordan. Reading the Everyday: Essays on Language, Culture, and the Stories We Tell. Sample University Press, 2023.".into()];
        let directory = std::env::var_os("FLOWWRITE_SAMPLE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let path = directory.join("MLA-example.pdf");
        app.write_pdf(&path).unwrap();
        assert!(std::fs::read(&path).unwrap().starts_with(b"%PDF"));
        if std::env::var_os("FLOWWRITE_SAMPLE_DIR").is_some() {
            std::fs::write(
                directory.join("MLA-example.flowwrite"),
                serde_json::to_vec_pretty(&app.local_document()).unwrap(),
            )
            .unwrap();
            std::fs::write(directory.join("MLA-example.md"), app.markdown()).unwrap();
        } else {
            std::fs::remove_file(path).unwrap();
        }
    }
    #[test]
    fn markdown_keeps_links_styles_and_alphabetized_citations() {
        let mut app = FlowWriteApp::empty();
        app.blocks[0].text = "Read [[Research notes]]".into();
        app.blocks[0].format.bold = true;
        app.citations = vec!["Zulu. A book.".into(), "Adams. Another book.".into()];
        let result = app.markdown();
        assert!(result.contains("**Read **[Research notes](<Research notes.md>)"));
        assert!(result.find("Adams.").unwrap() < result.find("Zulu.").unwrap());
        assert!(result.contains("## Works Cited"));
    }
}
