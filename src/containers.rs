use super::*;
impl FlowriteApp {
    pub(super) fn container_editor(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        index: usize,
        width: f32,
    ) -> egui::Response {
        let kind = self.blocks[index].kind;
        if kind != BlockKind::Code && kind != BlockKind::Callout {
            if let Some(color) = self.blocks[index].background {
                return Frame::none()
                    .fill(Color32::from_rgb(color[0], color[1], color[2]))
                    .rounding(5.0)
                    .inner_margin(Margin::same(8.0))
                    .show(ui, |ui| {
                        self.block_editor(ui, ctx, index, (width - 16.0).max(100.0))
                    })
                    .inner;
            }
            return self.block_editor(ui, ctx, index, width);
        }
        let (bar, background) = if kind == BlockKind::Callout {
            self.blocks[index].callout_kind.colors()
        } else {
            ([70, 72, 78], [32, 34, 38])
        };
        let background = self.blocks[index].background.unwrap_or(background);
        let frame = Frame::none()
            .fill(Color32::from_rgb(
                background[0],
                background[1],
                background[2],
            ))
            .stroke(Stroke::new(
                1.0_f32,
                Color32::from_rgb(bar[0], bar[1], bar[2]).gamma_multiply(0.45),
            ))
            .rounding(8.0)
            .inner_margin(Margin::same(if kind == BlockKind::Code {
                12.0
            } else {
                16.0
            }));
        let result = frame.show(ui, |ui| {
            ui.with_layout(
                if kind == BlockKind::Code {
                    Layout::top_down(Align::LEFT)
                } else {
                    *ui.layout()
                },
                |ui| {
                    ui.set_width((width - 32.0).max(100.0));
                    ui.horizontal(|ui| {
                        if kind == BlockKind::Code {
                            egui::ComboBox::from_id_salt(("language", self.blocks[index].id))
                                .selected_text(if self.blocks[index].language.is_empty() {
                                    "Plain text"
                                } else {
                                    &self.blocks[index].language
                                })
                                .show_ui(ui, |ui| {
                                    for (label, name) in [
                                        ("Plain text", ""),
                                        ("Rust", "rust"),
                                        ("Python", "python"),
                                        ("JavaScript", "javascript"),
                                        ("TypeScript", "typescript"),
                                        ("HTML", "html"),
                                        ("CSS", "css"),
                                        ("JSON", "json"),
                                        ("SQL", "sql"),
                                        ("Bash", "bash"),
                                    ] {
                                        if ui
                                            .selectable_value(
                                                &mut self.blocks[index].language,
                                                name.into(),
                                                label,
                                            )
                                            .hand()
                                            .changed()
                                        {
                                            self.dirty = true;
                                        }
                                    }
                                })
                                .response
                                .hand();
                            if ui
                                .checkbox(&mut self.blocks[index].code_wrap, "Wrap")
                                .hand()
                                .changed()
                            {
                                self.dirty = true;
                            }
                            let copied = ctx
                                .data(|d| {
                                    d.get_temp::<f64>(egui::Id::new((
                                        "copied",
                                        self.blocks[index].id,
                                    )))
                                })
                                .is_some_and(|t| ctx.input(|i| i.time) - t < 1.5);
                            if icons::control(
                                ui,
                                if copied { Icon::Check } else { Icon::Copy },
                                if copied { "Copied" } else { "Copy" },
                                if copied { "Copied to clipboard" } else { "Copy code to clipboard" },
                                true,
                                copied,
                            )
                            .clicked()
                            {
                                ctx.copy_text(self.blocks[index].text.clone());
                                ctx.data_mut(|d| {
                                    d.insert_temp(
                                        egui::Id::new(("copied", self.blocks[index].id)),
                                        ctx.input(|i| i.time),
                                    )
                                });
                                ctx.request_repaint_after(std::time::Duration::from_millis(1500));
                            }
                        } else {
                            icons::menu(
                                ui,
                                icons::callout(self.blocks[index].callout_kind),
                                "",
                                |ui| {
                                    for candidate in CalloutKind::ALL {
                                        if ui
                                            .selectable_value(
                                                &mut self.blocks[index].callout_kind,
                                                candidate,
                                                candidate.label(),
                                            )
                                            .hand()
                                            .changed()
                                        {
                                            self.dirty = true;
                                        }
                                    }
                                },
                            )
                            .response
                            .hand();
                        }
                    });
                    ui.add_space(6.0);
                    if kind == BlockKind::Code && !self.blocks[index].code_wrap {
                        egui::ScrollArea::horizontal()
                            .id_salt(("code_scroll", self.blocks[index].id))
                            .show(ui, |ui| self.block_editor(ui, ctx, index, f32::INFINITY))
                            .inner
                    } else {
                        self.block_editor(ui, ctx, index, ui.available_width())
                    }
                },
            )
            .inner
        });
        if kind == BlockKind::Callout {
            let rect = egui::Rect::from_min_max(
                result.response.rect.min,
                result.response.rect.left_bottom() + Vec2::new(4.0, 0.0),
            );
            ui.painter()
                .rect_filled(rect, 2.0, Color32::from_rgb(bar[0], bar[1], bar[2]));
        }
        result.inner
    }
}
