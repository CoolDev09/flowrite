use super::*;

const TEXT: [(&str, [u8; 3]); 9] = [
    ("Gray", [190, 190, 190]),
    ("Brown", [209, 171, 137]),
    ("Orange", [240, 174, 114]),
    ("Yellow", [235, 211, 132]),
    ("Green", [140, 208, 156]),
    ("Blue", [145, 196, 235]),
    ("Teal", [132, 212, 209]),
    ("Pink", [230, 163, 190]),
    ("Red", [239, 150, 150]),
];
const BACKGROUNDS: [(&str, [u8; 3]); 9] = [
    ("Gray", [52, 53, 57]),
    ("Brown", [61, 45, 34]),
    ("Orange", [64, 43, 27]),
    ("Yellow", [60, 53, 27]),
    ("Green", [30, 52, 37]),
    ("Blue", [29, 46, 62]),
    ("Teal", [28, 52, 52]),
    ("Pink", [59, 35, 48]),
    ("Red", [60, 32, 32]),
];
#[derive(Clone, Copy)]
enum ColorAction {
    Text(Option<[u8; 3]>),
    Highlight(Option<[u8; 3]>),
    Block(Option<[u8; 3]>),
    Page(Option<[u8; 3]>),
    Clear,
}
fn swatches(ui: &mut egui::Ui, colors: &[(&str, [u8; 3])]) -> Option<Option<[u8; 3]>> {
    if ui.button("Default / no color").hand().clicked() {
        return Some(None);
    }
    let mut chosen = None;
    for row in colors.chunks(3) {
        ui.horizontal(|ui| {
            for &(label, color) in row {
                let button = egui::Button::new(RichText::new(label).color(
                    if color[0] as u32 + color[1] as u32 + color[2] as u32 > 400 {
                        Color32::BLACK
                    } else {
                        Color32::WHITE
                    },
                ))
                .fill(Color32::from_rgb(color[0], color[1], color[2]));
                if ui.add_sized([62.0, 28.0], button).hand().clicked() {
                    chosen = Some(Some(color));
                }
            }
        });
    }
    chosen
}
impl FlowriteApp {
    pub(super) fn colors_menu(&mut self, ui: &mut egui::Ui) {
        let mut action = None;
        icons::menu(ui, Icon::Color, "", |ui| {
            ui.menu_button("Text color", |ui| {
                if let Some(c) = swatches(ui, &TEXT) {
                    action = Some(ColorAction::Text(c));
                }
            })
            .response
            .hand();
            ui.menu_button("Text highlight", |ui| {
                if let Some(c) = swatches(ui, &BACKGROUNDS) {
                    action = Some(ColorAction::Highlight(c));
                }
            })
            .response
            .hand();
            ui.menu_button("Block background", |ui| {
                if let Some(c) = swatches(ui, &BACKGROUNDS) {
                    action = Some(ColorAction::Block(c));
                }
            })
            .response
            .hand();
            ui.menu_button("Page background", |ui| {
                if let Some(c) = swatches(
                    ui,
                    &[
                        ("Dark", [24, 25, 28]),
                        ("Dim", [31, 32, 35]),
                        ("Warm", [32, 29, 26]),
                        ("Cool", [25, 30, 34]),
                    ],
                ) {
                    action = Some(ColorAction::Page(c));
                }
            })
            .response
            .hand();
            ui.separator();
            if ui
                .button("Clear formatting / reset colors")
                .hand()
                .clicked()
            {
                action = Some(ColorAction::Clear);
            }
            if action.is_some() {
                ui.close_menu();
            }
        })
        .response
        .hand();
        if let Some(action) = action {
            self.apply_color(action);
        }
    }
    fn apply_color(&mut self, action: ColorAction) {
        if let ColorAction::Page(color) = action {
            self.style.page_color = color;
        } else {
            let index = self
                .focused_block
                .and_then(|id| self.blocks.iter().position(|b| b.id == id))
                .unwrap_or(0);
            let block = &mut self.blocks[index];
            match action {
                ColorAction::Block(color) => block.background = color,
                ColorAction::Text(_) | ColorAction::Highlight(_) | ColorAction::Clear => {
                    let mut styles = block.character_styles();
                    let range = self
                        .selection
                        .filter(|(id, a, b)| *id == block.id && a < b)
                        .map(|(_, a, b)| (a, b));
                    let change = |style: &mut InlineStyle| match action {
                        ColorAction::Text(c) => style.color = c,
                        ColorAction::Highlight(c) => style.highlight = c,
                        ColorAction::Clear => *style = InlineStyle::default(),
                        _ => {}
                    };
                    if let Some((start, end)) = range {
                        for style in styles.iter_mut().take(end).skip(start) {
                            change(style);
                        }
                    } else {
                        change(&mut block.format);
                        for style in &mut styles {
                            change(style);
                        }
                        if matches!(action, ColorAction::Clear) {
                            block.background = None;
                        }
                    }
                    block.set_character_styles(&styles);
                }
                _ => {}
            }
        }
        self.dirty = true;
        self.force_history = true;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn colors_apply_only_to_selected_unicode_range() {
        let mut app = FlowriteApp::empty();
        app.blocks[0].text = "é blue text".into();
        app.selection = Some((1, 2, 6));
        app.apply_color(ColorAction::Text(Some([100, 180, 220])));
        let styles = app.blocks[0].character_styles();
        assert_eq!(styles[1].color, None);
        assert_eq!(styles[2].color, Some([100, 180, 220]));
        assert_eq!(styles[6].color, None);
        app.apply_color(ColorAction::Clear);
        assert_eq!(
            app.blocks[0].character_styles(),
            vec![InlineStyle::default(); 11]
        );
    }
}
