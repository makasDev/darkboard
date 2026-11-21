//! The board document and its undo/redo history.

use crate::canvas::stroke::Stroke;

#[derive(Default)]
pub struct Document {
    strokes: Vec<Stroke>,
}

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn strokes(&self) -> &[Stroke] {
        &self.strokes
    }

    pub fn len(&self) -> usize {
        self.strokes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strokes.is_empty()
    }

    pub fn push(&mut self, stroke: Stroke) {
        self.strokes.push(stroke);
    }

    pub fn remove(&mut self, index: usize) -> Stroke {
        self.strokes.remove(index)
    }

    pub fn pop(&mut self) -> Option<Stroke> {
        self.strokes.pop()
    }

    /// Remove every stroke and hand them over to the caller.
    pub fn take_all(&mut self) -> Vec<Stroke> {
        std::mem::take(&mut self.strokes)
    }

    pub fn replace(&mut self, strokes: Vec<Stroke>) {
        self.strokes = strokes;
    }

    /// Replace the strokes with sanitized copies, dropping unusable ones.
    /// Returns how many were dropped.
    pub fn replace_sanitized(&mut self, mut strokes: Vec<Stroke>) -> usize {
        let before = strokes.len();
        strokes.retain_mut(|stroke| stroke.sanitize());
        self.strokes = strokes;
        before - self.strokes.len()
    }
}

/// Undo entries never duplicate stroke data that is still in the document:
/// `Add` only needs to pop, and `Clear` owns the strokes it removed.
enum UndoEdit {
    Add,
    Clear(Vec<Stroke>),
    Erase(Vec<Stroke>),
}

/// Redo entries own the data needed to put something back.
enum RedoEdit {
    Add(Stroke),
    Clear(Vec<Stroke>),
    Erase(Vec<Stroke>),
}

#[derive(Default)]
pub struct History {
    undo: Vec<UndoEdit>,
    redo: Vec<RedoEdit>,
}

impl History {
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Record a stroke that has just been appended to the document.
    pub fn record_add(&mut self) {
        self.undo.push(UndoEdit::Add);
        self.redo.clear();
    }

    /// Record a clear. The strokes must already have been taken from the document.
    pub fn record_clear(&mut self, strokes: Vec<Stroke>) {
        if strokes.is_empty() {
            return;
        }
        self.undo.push(UndoEdit::Clear(strokes));
        self.redo.clear();
    }

    /// Record an erase. The strokes must already have been removed from the document.
    pub fn record_erase(&mut self, strokes: Vec<Stroke>) {
        if strokes.is_empty() {
            return;
        }
        self.undo.push(UndoEdit::Erase(strokes));
        self.redo.clear();
    }

    pub fn undo(&mut self, document: &mut Document) -> bool {
        let Some(edit) = self.undo.pop() else {
            return false;
        };

        match edit {
            UndoEdit::Add => match document.pop() {
                Some(stroke) => {
                    self.redo.push(RedoEdit::Add(stroke));
                    true
                }
                None => false,
            },
            UndoEdit::Clear(strokes) => {
                let current = document.take_all();
                document.replace(strokes);
                self.redo.push(RedoEdit::Clear(current));
                true
            }
            UndoEdit::Erase(strokes) => {
                self.redo.push(RedoEdit::Erase(strokes.clone()));
                for stroke in strokes {
                    document.push(stroke);
                }
                true
            }
        }
    }

    pub fn redo(&mut self, document: &mut Document) -> bool {
        let Some(edit) = self.redo.pop() else {
            return false;
        };

        match edit {
            RedoEdit::Add(stroke) => {
                document.push(stroke);
                self.undo.push(UndoEdit::Add);
            }
            RedoEdit::Clear(strokes) => {
                let current = document.take_all();
                document.replace(strokes);
                self.undo.push(UndoEdit::Clear(current));
            }
            RedoEdit::Erase(strokes) => {
                let mut i = 0;
                while i < document.len() {
                    if strokes.iter().any(|r| r == &document.strokes()[i]) {
                        document.remove(i);
                    } else {
                        i += 1;
                    }
                }
                self.undo.push(UndoEdit::Erase(strokes));
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    fn stroke(x: f32) -> Stroke {
        Stroke::new(vec![pos2(x, 0.0), pos2(x + 1.0, 1.0)], 2.0)
    }

    #[test]
    fn undo_and_redo_add() {
        let mut document = Document::new();
        let mut history = History::default();

        document.push(stroke(0.0));
        history.record_add();

        assert!(history.undo(&mut document));
        assert!(document.is_empty());
        assert!(history.can_redo());

        assert!(history.redo(&mut document));
        assert_eq!(document.len(), 1);
        assert!(!history.can_redo());
    }

    #[test]
    fn undo_redo_clear() {
        let mut document = Document::new();
        let mut history = History::default();

        for i in 0..3 {
            document.push(stroke(i as f32));
            history.record_add();
        }

        let cleared = document.take_all();
        history.record_clear(cleared);
        assert!(document.is_empty());

        assert!(history.undo(&mut document));
        assert_eq!(document.len(), 3);

        assert!(history.redo(&mut document));
        assert!(document.is_empty());
    }

    #[test]
    fn new_edit_drops_redo() {
        let mut document = Document::new();
        let mut history = History::default();

        document.push(stroke(0.0));
        history.record_add();
        history.undo(&mut document);

        document.push(stroke(5.0));
        history.record_add();

        assert!(!history.can_redo());
    }

    #[test]
    fn undo_on_empty_history_is_noop() {
        let mut document = Document::new();
        let mut history = History::default();
        assert!(!history.undo(&mut document));
        assert!(!history.redo(&mut document));
    }
}
