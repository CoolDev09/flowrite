use super::*;
use crate::editor_ui::document_font;
use egui::text::{LayoutJob, TextFormat};

pub(super) fn job(
    text: &str,
    base: InlineStyle,
    marks: &[FormatSpan],
    style: &DocumentStyle,
    kind: BlockKind,
    size: f32,
    width: f32,
    matches: &[(usize, usize)],
) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    let mut run_start = 0;
    let mut previous: Option<(InlineStyle, bool)> = None;
    let boundaries: Vec<_> = text
        .char_indices()
        .map(|(byte, _)| byte)
        .chain(std::iter::once(text.len()))
        .collect();
    for index in 0..boundaries.len() {
        let next = if index + 1 < boundaries.len() {
            let mut inline = marks
                .iter()
                .find(|span| index >= span.start && index < span.end)
                .map(|span| span.style)
                .unwrap_or(base);
            if matches!(kind, BlockKind::Heading1 | BlockKind::Heading2) {
                inline.bold = true;
            }
            Some((
                inline,
                matches
                    .iter()
                    .any(|&(start, end)| index >= start && index < end),
            ))
        } else {
            None
        };
        if next != previous {
            if let Some((inline, matched)) = previous {
                job.append(
                    &text[boundaries[run_start]..boundaries[index]],
                    if run_start == 0
                        && kind == BlockKind::Text
                        && style.paper_format != PaperFormat::Freeform
                    {
                        style.first_line_indent_inches * 96.0
                    } else {
                        0.0
                    },
                    TextFormat {
                        font_id: if kind == BlockKind::Code {
                            document_font(
                                &DocumentStyle {
                                    font: "Liberation Mono".into(),
                                    ..style.clone()
                                },
                                inline,
                                size,
                            )
                        } else {
                            document_font(style, inline, size)
                        },
                        color: inline
                            .color
                            .map(|c| Color32::from_rgb(c[0], c[1], c[2]))
                            .unwrap_or(theme::current().ink),
                        background: if matched {
                            Color32::from_rgb(84, 76, 43)
                        } else {
                            inline
                                .highlight
                                .map(|c| Color32::from_rgb(c[0], c[1], c[2]))
                                .unwrap_or(Color32::TRANSPARENT)
                        },
                        underline: if inline.underline {
                            Stroke::new(1.0_f32, theme::current().ink)
                        } else {
                            Stroke::NONE
                        },
                        line_height: Some(size * style.line_spacing),
                        ..Default::default()
                    },
                );
            }
            run_start = index;
            previous = next;
        }
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn long_uniform_document_uses_one_text_run() {
        let text = "word ".repeat(5000);
        let job = job(
            &text,
            InlineStyle::default(),
            &[],
            &DocumentStyle::default(),
            BlockKind::Text,
            16.0,
            700.0,
            &[],
        );
        assert_eq!(job.sections.len(), 1);
        assert_eq!(job.text, text);
    }
}
