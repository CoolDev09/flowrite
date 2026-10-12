use super::*;

#[derive(Clone)]
pub(super) struct UndoSnapshot {
    pub document: SavedDocument,
    pub caret: Option<(u64, usize)>,
    pub bytes: usize,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn undo_snapshot_retains_caret() {
        let mut app = FlowriteApp::empty();
        app.blocks[0].text = "é paragraph".into();
        app.selection = Some((1, 3, 3));
        let snapshot = app.history_entry();
        assert_eq!(snapshot.caret, Some((1, 3)));
        assert!(snapshot.bytes >= app.blocks[0].text.len());
    }
    #[test]
    fn unchanged_frames_do_not_add_history() {
        let mut app = FlowriteApp::empty();
        app.reset_history_baseline();
        for time in 0..100 {
            app.record_edits(time as f64);
        }
        assert!(app.undo_stack.is_empty());
    }
}

impl FlowriteApp {
    pub(super) fn history_entry(&self) -> UndoSnapshot {
        UndoSnapshot {
            document: self.local_document(),
            caret: self.selection.map(|(id, _, end)| (id, end)),
            bytes: self.title.len()
                + self
                    .blocks
                    .iter()
                    .map(|b| {
                        std::mem::size_of::<Block>()
                            + b.text.len()
                            + b.marks.len() * std::mem::size_of::<FormatSpan>()
                    })
                    .sum::<usize>()
                + self.citations.iter().map(String::len).sum::<usize>(),
        }
    }

    pub(super) fn reset_history_baseline(&mut self) {
        self.history_baseline = Some(self.history_entry());
    }

    pub(super) fn record_edits(&mut self, now: f64) {
        if !self.dirty {
            return;
        }
        let after = self.history_entry();
        if let Some(before) = self.history_baseline.take() {
            if before.document != after.document {
                self.last_document_change = now;
                if !self.history_navigation {
                    if self.force_history
                        || now - self.last_history_time > 0.75
                        || self.undo_stack.is_empty()
                    {
                        self.undo_stack.push(before);
                        let mut bytes: usize = self.undo_stack.iter().map(|s| s.bytes).sum();
                        while bytes > 50 * 1024 * 1024 && self.undo_stack.len() > 1 {
                            let oldest = self.undo_stack.remove(0);
                            bytes = bytes.saturating_sub(oldest.bytes);
                        }
                    }
                    self.last_history_time = now;
                    self.redo_stack.clear();
                }
            }
        }
        self.history_baseline = Some(after);
        // A draft becomes a library document only after a content edit.
        if self.active_document.is_empty() {
            self.commit_current();
        }
    }
}
