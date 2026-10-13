use super::*;

fn copy_feedback_active(ctx: &egui::Context, id: egui::Id, now: f64) -> bool {
    ctx.data(|d| d.get_temp::<f64>(id))
        .is_some_and(|copied_at| now - copied_at < 1.5)
}

fn code_copy_control(ui: &mut egui::Ui, ctx: &egui::Context, block: &Block) -> egui::Response {
    let id = egui::Id::new(("copied", block.id));
    // Context accessors share a lock: never call ctx.input inside data_mut.
    let now = ctx.input(|i| i.time);
    let copied = copy_feedback_active(ctx, id, now);
    let response = icons::control(
        ui,
        if copied { Icon::Check } else { Icon::Copy },
        if copied { "Copied" } else { "Copy" },
        if copied {
            "Copied to clipboard"
        } else {
            "Copy code to clipboard"
        },
        true,
        copied,
    );
    #[cfg(test)]
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::new(("code_copy_bounds", block.id)), response.rect)
    });
    if response.clicked() {
        ctx.copy_text(block.text.clone());
        ctx.data_mut(|d| d.insert_temp(id, now));
        ctx.request_repaint_after(std::time::Duration::from_millis(1500));
    }
    response
}

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
                            let response =
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
                            design::named(
                                response,
                                egui::WidgetType::ComboBox,
                                &format!(
                                    "Code language: {}",
                                    if self.blocks[index].language.is_empty() {
                                        "Plain text"
                                    } else {
                                        &self.blocks[index].language
                                    }
                                ),
                            );
                            if ui
                                .checkbox(&mut self.blocks[index].code_wrap, "Wrap")
                                .hand()
                                .changed()
                            {
                                self.dirty = true;
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                code_copy_control(ui, ctx, &self.blocks[index]);
                            });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_is_in_the_code_header_right_corner_and_still_copies() {
        for width in [280.0, 500.0, 780.0] {
            let ctx = egui::Context::default();
            configure_fonts(&ctx);
            let mut app = FlowriteApp::empty();
            app.blocks = vec![Block::new(
                1,
                BlockKind::Code,
                "print(\"café Ω\")\n    print(\"second line\")",
            )];
            let mut render = |events| {
                let mut body = egui::Rect::NOTHING;
                let output = ctx.run(
                    egui::RawInput {
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            body = app.container_editor(ui, ctx, 0, width).rect;
                        });
                    },
                );
                (output, body)
            };
            let (_, body) = render(vec![]);
            let copy = ctx
                .data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("code_copy_bounds", 1u64))))
                .unwrap();
            assert!(copy.top() < body.top());
            assert!(
                (copy.right() - body.right()).abs() <= 8.0,
                "Copy isn't right-aligned: {copy:?} vs {body:?}"
            );
            let (output, _) = render(click(copy.center()));
            assert_eq!(output.platform_output.copied_text, app.blocks[0].text);
        }
    }

    fn frame(
        ctx: &egui::Context,
        block: &Block,
        time: f64,
        events: Vec<egui::Event>,
    ) -> (egui::FullOutput, egui::Response) {
        let mut response = None;
        let output = ctx.run(
            egui::RawInput {
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    response = Some(code_copy_control(ui, ctx, block));
                });
            },
        );
        (output, response.unwrap())
    }

    fn click(position: Pos2) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }

    #[test]
    fn copy_click_emits_exact_code_and_feedback_expires_per_block() {
        let ctx = egui::Context::default();
        let block = Block::new(41, BlockKind::Code, "// 🦀 café\n\tprintln!(\"你好\");\n");
        let (initial, response) = frame(&ctx, &block, 10.0, vec![]);
        assert!(initial.platform_output.copied_text.is_empty());
        let (output, response) = frame(&ctx, &block, 10.1, click(response.rect.center()));
        assert!(response.clicked());
        assert_eq!(output.platform_output.copied_text, block.text);
        let id = egui::Id::new(("copied", block.id));
        assert!(copy_feedback_active(&ctx, id, 10.2));
        assert!(!copy_feedback_active(
            &ctx,
            egui::Id::new(("copied", 42_u64)),
            10.2,
        ));
        assert!(!copy_feedback_active(&ctx, id, 11.7));
        let (later, _) = frame(&ctx, &block, 11.7, vec![]);
        assert!(later.platform_output.copied_text.is_empty());
    }

    #[test]
    fn repeated_keyboard_copy_refreshes_feedback_without_changing_code() {
        let ctx = egui::Context::default();
        let block = Block::new(7, BlockKind::Code, "fn main() {}\n");
        let (_, response) = frame(&ctx, &block, 1.0, vec![]);
        for time in [1.1, 1.9] {
            ctx.memory_mut(|memory| memory.request_focus(response.id));
            let (output, activated) = frame(
                &ctx,
                &block,
                time,
                vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert!(activated.clicked());
            assert_eq!(output.platform_output.copied_text, "fn main() {}\n");
            assert_eq!(
                ctx.data(|data| data.get_temp::<f64>(egui::Id::new(("copied", 7_u64)))),
                Some(time),
            );
        }
        assert!(copy_feedback_active(
            &ctx,
            egui::Id::new(("copied", 7_u64)),
            3.0,
        ));
        assert!(!copy_feedback_active(
            &ctx,
            egui::Id::new(("copied", 7_u64)),
            3.5,
        ));
        assert_eq!(block.text, "fn main() {}\n");
    }

    #[test]
    fn empty_code_copy_completes_without_a_panic_or_lockup() {
        let ctx = egui::Context::default();
        let block = Block::new(8, BlockKind::Code, "");
        let (_, response) = frame(&ctx, &block, 2.0, vec![]);
        let (output, response) = frame(&ctx, &block, 2.1, click(response.rect.center()));
        assert!(response.clicked());
        assert!(output.platform_output.copied_text.is_empty());
        assert!(copy_feedback_active(
            &ctx,
            egui::Id::new(("copied", 8_u64)),
            2.2,
        ));
    }
}
