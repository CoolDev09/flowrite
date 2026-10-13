use super::*;

impl FlowriteApp {
    pub(super) fn toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("format_toolbar")
            .exact_height(42.0)
            .frame(
                Frame::none()
                    .fill(theme::current().panel)
                    .inner_margin(Margin::symmetric(16.0, 4.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("format_overflow")
                    .show(ui, |ui| self.format_controls(ui, ctx));
            });
    }

    fn format_controls(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            let index = self.active_index();
            let mut kind = self.blocks[index].kind;
            let response = egui::ComboBox::from_id_salt("block_type")
                .selected_text(if kind == BlockKind::Text {
                    "Paragraph"
                } else {
                    kind.label()
                })
                .width(120.0)
                .show_ui(ui, |ui| {
                    for candidate in BlockKind::ALL {
                        ui.selectable_value(
                            &mut kind,
                            candidate,
                            if candidate == BlockKind::Text {
                                "Paragraph"
                            } else {
                                candidate.label()
                            },
                        )
                        .hand();
                    }
                })
                .response
                .hand();
            design::named(
                response,
                egui::WidgetType::ComboBox,
                &format!("Block type: {}", kind.label()),
            );
            if kind != self.blocks[index].kind {
                self.apply_command(index, kind);
            }
            let old = self.style.font.clone();
            let response = egui::ComboBox::from_id_salt("font")
                .selected_text(&self.style.font)
                .width(130.0)
                .show_ui(ui, |ui| {
                    for name in fonts::FAMILIES {
                        ui.selectable_value(&mut self.style.font, name.to_owned(), name)
                            .hand();
                    }
                })
                .response
                .hand();
            design::named(
                response,
                egui::WidgetType::ComboBox,
                &format!("Font family: {}", self.style.font),
            );
            if old != self.style.font {
                self.dirty = true;
                self.force_history = true;
            }
            let response = ui.add(
                egui::DragValue::new(&mut self.style.font_size)
                    .range(8.0..=36.0)
                    .speed(0.5)
                    .suffix(" pt"),
            );
            if response.changed() {
                self.dirty = true;
                self.force_history = true;
            }
            response.widget_info(|| {
                let mut info = egui::WidgetInfo::drag_value(true, self.style.font_size as f64);
                info.label = Some("Font size in points".into());
                info
            });
            ui.separator();
            self.decoration_controls(ui, ctx);
            self.colors_menu(ui);
            icons::menu(ui, Icon::Spacing, "", |ui| {
                for (label, value) in [
                    ("Single", 1.0),
                    ("Comfortable", 1.4),
                    ("1.5", 1.5),
                    ("Double", 2.0),
                ] {
                    if ui
                        .selectable_label((self.style.line_spacing - value).abs() < 0.01, label)
                        .hand()
                        .clicked()
                    {
                        self.style.line_spacing = value;
                        self.dirty = true;
                        self.force_history = true;
                        ui.close_menu();
                    }
                }
            });
            self.image_button(ui);
        });
    }

    fn decoration_controls(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let block = &self.blocks[self.active_index()];
        let character = self
            .selection
            .filter(|(id, _, _)| *id == block.id)
            .map(|(_, start, _)| start)
            .unwrap_or(0);
        let style = block
            .character_styles()
            .get(character)
            .copied()
            .unwrap_or(block.format);
        for (icon, label, decoration, active, hint) in [
            (
                Icon::Bold,
                "Bold",
                Decoration::Bold,
                style.bold,
                "Bold · Ctrl+B",
            ),
            (
                Icon::Italic,
                "Italic",
                Decoration::Italic,
                style.italic,
                "Italic · Ctrl+I",
            ),
            (
                Icon::Underline,
                "Underline",
                Decoration::Underline,
                style.underline,
                "Underline · Ctrl+U",
            ),
        ] {
            if icons::control(ui, icon, label, hint, true, active).clicked() {
                self.toggle_decoration(decoration, ctx);
            }
        }
    }

    pub(super) fn selection_toolbar(&mut self, ctx: &egui::Context) {
        if self.show_home
            || self.settings_open
            || self.settings.editor.toolbar
            || self.focus_mode
            || !self.selection.is_some_and(|(_, a, b)| a != b)
        {
            return;
        }
        let screen = ctx.screen_rect();
        let position = self
            .selection_popup
            .unwrap_or(Pos2::new(screen.center().x - 80.0, 100.0));
        let position = Pos2::new(
            position.x.clamp(16.0, (screen.right() - 210.0).max(16.0)),
            position.y.clamp(90.0, (screen.bottom() - 70.0).max(90.0)),
        );
        egui::Area::new(egui::Id::new("selection_formatting"))
            .order(egui::Order::Foreground)
            .fixed_pos(position)
            .show(ctx, |ui| {
                Frame::popup(ui.style()).inner_margin(8.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        self.decoration_controls(ui, ctx);
                        self.colors_menu(ui);
                    });
                });
            });
    }
}
