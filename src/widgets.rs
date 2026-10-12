use super::*;
pub(super) trait HandCursor {
    fn hand(self) -> Self;
}
impl HandCursor for egui::Response {
    fn hand(self) -> Self {
        if self.enabled() && pointer_enabled(&self.ctx) {
            self.on_hover_cursor(egui::CursorIcon::PointingHand)
        } else {
            self
        }
    }
}
pub(super) fn pointer_enabled(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(egui::Id::new("pointer_enabled")))
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hand_cursor_enabled_disabled_and_text_fields() {
        let ctx = egui::Context::default();
        let mut text = String::new();
        let mut bounds = Vec::new();
        let draw = |ctx: &egui::Context, bounds: &mut Vec<egui::Rect>, text: &mut String| {
            egui::CentralPanel::default().show(ctx, |ui| {
                bounds.clear();
                bounds.push(ui.button("Button").hand().rect);
                bounds.push(ui.small_button("Small").hand().rect);
                bounds.push(ui.selectable_label(false, "Select").hand().rect);
                bounds.push(
                    ui.add_enabled(false, egui::Button::new("Disabled"))
                        .hand()
                        .rect,
                );
                bounds.push(ui.text_edit_singleline(text).rect);
            });
        };
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            draw(ctx, &mut bounds, &mut text)
        });
        for (index, expected) in [
            (0, egui::CursorIcon::PointingHand),
            (1, egui::CursorIcon::PointingHand),
            (2, egui::CursorIcon::PointingHand),
            (3, egui::CursorIcon::Default),
            (4, egui::CursorIcon::Text),
        ] {
            let position = bounds[index].center();
            let output = ctx.run(
                egui::RawInput {
                    events: vec![egui::Event::PointerMoved(position)],
                    ..Default::default()
                },
                |ctx| draw(ctx, &mut bounds, &mut text),
            );
            assert_eq!(
                output.platform_output.cursor_icon, expected,
                "control {index}"
            );
        }
    }
    #[test]
    fn clickable_controls_use_the_cursor_helper() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            let source = source
                .split("#[cfg(test)]")
                .next()
                .unwrap()
                .split_whitespace()
                .collect::<String>();
            for token in ["ui.button(", "ui.small_button(", "ui.selectable_label("] {
                let mut offset = 0;
                while let Some(found) = source[offset..].find(token) {
                    let start = offset + found;
                    let mut depth = 1;
                    let mut quoted = false;
                    let mut escaped = false;
                    let args = start + token.len();
                    let mut end = args;
                    for (i, c) in source[args..].char_indices() {
                        if quoted {
                            if escaped {
                                escaped = false;
                            } else if c == '\\' {
                                escaped = true;
                            } else if c == '"' {
                                quoted = false;
                            }
                        } else {
                            if c == '"' {
                                quoted = true;
                            } else if c == '(' {
                                depth += 1;
                            } else if c == ')' {
                                depth -= 1;
                                if depth == 0 {
                                    end = args + i + 1;
                                    break;
                                }
                            }
                        }
                    }
                    let suffix = source[end..].split(';').next().unwrap();
                    assert!(
                        suffix.starts_with(".hand()")
                            || suffix.starts_with(".on_hover") && suffix.contains(".hand()"),
                        "{}: control needs .hand(): {}",
                        path.display(),
                        &source[start..end]
                    );
                    offset = end;
                }
            }
        }
    }
}
