use super::*;
use egui::text::{CCursor, CCursorRange};
use std::sync::Arc;

impl FlowriteApp {
    pub(super) fn divider_editor(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        index: usize,
        width: f32,
    ) -> egui::Response {
        let id = self.blocks[index].id;
        let widget = egui::Id::new(("block", &self.active_document, id));
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 28.0), Sense::hover());
        let response = ui.interact(rect, widget, Sense::click()).hand();
        if self.focus_request.is_some_and(|(target, _)| target == id) {
            response.request_focus();
            self.focus_request = None;
        }
        if response.clicked() {
            response.request_focus();
        }
        if response.has_focus() {
            ctx.memory_mut(|m| {
                m.set_focus_lock_filter(
                    widget,
                    egui::EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..Default::default()
                    },
                )
            });
            if response.gained_focus() {
                ctx.request_discard("Settle divider keyboard focus");
            }
            self.focused_block = Some(id);
            self.selection = Some((id, 0, 0));
            ui.painter()
                .rect_stroke(rect, 3.0, Stroke::new(1.0_f32, theme::current().accent));
        }
        ui.painter().line_segment(
            [
                Pos2::new(rect.left(), rect.center().y),
                Pos2::new(rect.right(), rect.center().y),
            ],
            Stroke::new(1.0_f32, theme::current().stroke),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Divider block")
        });
        let _ = ctx;
        response
    }
    pub(super) fn block_shortcuts(&mut self, ctx: &egui::Context) {
        if self.show_home || self.slash_target.is_some() || self.block_picker_target.is_some() {
            return;
        }
        let Some(index) = self.blocks.iter().position(|b| {
            ctx.memory(|m| m.has_focus(egui::Id::new(("block", &self.active_document, b.id))))
        }) else {
            return;
        };
        let widget = egui::Id::new(("block", &self.active_document, self.blocks[index].id));
        if matches!(
            self.blocks[index].kind,
            BlockKind::Image | BlockKind::Divider
        ) {
            if ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace)
                    || i.consume_key(egui::Modifiers::NONE, egui::Key::Delete)
            }) {
                if self.blocks.len() > 1 {
                    self.blocks.remove(index);
                } else {
                    self.blocks[index] = Block::new(self.blocks[index].id, BlockKind::Text, "");
                }
                let target = index.min(self.blocks.len() - 1);
                self.focus_request = Some((self.blocks[target].id, 0));
                self.dirty = true;
                self.force_history = true;
            } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                let block = self.make_block(BlockKind::Text, "");
                let id = block.id;
                self.blocks.insert(index + 1, block);
                self.focus_request = Some((id, 0));
                self.dirty = true;
                self.force_history = true;
            }
            return;
        }
        let Some(range) = TextEdit::load_state(ctx, widget).and_then(|s| s.cursor.char_range())
        else {
            return;
        };
        let empty_selection = range.primary.index == range.secondary.index;
        let cursor = range.primary.index;
        let len = self.blocks[index].text.chars().count();
        let kind = self.blocks[index].kind;
        let keys = ctx.input_mut(|i| {
            (
                i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::ArrowUp,
                ),
                i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::ArrowDown,
                ),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::ArrowUp),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::ArrowDown),
            )
        });
        if keys.0 || keys.1 {
            self.move_block_tree(index, keys.1);
            return;
        }
        if keys.2 || keys.3 {
            if let Some(block) = if keys.2 {
                self.blocks.first()
            } else {
                self.blocks.last()
            } {
                self.focus_request = Some((
                    block.id,
                    if keys.2 {
                        0
                    } else {
                        block.text.chars().count()
                    },
                ));
                self.scroll_target = Some(block.id);
            }
            return;
        }
        if matches!(
            kind,
            BlockKind::Bulleted | BlockKind::Numbered | BlockKind::Todo | BlockKind::Code
        ) {
            let tab = ctx.input_mut(|i| {
                // A quick chord can release Shift before the next UI frame.
                // Use the Tab event's modifiers, not the final modifier snapshot.
                let shift = i
                    .events
                    .iter()
                    .find_map(|event| match event {
                        egui::Event::Key {
                            key: egui::Key::Tab,
                            pressed: true,
                            modifiers,
                            ..
                        } if !modifiers.command && !modifiers.ctrl && !modifiers.alt => {
                            Some(modifiers.shift)
                        }
                        _ => None,
                    })
                    .unwrap_or(false);
                i.consume_key(
                    if shift {
                        egui::Modifiers::SHIFT
                    } else {
                        egui::Modifiers::NONE
                    },
                    egui::Key::Tab,
                )
                .then_some(shift)
            });
            if let Some(shift) = tab {
                if kind == BlockKind::Code {
                    if shift {
                        let chars: Vec<_> = self.blocks[index].text.chars().collect();
                        let line_start = chars[..cursor]
                            .iter()
                            .rposition(|&c| c == '\n')
                            .map_or(0, |p| p + 1);
                        let remove = if chars.get(line_start) == Some(&'\t') {
                            1
                        } else {
                            chars[line_start..]
                                .iter()
                                .take(4)
                                .take_while(|&&c| c == ' ')
                                .count()
                        };
                        if remove > 0 {
                            self.insert_at(index, line_start, line_start + remove, "");
                            self.focus_request = Some((
                                self.blocks[index].id,
                                cursor.saturating_sub(remove).max(line_start),
                            ));
                        }
                    } else {
                        let [start, end] = range.sorted();
                        self.insert_at(index, start.index, end.index, "    ");
                    }
                } else if shift {
                    self.blocks[index].indent = self.blocks[index].indent.saturating_sub(1);
                    self.dirty = true;
                } else {
                    self.blocks[index].indent = (self.blocks[index].indent + 1).min(6);
                    self.dirty = true;
                }
                // Tab belongs to the editor, not egui's next-widget traversal.
                ctx.memory_mut(|m| m.request_focus(widget));
                if kind != BlockKind::Code {
                    let mut state = TextEdit::load_state(ctx, widget).unwrap_or_default();
                    state.cursor.set_char_range(Some(range));
                    state.store(ctx, widget);
                }
                self.force_history = true;
                return;
            }
        }
        if matches!(kind, BlockKind::Code | BlockKind::Callout) {
            let exit = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter))
                || (empty_selection
                    && cursor == len
                    && self.blocks[index].text.ends_with('\n')
                    && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)));
            if exit {
                if self.blocks[index].text.ends_with('\n') {
                    self.blocks[index].text.pop();
                    let styles = self.blocks[index].character_styles();
                    self.blocks[index].set_character_styles(&styles);
                }
                let next = self.make_block(BlockKind::Text, "");
                let id = next.id;
                self.blocks.insert(index + 1, next);
                self.focus_request = Some((id, 0));
                self.dirty = true;
                self.force_history = true;
                return;
            }
        }
        if self.settings.editor.markdown
            && self.blocks[index].text == "```"
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        {
            self.blocks[index].kind = BlockKind::Code;
            self.blocks[index].text.clear();
            self.dirty = true;
            self.force_history = true;
            return;
        }
        if empty_selection
            && cursor == 0
            && ctx.input(|i| {
                i.modifiers == egui::Modifiers::NONE && i.key_pressed(egui::Key::Backspace)
            })
        {
            if self.blocks[index].text.is_empty()
                && (self.blocks[index].kind == BlockKind::Callout || self.blocks[index].indent > 0)
            {
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace));
                self.blocks[index].kind = BlockKind::Text;
                self.blocks[index].indent = 0;
                self.dirty = true;
                return;
            }
            if index > 0 {
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace));
                if matches!(
                    self.blocks[index - 1].kind,
                    BlockKind::Divider | BlockKind::Image
                ) {
                    self.focus_request = Some((self.blocks[index - 1].id, 0));
                } else {
                    self.merge_blocks(index - 1, index);
                }
            }
        } else if empty_selection
            && cursor == len
            && index + 1 < self.blocks.len()
            && ctx
                .input(|i| i.modifiers == egui::Modifiers::NONE && i.key_pressed(egui::Key::Delete))
        {
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete));
            if matches!(
                self.blocks[index + 1].kind,
                BlockKind::Divider | BlockKind::Image
            ) {
                self.focus_request = Some((self.blocks[index + 1].id, 0));
            } else {
                self.merge_blocks(index, index + 1);
            }
        }
    }
    pub(super) fn insert_at(&mut self, index: usize, start: usize, end: usize, text: &str) {
        let block = &mut self.blocks[index];
        let chars: Vec<_> = block.text.chars().collect();
        let start = start.min(chars.len());
        let end = end.max(start).min(chars.len());
        let mut styles = block.character_styles();
        let style = styles
            .get(start.saturating_sub(1))
            .copied()
            .unwrap_or(block.format);
        block.text = chars[..start].iter().collect::<String>()
            + text
            + &chars[end..].iter().collect::<String>();
        styles.splice(start..end, std::iter::repeat_n(style, text.chars().count()));
        block.set_character_styles(&styles);
        self.focus_request = Some((block.id, start + text.chars().count()));
        self.dirty = true;
        self.force_history = true;
    }
    pub(super) fn merge_blocks(&mut self, left: usize, right: usize) {
        let next = self.blocks.remove(right);
        let block = &mut self.blocks[left];
        let caret = block.text.chars().count();
        let mut styles = block.character_styles();
        styles.extend(next.character_styles());
        block.text.push_str(&next.text);
        block.link_ids.extend(next.link_ids);
        block.set_character_styles(&styles);
        self.focus_request = Some((block.id, caret));
        self.focused_block = Some(block.id);
        self.dirty = true;
        self.force_history = true;
    }
    pub(super) fn move_block_tree(&mut self, index: usize, down: bool) {
        let indent = self.blocks[index].indent;
        let end = (index + 1..self.blocks.len())
            .find(|&i| self.blocks[i].indent <= indent)
            .unwrap_or(self.blocks.len());
        if down && end < self.blocks.len() {
            let next_end = (end + 1..self.blocks.len())
                .find(|&i| self.blocks[i].indent <= self.blocks[end].indent)
                .unwrap_or(self.blocks.len());
            self.blocks[index..next_end].rotate_left(end - index);
            self.dirty = true;
        } else if !down && index > 0 {
            let previous = (0..index)
                .rev()
                .find(|&i| self.blocks[i].indent <= indent)
                .unwrap_or(0);
            self.blocks[previous..end].rotate_right(end - index);
            self.dirty = true;
        }
        if self.dirty {
            if let Some(id) = self.focused_block {
                self.focus_request = Some((id, self.selection.map(|(_, _, end)| end).unwrap_or(0)));
            }
            self.force_history = true;
        }
    }
    /// Let TextEdit handle movement inside a block; cross only at its visual edges.
    pub(super) fn navigate_blocks(&mut self, ctx: &egui::Context) {
        if self.show_home || self.slash_target.is_some() || self.block_picker_target.is_some() {
            return;
        }
        let Some(index) = self.blocks.iter().position(|block| {
            ctx.memory(|m| m.has_focus(egui::Id::new(("block", &self.active_document, block.id))))
        }) else {
            return;
        };
        let key = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } if *modifiers == egui::Modifiers::NONE
                    && matches!(
                        key,
                        egui::Key::ArrowUp
                            | egui::Key::ArrowDown
                            | egui::Key::ArrowLeft
                            | egui::Key::ArrowRight
                    ) =>
                {
                    Some(*key)
                }
                _ => None,
            })
        });
        let Some(key) = key else { return };
        if matches!(
            self.blocks[index].kind,
            BlockKind::Image | BlockKind::Divider
        ) {
            let backwards = matches!(key, egui::Key::ArrowLeft | egui::Key::ArrowUp);
            let target = if backwards {
                index.checked_sub(1)
            } else {
                (index + 1 < self.blocks.len()).then_some(index + 1)
            };
            if let Some(target) = target {
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key));
                let block = &self.blocks[target];
                let widget = egui::Id::new(("block", &self.active_document, block.id));
                let cursor = if backwards {
                    block.text.chars().count()
                } else {
                    0
                };
                let mut state = TextEdit::load_state(ctx, widget).unwrap_or_default();
                state
                    .cursor
                    .set_char_range(Some(CCursorRange::one(CCursor::new(cursor))));
                state.store(ctx, widget);
                ctx.memory_mut(|m| m.request_focus(widget));
                self.focus_request = None;
                self.focused_block = Some(block.id);
                self.selection = Some((block.id, cursor, cursor));
                self.scroll_target = Some(block.id);
            }
            return;
        }
        let widget_id = egui::Id::new(("block", &self.active_document, self.blocks[index].id));
        let Some(range) = TextEdit::load_state(ctx, widget_id).and_then(|s| s.cursor.char_range())
        else {
            return;
        };
        // Preserve TextEdit's selection collapsing and Shift/Ctrl navigation.
        if range.primary.index != range.secondary.index {
            return;
        }
        let Some(galley) = ctx.data(|d| d.get_temp::<Arc<egui::Galley>>(widget_id.with("layout")))
        else {
            return;
        };
        let cursor = galley.from_ccursor(range.primary);
        let backwards = match key {
            egui::Key::ArrowLeft if range.primary.index == 0 => true,
            egui::Key::ArrowRight
                if range.primary.index == self.blocks[index].text.chars().count() =>
            {
                false
            }
            egui::Key::ArrowUp if cursor.rcursor.row == 0 => true,
            egui::Key::ArrowDown if cursor.rcursor.row + 1 >= galley.rows.len() => false,
            _ => return,
        };
        let target = if backwards {
            index.checked_sub(1)
        } else {
            (index + 1 < self.blocks.len()).then_some(index + 1)
        };
        let Some(target) = target else { return };
        let block = &self.blocks[target];
        let target_id = egui::Id::new(("block", &self.active_document, block.id));
        let target_cursor = if matches!(key, egui::Key::ArrowUp | egui::Key::ArrowDown) {
            ctx.data(|d| d.get_temp::<Arc<egui::Galley>>(target_id.with("layout")))
                .and_then(|target_galley| {
                    let row = if backwards {
                        target_galley.rows.last()
                    } else {
                        target_galley.rows.first()
                    }?;
                    let x = galley.pos_from_cursor(&cursor).center().x;
                    Some(
                        target_galley
                            .cursor_from_pos(Vec2::new(x, row.rect.center().y))
                            .ccursor,
                    )
                })
                .unwrap_or(CCursor::new(0))
        } else if backwards {
            CCursor::new(block.text.chars().count())
        } else {
            CCursor::new(0)
        };
        ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, key);
        });
        let mut target_state = TextEdit::load_state(ctx, target_id).unwrap_or_default();
        target_state
            .cursor
            .set_char_range(Some(CCursorRange::one(target_cursor)));
        target_state.store(ctx, target_id);
        ctx.memory_mut(|m| m.request_focus(target_id));
        self.focus_request = None;
        self.focused_block = Some(block.id);
        self.selection = Some((block.id, target_cursor.index, target_cursor.index));
        self.scroll_target = Some(block.id);
        // Keep row affinity as well as the Unicode offset at wrapped line boundaries.
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn merge_keeps_unicode_and_styles() {
        let mut app = FlowriteApp::empty();
        app.blocks = vec![
            Block::new(1, BlockKind::Text, "é "),
            Block::new(2, BlockKind::Text, "bold"),
        ];
        app.blocks[1].format.bold = true;
        app.merge_blocks(0, 1);
        assert_eq!(app.blocks[0].text, "é bold");
        assert!(app.blocks[0].character_styles()[2].bold);
        assert_eq!(app.focus_request, Some((1, 2)));
    }
    #[test]
    fn moving_parent_keeps_children() {
        let mut app = FlowriteApp::empty();
        app.blocks = (1..=4)
            .map(|id| Block::new(id, BlockKind::Bulleted, "item"))
            .collect();
        app.blocks[1].indent = 1;
        app.move_block_tree(0, true);
        assert_eq!(
            app.blocks.iter().map(|b| b.id).collect::<Vec<_>>(),
            vec![3, 1, 2, 4]
        );
    }
}
