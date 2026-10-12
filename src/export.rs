use super::*;

impl FlowriteApp {
    #[cfg(test)]
    pub(super) fn markdown(&self) -> String {
        self.markdown_with_images(&format!("{}_images", safe_filename(&self.title)))
    }
    fn markdown_with_images(&self, image_folder: &str) -> String {
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
        let mut counters = [0usize; 7];
        for block in &self.blocks {
            if block.kind == BlockKind::Image {
                if self.settings.export.include_images {
                    if let Some(asset) = &block.image {
                        if images::safe_asset(&asset.name) {
                            let alt = if asset.show_caption {
                                asset.caption.replace(['[', ']'], "")
                            } else {
                                String::new()
                            };
                            output.push_str(&format!(
                                "![{alt}](<{}>)\n\n",
                                markdown_path(&format!(
                                    "{image_folder}/image-{}-{}",
                                    block.id, asset.name
                                ))
                            ));
                        }
                    }
                }
                continue;
            }
            if block.kind == BlockKind::Divider {
                output.push_str("---\n\n");
                continue;
            }
            if block.kind == BlockKind::Numbered {
                let level = block.indent.min(6) as usize;
                counters[level] += 1;
                counters[level + 1..].fill(0);
            } else if !matches!(block.kind, BlockKind::Bulleted | BlockKind::Todo) {
                counters.fill(0);
            }
            let number = counters[block.indent.min(6) as usize];
            if block.kind == BlockKind::Code {
                let longest = block
                    .text
                    .split(|c| c != '`')
                    .map(str::len)
                    .max()
                    .unwrap_or(0);
                let fence = "`".repeat((longest + 1).max(3));
                output.push_str(&format!(
                    "{fence}{}\n{}\n{fence}\n\n",
                    block.language, block.text
                ));
                continue;
            }
            if block.kind == BlockKind::Callout {
                output.push_str(&format!("> [!{}]\n", block.callout_kind.marker()));
                for line in markdown_inline(block).lines() {
                    output.push_str(&format!("> {line}\n"));
                }
                output.push('\n');
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
            output.push_str(&"  ".repeat(block.indent.min(6) as usize));
            output.push_str(&prefix);
            output.push_str(&markdown_inline(block));
            output.push_str("\n\n");
        }
        let citations = self.citation_entries();
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
        let name = format!("{}.{}", safe_filename(&self.title), extension);
        let default = self.settings.general.folder.join(&name);
        let path = if self.settings.general.ask_export || default.exists() {
            FileDialog::new()
                .set_title(title)
                .set_directory(&self.settings.general.folder)
                .set_file_name(name)
                .add_filter(filter, &[extension])
                .save_file()
        } else {
            std::fs::create_dir_all(&self.settings.general.folder).ok();
            Some(default)
        };
        let Some(path) = path else {
            return;
        };
        let result = match target {
            ExportTarget::Markdown => self.write_markdown(&path),
            ExportTarget::Pdf => self.write_pdf(&path).map_err(|e| e.to_string()),
        };
        match result {
            Ok(()) => {
                self.exported_path = Some(path.clone());
                if self.settings.export.toast {
                    self.notice = Some(format!(
                        "Exported {}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ));
                }
                if self.settings.export.auto_open {
                    settings_ui::open_path(&path);
                }
                if let Some(parent) = path.parent() {
                    self.settings.general.folder = parent.to_owned();
                    let _ = self.settings.save(&self.workspace.directory);
                }
            }
            Err(error) => self.notice = Some(format!("Could not export: {error}")),
        }
    }

    fn write_markdown(&self, path: &Path) -> Result<(), String> {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let base = format!(
            "{}_images",
            path.file_stem().unwrap_or_default().to_string_lossy()
        );
        let mut folder = base.clone();
        let mut suffix = 2;
        while parent.join(&folder).exists() {
            folder = format!("{base}-{suffix}");
            suffix += 1;
        }
        let mut assets = Vec::new();
        if self.settings.export.include_images {
            for block in &self.blocks {
                if let Some(asset) = &block.image {
                    if asset.name.is_empty() {
                        continue;
                    }
                    if !images::safe_asset(&asset.name) {
                        return Err("Unsafe image asset name".into());
                    }
                    assets.push((
                        format!("image-{}-{}", block.id, asset.name),
                        images::framed_image(
                            &images::asset_directory(&self.workspace, &self.active_document)
                                .join(&asset.name),
                            asset.frame,
                        )?,
                    ));
                }
            }
        }
        if !assets.is_empty() {
            std::fs::create_dir(parent.join(&folder)).map_err(|e| e.to_string())?;
            for (name, image) in assets {
                image
                    .save(parent.join(&folder).join(name))
                    .map_err(|e| e.to_string())?;
            }
        }
        workspace::write_atomic(path, self.markdown_with_images(&folder).as_bytes())
            .map_err(|e| e.to_string())
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
        let mut counters = [0usize; 7];
        for block in &self.blocks {
            if block.kind == BlockKind::Image {
                if self.settings.export.include_images {
                    if let Some(asset) = &block.image {
                        if !asset.name.is_empty() {
                            if !images::safe_asset(&asset.name) {
                                return Err("Unsafe image asset name".into());
                            }
                            pdf.image(
                                asset,
                                &images::asset_directory(&self.workspace, &self.active_document)
                                    .join(&asset.name),
                            )?;
                        }
                    }
                }
                continue;
            }
            if block.kind == BlockKind::Divider {
                pdf.ensure_space();
                pdf.rule(pdf.margin, pdf.page_width - pdf.margin, pdf.y);
                pdf.y -= pdf.line_height;
                continue;
            }
            if block.kind == BlockKind::Numbered {
                let level = block.indent.min(6) as usize;
                counters[level] += 1;
                counters[level + 1..].fill(0);
            } else if !matches!(block.kind, BlockKind::Bulleted | BlockKind::Todo) {
                counters.fill(0);
            }
            let number = counters[block.indent.min(6) as usize];
            if matches!(block.kind, BlockKind::Code | BlockKind::Callout) {
                pdf.container(block);
                continue;
            }
            let prefix = match block.kind {
                BlockKind::Bulleted => format!("{} ", list_marker(block.indent, number, true)),
                BlockKind::Numbered => format!("{} ", list_marker(block.indent, number, false)),
                BlockKind::Todo => {
                    if block.checked {
                        "[x] ".to_owned()
                    } else {
                        "[ ] ".to_owned()
                    }
                }
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
            let list_indent = block.indent.min(6) as f32 * 6.35;
            pdf.block_background = block.background;
            pdf.paragraph(
                &format!("{prefix}{}", block.text),
                &styles,
                size,
                indent + list_indent,
                list_indent,
                false,
            );
            pdf.block_background = None;
            if self.style.paper_format == PaperFormat::Freeform {
                pdf.y -= pdf.line_height * 0.35;
            }
        }
        let citations = self.citation_entries();
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
                pdf.paragraph(
                    &citation,
                    &[],
                    self.style.font_size,
                    0.0,
                    self.settings.citations.hanging_inches * 25.4,
                    false,
                );
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
    page_width: f32,
    page_height: f32,
    running_header: bool,
    page_colors: bool,
    mono: bool,
    background: [u8; 3],
    ink: [u8; 3],
    block_background: Option<[u8; 3]>,
}

impl Paper {
    fn new(app: &FlowriteApp) -> Result<Self, Box<dyn std::error::Error>> {
        let (page_width, page_height) = if app.settings.export.a4 {
            (210.0, 297.0)
        } else {
            (215.9, 279.4)
        };
        let (document, page, layer) =
            PdfDocument::new(&app.title, Mm(page_width), Mm(page_height), "Document");
        let mut fonts = Vec::new();
        for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
            fonts.push(document.add_external_font(std::io::Cursor::new(fonts::data(
                &app.style.font,
                bold,
                italic,
            )))?);
        }
        for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
            fonts.push(document.add_external_font(std::io::Cursor::new(fonts::data(
                "Liberation Mono",
                bold,
                italic,
            )))?);
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
            y: page_height - margin - app.style.font_size * 25.4 / 72.0,
            line_height: app.style.font_size * app.style.line_spacing * 25.4 / 72.0,
            page_width,
            page_height,
            running_header: app.settings.citations.running_header
                && app.settings.export.page_numbers,
            page_colors: app.settings.export.page_colors,
            mono: false,
            background: if app.settings.export.page_colors {
                app.style
                    .page_color
                    .unwrap_or(app.theme.canvas.to_array()[..3].try_into().unwrap())
            } else {
                [255, 255, 255]
            },
            ink: if app.settings.export.page_colors {
                app.theme.ink.to_array()[..3].try_into().unwrap()
            } else {
                [0, 0, 0]
            },
            block_background: None,
        };
        result.rectangle(0.0, 0.0, page_width, page_height, result.background);
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
        let index =
            usize::from(style.bold) + 2 * usize::from(style.italic) + if self.mono { 4 } else { 0 };
        let width = self.width(text, size, style);
        if self.page_colors {
            if let Some(color) = style.highlight {
                self.rectangle(x, y - 1.0, width, size * 25.4 / 72.0 * 1.2, color);
            }
        }
        self.layer.set_fill_color(pdf_color(if self.page_colors {
            style.color.unwrap_or(self.ink)
        } else {
            [0, 0, 0]
        }));
        self.layer
            .use_text(text, size, Mm(x), Mm(y), &self.fonts[index]);
        if style.underline {
            self.rule(x, x + width, y - 0.7);
        }
    }
    fn rule(&self, x1: f32, x2: f32, y: f32) {
        self.layer.set_outline_color(pdf_color(self.ink));
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
        if self.style.paper_format == PaperFormat::Freeform || !self.running_header {
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
            self.page_width - self.margin - width,
            self.page_height - 12.7,
            style,
        );
    }
    fn new_page(&mut self) {
        let (page, layer) =
            self.document
                .add_page(Mm(self.page_width), Mm(self.page_height), "Document");
        self.layer = self.document.get_page(page).get_layer(layer);
        self.rectangle(0.0, 0.0, self.page_width, self.page_height, self.background);
        self.number += 1;
        self.y = self.page_height - self.margin - self.style.font_size * 25.4 / 72.0;
        self.header();
    }
    fn ensure_space(&mut self) {
        if self.y < self.margin {
            self.new_page();
        }
    }
    fn rectangle(&self, x: f32, y: f32, width: f32, height: f32, color: [u8; 3]) {
        self.layer.set_fill_color(pdf_color(color));
        self.layer.add_polygon(printpdf::Polygon {
            rings: vec![vec![
                (printpdf::Point::new(Mm(x), Mm(y)), false),
                (printpdf::Point::new(Mm(x + width), Mm(y)), false),
                (printpdf::Point::new(Mm(x + width), Mm(y + height)), false),
                (printpdf::Point::new(Mm(x), Mm(y + height)), false),
            ]],
            mode: printpdf::path::PaintMode::Fill,
            ..Default::default()
        });
    }
    fn rounded_box(&self, x: f32, y: f32, width: f32, height: f32, color: [u8; 3]) {
        let radius = 2.0_f32.min(height / 2.0);
        let mut points = Vec::new();
        for (cx, cy, angle) in [
            (x + width - radius, y + radius, -std::f32::consts::FRAC_PI_2),
            (x + width - radius, y + height - radius, 0.0),
            (x + radius, y + height - radius, std::f32::consts::FRAC_PI_2),
            (x + radius, y + radius, std::f32::consts::PI),
        ] {
            for i in 0..=8 {
                let a = angle + i as f32 * std::f32::consts::FRAC_PI_2 / 8.0;
                points.push((
                    printpdf::Point::new(Mm(cx + radius * a.cos()), Mm(cy + radius * a.sin())),
                    false,
                ));
            }
        }
        self.layer.set_fill_color(pdf_color(color));
        self.layer.set_outline_color(pdf_color([190, 190, 190]));
        self.layer.set_outline_thickness(0.4);
        self.layer.add_polygon(printpdf::Polygon {
            rings: vec![points],
            mode: printpdf::path::PaintMode::FillStroke,
            ..Default::default()
        });
    }
    fn container(&mut self, block: &Block) {
        let code = block.kind == BlockKind::Code;
        let font = self.style.font.clone();
        let original_height = self.line_height;
        if code {
            self.style.font = "Liberation Mono".into();
            self.mono = true;
        }
        let size = if code {
            self.style.font_size.min(12.0)
        } else {
            self.style.font_size
        };
        self.line_height = size * 1.5 * 25.4 / 72.0;
        let padding = 6.35;
        let width = self.page_width - 2.0 * self.margin;
        self.y -= padding + size * 25.4 / 72.0;
        let styles = block.character_styles();
        let job = text_layout::job(
            &block.text,
            block.format,
            &block.marks,
            &self.style,
            block.kind,
            size,
            (width - 2.0 * padding) * 72.0 / 25.4,
            &[],
        );
        let galley = self.metrics.fonts(|fonts| fonts.layout_job(job));
        let mut offset = 0;
        let mut row_index = 0;
        while row_index < galley.rows.len() {
            let available = ((self.y - self.margin - padding) / self.line_height)
                .floor()
                .max(0.0) as usize;
            if available == 0 {
                self.new_page();
                self.y -= padding;
                continue;
            }
            let count = available.min(galley.rows.len() - row_index);
            let height = count as f32 * self.line_height + 2.0 * padding;
            let top = self.y + size * 25.4 / 72.0 + padding;
            let (bar, mut tint) = if code {
                ([140, 140, 140], [243, 243, 243])
            } else {
                let (bar, _) = block.callout_kind.colors();
                (bar, [246, 247, 249])
            };
            if self.page_colors {
                tint = block.background.unwrap_or(if code {
                    [32, 34, 38]
                } else {
                    block.callout_kind.colors().1
                });
            }
            self.rounded_box(self.margin, top - height, width, height, tint);
            if !code {
                self.rectangle(self.margin, top - height + 2.0, 1.4, height - 4.0, bar);
            }
            for row in &galley.rows[row_index..row_index + count] {
                let mut x = self.margin + padding;
                let mut run = String::new();
                let mut last = InlineStyle::default();
                for glyph in &row.glyphs {
                    let style = styles.get(offset).copied().unwrap_or_default();
                    if !run.is_empty() && style != last {
                        self.draw(&run, size, x, self.y, last);
                        x += self.width(&run, size, last);
                        run.clear();
                    }
                    run.push(glyph.chr);
                    last = style;
                    offset += 1;
                }
                if !run.is_empty() {
                    self.draw(&run, size, x, self.y, last);
                }
                offset += usize::from(row.ends_with_newline);
                self.y -= self.line_height;
            }
            self.y -= padding * 2.0;
            row_index += count;
        }
        if galley.rows.is_empty() {
            self.rounded_box(self.margin, self.y - 10.0, width, 15.0, [243, 243, 243]);
            self.y -= 20.0;
        }
        self.style.font = font;
        self.mono = false;
        self.line_height = original_height;
    }
    fn image(
        &mut self,
        asset: &images::ImageAsset,
        path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let image = images::framed_image(path, asset.frame).map_err(std::io::Error::other)?;
        let ratio = image.width() as f32 / image.height() as f32;
        let available = self.page_width - 2.0 * self.margin;
        let mut width = match asset.frame {
            images::ImageFrame::Inline => available.min(137.6),
            images::ImageFrame::Circle => available.min(74.0),
            images::ImageFrame::Square | images::ImageFrame::Rounded => available.min(111.0),
            _ => available,
        };
        let max_height = self.page_height - 2.0 * self.margin - self.line_height * 2.0;
        let mut height = width / ratio;
        if height > max_height {
            height = max_height;
            width = height * ratio;
        }
        if self.y - height - self.line_height < self.margin {
            self.new_page();
        }
        // Flatten alpha against the print page; printpdf 0.7's mask path has
        // incorrect dimensions for non-square images.
        let mut rgb = Vec::with_capacity(image.width() as usize * image.height() as usize * 3);
        for pixel in image.pixels() {
            for (index, &channel) in pixel.0[..3].iter().enumerate() {
                rgb.push(
                    ((channel as u32 * pixel[3] as u32
                        + self.background[index] as u32 * (255 - pixel[3] as u32))
                        / 255) as u8,
                );
            }
        }
        let object = printpdf::ImageXObject {
            width: printpdf::Px(image.width() as usize),
            height: printpdf::Px(image.height() as usize),
            color_space: printpdf::ColorSpace::Rgb,
            bits_per_component: printpdf::ColorBits::Bit8,
            interpolate: true,
            image_data: rgb,
            image_filter: None,
            smask: None,
            clipping_bbox: None,
        };
        printpdf::Image::from(object).add_to_layer(
            self.layer.clone(),
            printpdf::ImageTransform {
                translate_x: Some(Mm(self.margin)),
                translate_y: Some(Mm(self.y - height)),
                scale_x: Some(width / (image.width() as f32 * 25.4 / 300.0)),
                scale_y: Some(height / (image.height() as f32 * 25.4 / 300.0)),
                dpi: Some(300.0),
                ..Default::default()
            },
        );
        self.y -= height + self.line_height;
        if asset.show_caption && !asset.caption.is_empty() {
            self.paragraph(&asset.caption, &[], self.style.font_size, 0.0, 0.0, true);
        }
        Ok(())
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
        let factor = 25.4 / 72.0;
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = (self.page_width - 2.0 * self.margin - rest_indent) / factor;
        let mut start = 0;
        while start < chars.len() {
            let style = styles.get(start).copied().unwrap_or_default();
            let mut end = start + 1;
            while end < chars.len() && styles.get(end).copied().unwrap_or_default() == style {
                end += 1;
            }
            job.append(
                &chars[start..end].iter().collect::<String>(),
                if start == 0 {
                    (first_indent - rest_indent) / factor
                } else {
                    0.0
                },
                egui::TextFormat {
                    font_id: editor_ui::document_font(&self.style, style, size),
                    color: Color32::BLACK,
                    ..Default::default()
                },
            );
            start = end;
        }
        let galley = self.metrics.fonts(|fonts| fonts.layout_job(job));
        let mut start = 0;
        for row in &galley.rows {
            let end = start + row.glyphs.len();
            let mut line_end = end.min(chars.len());
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
                (self.page_width - width) / 2.0
            } else {
                self.margin + rest_indent + row.rect.left() * factor
            };
            self.ensure_space();
            if self.page_colors {
                if let Some(color) = self.block_background {
                    self.rectangle(
                        self.margin,
                        self.y - 1.0,
                        self.page_width - 2.0 * self.margin,
                        self.line_height,
                        color,
                    );
                }
            }
            for (text, style) in runs {
                self.draw(&text, size, x, self.y, style);
                x += self.width(&text, size, style);
            }
            self.y -= self.line_height.max(size * 25.4 / 72.0 * 1.2);
            start = end + usize::from(row.ends_with_newline);
        }
    }
}

fn pdf_color(color: [u8; 3]) -> printpdf::Color {
    printpdf::Color::Rgb(printpdf::Rgb::new(
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0,
        None,
    ))
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
                markdown_path(&safe_filename(&link.title))
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
        let trimmed = text.trim();
        let leading = text.len() - text.trim_start().len();
        let trailing = text.trim_end().len();
        output.push_str(&text[..leading]);
        if trimmed.is_empty() {
            index = end;
            continue;
        }
        if style.underline {
            output.push_str("<u>");
        }
        if style.bold {
            output.push_str("**");
        }
        if style.italic {
            output.push('*');
        }
        output.push_str(trimmed);
        if style.italic {
            output.push('*');
        }
        if style.bold {
            output.push_str("**");
        }
        if style.underline {
            output.push_str("</u>");
        }
        output.push_str(&text[trailing..]);
        index = end;
    }
    output
}

fn markdown_path(path: &str) -> String {
    path.replace('%', "%25").replace('#', "%23")
}

pub(super) fn safe_filename(title: &str) -> String {
    let cleaned: String = title
        .trim()
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            c if c.is_control() => ' ',
            _ => c,
        })
        .collect();
    let cleaned = cleaned.trim_end_matches(['.', ' ']);
    let stem = cleaned.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix)
                .is_some_and(|n| n.len() == 1 && matches!(n.as_bytes()[0], b'1'..=b'9'))
        });
    if cleaned.is_empty() {
        "untitled-document".to_owned()
    } else if reserved {
        let (stem, extension) = cleaned
            .split_once('.')
            .map(|(s, e)| (s, format!(".{e}")))
            .unwrap_or((cleaned, String::new()));
        format!("{stem}-document{extension}")
    } else {
        cleaned.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn structured_blocks_and_images_export_together() {
        let mut app = FlowriteApp::empty();
        let directory = std::env::var_os("FLOWRITE_SAMPLE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::temp_dir().join(format!("flowrite-export-{}", workspace::timestamp()))
            });
        std::fs::create_dir_all(&directory).unwrap();
        app.workspace.directory = directory.join("isolated-library");
        app.dirty = true;
        app.commit_current();
        app.title = "Block export review".into();
        app.blocks=vec![Block::new(1,BlockKind::Text,"A paragraph above a callout, code container, nested list, and cropped image."),Block::new(2,BlockKind::Callout,"Check that this note has a distinct container, comfortable padding, and readable text. This paragraph wraps across several lines to verify the print layout."),Block::new(3,BlockKind::Code,"fn main() {\n    println!(\"Hello, flowrite!\");\n}"),Block::new(4,BlockKind::Numbered,"Parent item"),Block::new(5,BlockKind::Numbered,"Child item"),Block::new(6,BlockKind::Numbered,"Second child"),Block::new(7,BlockKind::Numbered,"Next parent"),Block::new(8,BlockKind::Image,"")];
        app.blocks[2].language = "rust".into();
        app.blocks[4].indent = 1;
        app.blocks[5].indent = 1;
        app.blocks[7].image = Some(images::ImageAsset {
            name: "sample.png".into(),
            frame: images::ImageFrame::Circle,
            caption: "Figure 1. Center-cropped circular sample.".into(),
            show_caption: true,
        });
        let assets = images::asset_directory(&app.workspace, &app.active_document);
        std::fs::create_dir_all(&assets).unwrap();
        image::RgbaImage::from_fn(240, 120, |x, y| {
            image::Rgba([(x * 255 / 240) as u8, (y * 255 / 120) as u8, 160, 255])
        })
        .save(assets.join("sample.png"))
        .unwrap();
        let pdf = directory.join("Block-export.pdf");
        app.write_pdf(&pdf).unwrap();
        assert!(std::fs::read(pdf).unwrap().starts_with(b"%PDF"));
        app.settings.export.page_colors = true;
        app.style.page_color = Some([27, 29, 32]);
        app.blocks[0].format.color = Some([222, 183, 120]);
        let dark_pdf = directory.join("Block-export-dark.pdf");
        app.write_pdf(&dark_pdf).unwrap();
        assert!(std::fs::read(dark_pdf).unwrap().starts_with(b"%PDF"));
        let md = directory.join("Block-export.md");
        app.write_markdown(&md).unwrap();
        let text = std::fs::read_to_string(md).unwrap();
        assert!(text.contains("  1. Child item"));
        assert!(text.contains("  2. Second child"));
        assert!(text.contains("2. Next parent"));
        assert!(text.contains("```rust"));
        assert!(text.contains("> [!NOTE]"));
        let link = text
            .split("](<")
            .nth(1)
            .unwrap()
            .split(">)")
            .next()
            .unwrap();
        assert!(link.starts_with("Block-export_images"));
        let exported = image::open(directory.join(link)).unwrap().to_rgba8();
        assert_eq!(exported.dimensions(), (120, 120));
        assert_eq!(exported.get_pixel(0, 0)[3], 0);
    }
    #[test]
    fn windows_reserved_names_are_safe() {
        assert_eq!(safe_filename("Needed/Fixes"), "Needed-Fixes");
        assert_eq!(safe_filename("CON"), "CON-document");
        assert_eq!(safe_filename("LPT1.txt"), "LPT1-document.txt");
        assert_eq!(safe_filename("   ..."), "untitled-document");
    }
    #[test]
    fn mla_export_includes_a_separate_citation_page() {
        let mut app = FlowriteApp::empty();
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
        let directory = std::env::var_os("FLOWRITE_SAMPLE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let path = directory.join("MLA-example.pdf");
        app.write_pdf(&path).unwrap();
        assert!(std::fs::read(&path).unwrap().starts_with(b"%PDF"));
        if std::env::var_os("FLOWRITE_SAMPLE_DIR").is_some() {
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
        let mut app = FlowriteApp::empty();
        app.blocks[0].text = "Read [[Research notes]]".into();
        app.blocks[0].format.bold = true;
        app.citations = vec!["Zulu. A book.".into(), "Adams. Another book.".into()];
        let result = app.markdown();
        assert!(result.contains("**Read** [Research notes](<Research notes.md>)"));
        assert!(result.find("Adams.").unwrap() < result.find("Zulu.").unwrap());
        assert!(result.contains("## Works Cited"));
    }
}
