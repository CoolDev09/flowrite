//! Flaticon Uicons, Regular Rounded. Vendor SVGs are generated from the official
//! npm package by scripts/prepare_icons.py; no network access at runtime.
use super::*;

macro_rules! icons {
    ($($variant:ident => $name:literal),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[allow(dead_code)]
        pub(super) enum Icon { $($variant),+ }
        impl Icon {
            #[cfg(test)]
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
            pub fn name(self) -> &'static str { match self { $(Self::$variant => $name),+ } }
            fn bytes(self) -> &'static [u8] {
                match self { $(Self::$variant => include_bytes!(concat!("../assets/icons/vendor/", $name, ".svg"))),+ }
            }
        }
    };
}
icons! {
    Undo => "arrow-left", Redo => "arrow-right", Trash => "trash", Search => "search",
    Save => "disk", Settings => "settings", Sidebar => "sidebar", Image => "picture",
    Text => "text", List => "list", Checklist => "list-check", Quote => "quote-right",
    Info => "info", Tip => "bulb", Warning => "triangle-warning", Important => "exclamation",
    Copy => "copy", Close => "cross", Plus => "plus", More => "menu-dots",
    Export => "file-export", Folder => "folder", Open => "folder-open", Rename => "pencil",
    Bold => "bold", Italic => "italic", Underline => "underline", Spacing => "interlining",
    Document => "document", Citations => "book", Check => "check", Code => "code-simple",
    Heading => "heading", Divider => "horizontal-rule", PageSetup => "layout-fluid",
    Color => "palette", Import => "file-import", Up => "arrow-up", Down => "arrow-down",
}

pub(super) const DANGER: Color32 = Color32::from_rgb(235, 112, 112);
pub(super) const LICENSE: &str = include_str!("../assets/icons/vendor/LICENSE");

impl Icon {
    fn texture(self, ctx: &egui::Context, points: f32) -> egui::TextureHandle {
        // Twice the physical display resolution, cached per icon and DPI bucket.
        let pixels = (points * ctx.pixels_per_point() * 2.0)
            .ceil()
            .clamp(24.0, 256.0) as u32;
        let id = egui::Id::new(("flowrite-uicon", self.name(), pixels));
        if let Some(texture) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
            return texture;
        }
        let mut image = egui_extras::image::load_svg_bytes_with_size(
            self.bytes(),
            Some(egui::load::SizeHint::Size(pixels, pixels)),
        )
        .expect("Bundled SVG must pass icon validation tests");
        for pixel in &mut image.pixels {
            *pixel = Color32::from_white_alpha(pixel.a());
        }
        let texture = ctx.load_texture(
            format!("uicon-{}-{pixels}", self.name()),
            image,
            egui::TextureOptions::LINEAR,
        );
        ctx.data_mut(|d| d.insert_temp(id, texture.clone()));
        texture
    }
    pub fn image(self, ui: &egui::Ui, color: Color32) -> egui::Image<'static> {
        let texture = self.texture(ui.ctx(), 17.0);
        egui::Image::new((texture.id(), Vec2::splat(17.0))).tint(color)
    }
}

/// Consistent icon control; labelled for screen readers even when visually compact.
pub(super) fn control(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    hint: &str,
    enabled: bool,
    selected: bool,
) -> egui::Response {
    let enabled = enabled && ui.is_enabled();
    let color = if enabled {
        ui.visuals().text_color()
    } else {
        theme::current().muted.gamma_multiply(0.5)
    };
    let button = egui::Button::image(icon.image(ui, color))
        .min_size(Vec2::splat(28.0))
        .rounding(5.0)
        .selected(selected);
    let response = ui
        .scope(|ui| {
            ui.spacing_mut().button_padding = Vec2::new(6.0, 5.0);
            if theme::current().stroke != Color32::from_gray(120) {
                ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::NONE;
                ui.visuals_mut().widgets.noninteractive.bg_stroke = Stroke::NONE;
            }
            ui.add_enabled(enabled, button)
        })
        .inner
        .hand()
        .on_hover_text(hint);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    response
}
pub(super) fn labelled(ui: &mut egui::Ui, icon: Icon, label: &str) -> egui::Response {
    labelled_enabled(ui, icon, label, true)
}
pub(super) fn labelled_enabled(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    enabled: bool,
) -> egui::Response {
    let (label, shortcut) = label.split_once("    ").unwrap_or((label, ""));
    let button = egui::Button::image_and_text(icon.image(ui, ui.visuals().text_color()), label)
        .shortcut_text(shortcut)
        .min_size(Vec2::new(0.0, 28.0))
        .rounding(5.0);
    ui.add_enabled(enabled, button).hand()
}
pub(super) fn danger(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let button = egui::Button::image_and_text(
        Icon::Trash.image(ui, DANGER),
        RichText::new(label).color(DANGER),
    )
    .min_size(Vec2::new(0.0, 30.0));
    ui.add(button).hand()
}
pub(super) fn menu<R>(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    content: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    let image = icon.image(ui, ui.visuals().text_color());
    let nested = ui.stack().contained_in(egui::UiKind::Menu);
    let mut result = if nested && !label.is_empty() {
        // egui's submenu path ignores images. Reserve a leading icon slot,
        // then paint the same cached texture without replacing hover behavior.
        let result = ui.menu_button(format!("        {label}"), content);
        let rect = egui::Rect::from_center_size(
            Pos2::new(
                result.response.rect.left() + 12.5,
                result.response.rect.center().y,
            ),
            Vec2::splat(17.0),
        );
        ui.painter().image(
            icon.texture(ui.ctx(), 17.0).id(),
            rect,
            egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            ui.visuals().text_color(),
        );
        result
    } else if label.is_empty() {
        ui.menu_image_button(image, content)
    } else {
        ui.menu_image_text_button(image, label, content)
    };
    let caption = if label.is_empty() {
        match icon {
            Icon::Image => "Insert image",
            Icon::Color => "Text color and highlight",
            Icon::Spacing => "Line spacing",
            Icon::More => "Page actions",
            Icon::Document => "File menu · new, open, save and settings",
            Icon::Export => "Export document · PDF or Markdown",
            Icon::Info | Icon::Tip | Icon::Warning | Icon::Important => "Change callout type",
            _ => icon.name(),
        }
    } else {
        label
    };
    result.response = result.response.hand().on_hover_text(caption);
    result.response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), caption)
    });
    result
}
pub(super) fn callout(kind: CalloutKind) -> Icon {
    match kind {
        CalloutKind::Note | CalloutKind::Neutral => Icon::Info,
        CalloutKind::Tip => Icon::Tip,
        CalloutKind::Warning => Icon::Warning,
        CalloutKind::Important => Icon::Important,
    }
}
pub(super) fn block(kind: BlockKind) -> Icon {
    match kind {
        BlockKind::Text => Icon::Text,
        BlockKind::Heading1 | BlockKind::Heading2 => Icon::Heading,
        BlockKind::Bulleted | BlockKind::Numbered => Icon::List,
        BlockKind::Todo => Icon::Checklist,
        BlockKind::Quote => Icon::Quote,
        BlockKind::Code => Icon::Code,
        BlockKind::Callout => Icon::Info,
        BlockKind::Divider => Icon::Divider,
        BlockKind::Image => Icon::Image,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_names_are_hidden_until_hovered() {
        let ctx = egui::Context::default();
        let mut bounds = egui::Rect::NOTHING;
        let output = ctx.run(egui::RawInput {time:Some(0.0),..Default::default()}, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                bounds = control(ui, Icon::Save, "Save", "Save document · Ctrl+S", true, false).rect;
            });
        });
        assert!(!output.shapes.iter().any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text.contains("Save"))));
        let point = bounds.center();
        let mut shown = false;
        for frame in 0..45 {
            let output = ctx.run(egui::RawInput {time:Some(1.0+frame as f64*0.02),events:vec![egui::Event::PointerMoved(point)],..Default::default()}, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {control(ui, Icon::Save, "Save", "Save document · Ctrl+S", true, false);});
            });
            shown |= output.shapes.iter().any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text=="Save document · Ctrl+S"));
        }
        assert!(shown, "The hover tooltip must expose the action name and shortcut");
    }
    #[test]
    fn every_icon_parses_and_has_provenance() {
        let credits = include_str!("../assets/icons/ICONS.md");
        for &icon in Icon::ALL {
            let image = egui_extras::image::load_svg_bytes_with_size(
                icon.bytes(),
                Some(egui::load::SizeHint::Size(48, 48)),
            )
            .unwrap();
            assert_eq!(image.size, [48, 48]);
            assert!(
                image.pixels.iter().filter(|p| p.a() > 0).count() > 30,
                "Empty {} icon",
                icon.name()
            );
            assert!(credits.contains(&format!("| {}.svg |", icon.name())));
        }
    }
    #[test]
    fn textures_are_reused_and_disabled_controls_keep_arrow_cursor() {
        let ctx = egui::Context::default();
        let a = Icon::Undo.texture(&ctx, 17.0);
        let b = Icon::Undo.texture(&ctx, 17.0);
        assert_eq!(a.id(), b.id());
        let mut rect = egui::Rect::NOTHING;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                rect = control(ui, Icon::Undo, "Undo", "Undo · Ctrl+Z", false, false).rect;
            });
        });
        let output = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::PointerMoved(rect.center())],
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    control(ui, Icon::Undo, "Undo", "Undo · Ctrl+Z", false, false);
                });
            },
        );
        assert_eq!(
            output.platform_output.cursor_icon,
            egui::CursorIcon::Default
        );
    }

    #[test]
    fn icon_controls_activate_by_mouse_and_keyboard_and_menus_open() {
        let ctx = egui::Context::default();
        let mut bounds = egui::Rect::NOTHING;
        let mut id = egui::Id::NULL;
        let mut clicked = false;
        let mut draw = |ctx: &egui::Context| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let response = control(ui, Icon::Save, "Save", "Save · Ctrl+S", true, false);
                bounds = response.rect;
                id = response.id;
                clicked |= response.clicked();
            });
        };
        let _ = ctx.run(egui::RawInput::default(), &mut draw);
        drop(draw);
        let point = bounds.center();
        let input = egui::RawInput {
            events: vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                clicked |= control(ui, Icon::Save, "Save", "Save", true, false).clicked();
            });
        });
        assert!(clicked);
        clicked = false;
        ctx.memory_mut(|m| m.request_focus(id));
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    clicked |= control(ui, Icon::Save, "Save", "Save", true, false).clicked();
                });
            },
        );
        assert!(clicked);

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                bounds = menu(ui, Icon::Image, "", |ui| {
                    ui.label("Frame choices");
                })
                .response
                .rect;
            });
        });
        let point = bounds.center();
        let mut open = false;
        let mut painted = false;
        for events in [
            vec![egui::Event::PointerMoved(point)],
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            vec![],
        ] {
            open = false;
            let output = ctx.run(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        // egui 0.29's root-menu inner return is None even while
                        // its content is drawn. Verify actual content instead.
                        menu(ui, Icon::Image, "", |ui| {
                            open = true;
                            ui.label("Frame choices");
                        });
                    });
                },
            );
            painted = output.shapes.iter().any(
                |s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text=="Frame choices"),
            );
        }
        assert!(open);
        assert!(painted);
    }
}
