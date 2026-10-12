use super::*;

pub(super) const FAMILIES: [&str; 3] = ["Liberation Serif", "Liberation Sans", "Liberation Mono"];

impl FlowriteApp {
    pub(super) fn resolve_document_font(&mut self) {
        let requested = self.style.font.clone();
        let replacement = family(&requested);
        if requested != replacement {
            self.style.font = replacement.into();
            self.dirty = true;
            self.notice=Some(format!("“{requested}” is not bundled. This document uses {replacement} in the editor and PDF."));
        }
    }
}

pub(super) fn family(name: &str) -> &'static str {
    match name {
        "Times New Roman" | "Liberation Serif" => "Liberation Serif",
        "Consolas" | "JetBrains Mono" | "Liberation Mono" => "Liberation Mono",
        _ => "Liberation Sans",
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn missing_font_has_a_consistent_replacement_and_notice() {
        let mut app = crate::FlowriteApp::empty();
        app.style.font = "An unavailable font".into();
        app.resolve_document_font();
        assert_eq!(app.style.font, "Liberation Sans");
        assert!(app.notice.unwrap().contains("An unavailable font"));
    }
}

pub(super) fn data(family_name: &str, bold: bool, italic: bool) -> &'static [u8] {
    match (family(family_name), bold, italic) {
        ("Liberation Serif", false, false) => {
            include_bytes!("../assets/fonts/LiberationSerif-Regular.ttf")
        }
        ("Liberation Serif", true, false) => {
            include_bytes!("../assets/fonts/LiberationSerif-Bold.ttf")
        }
        ("Liberation Serif", false, true) => {
            include_bytes!("../assets/fonts/LiberationSerif-Italic.ttf")
        }
        ("Liberation Serif", true, true) => {
            include_bytes!("../assets/fonts/LiberationSerif-BoldItalic.ttf")
        }
        ("Liberation Mono", false, false) => {
            include_bytes!("../assets/fonts/LiberationMono-Regular.ttf")
        }
        ("Liberation Mono", true, false) => {
            include_bytes!("../assets/fonts/LiberationMono-Bold.ttf")
        }
        ("Liberation Mono", false, true) => {
            include_bytes!("../assets/fonts/LiberationMono-Italic.ttf")
        }
        ("Liberation Mono", true, true) => {
            include_bytes!("../assets/fonts/LiberationMono-BoldItalic.ttf")
        }
        (_, false, false) => include_bytes!("../assets/fonts/LiberationSans-Regular.ttf"),
        (_, true, false) => include_bytes!("../assets/fonts/LiberationSans-Bold.ttf"),
        (_, false, true) => include_bytes!("../assets/fonts/LiberationSans-Italic.ttf"),
        (_, true, true) => include_bytes!("../assets/fonts/LiberationSans-BoldItalic.ttf"),
    }
}

pub(super) fn configure(ctx: &egui::Context) {
    let mut definitions = egui::FontDefinitions::default();
    for family in FAMILIES {
        for (bold, italic, suffix) in [
            (false, false, ""),
            (true, false, " Bold"),
            (false, true, " Italic"),
            (true, true, " Bold Italic"),
        ] {
            let name = format!("{family}{suffix}");
            definitions.font_data.insert(
                name.clone(),
                egui::FontData::from_static(data(family, bold, italic)),
            );
            let mut fallback = definitions.families[&FontFamily::Proportional].clone();
            fallback.insert(0, name.clone());
            definitions
                .families
                .insert(FontFamily::Name(name.into()), fallback);
        }
    }
    definitions
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "Liberation Sans".into());
    definitions
        .families
        .get_mut(&FontFamily::Monospace)
        .unwrap()
        .insert(0, "Liberation Mono".into());
    ctx.set_fonts(definitions);
}
