use bmp_core::{Document, DocumentError};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
struct Snapshot(String);

impl Snapshot {
    fn capture(document: &Document) -> Result<Self, HistoryError> {
        Ok(Self(document.to_json_pretty()?))
    }

    fn restore(&self) -> Result<Document, HistoryError> {
        Ok(Document::from_json(&self.0)?)
    }
}

#[derive(Debug, Error)]
pub enum HistoryError {
    #[error(transparent)]
    Document(#[from] DocumentError),
    #[error("nothing to undo")]
    NothingToUndo,
    #[error("nothing to redo")]
    NothingToRedo,
}

#[derive(Debug, Clone)]
pub struct DocumentHistory {
    past: Vec<Snapshot>,
    present: Snapshot,
    future: Vec<Snapshot>,
}

impl DocumentHistory {
    pub fn new(document: &Document) -> Result<Self, HistoryError> {
        Ok(Self {
            past: Vec::new(),
            present: Snapshot::capture(document)?,
            future: Vec::new(),
        })
    }

    pub fn commit(&mut self, document: &Document) -> Result<(), HistoryError> {
        let next = Snapshot::capture(document)?;
        let previous = std::mem::replace(&mut self.present, next);
        self.past.push(previous);
        self.future.clear();
        Ok(())
    }

    pub fn undo(&mut self) -> Result<Document, HistoryError> {
        let previous = self.past.pop().ok_or(HistoryError::NothingToUndo)?;
        let current = std::mem::replace(&mut self.present, previous);
        self.future.push(current);
        self.present.restore()
    }

    pub fn redo(&mut self) -> Result<Document, HistoryError> {
        let next = self.future.pop().ok_or(HistoryError::NothingToRedo)?;
        let current = std::mem::replace(&mut self.present, next);
        self.past.push(current);
        self.present.restore()
    }

    pub fn undo_depth(&self) -> usize {
        self.past.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.future.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmp_core::{Layer, PointSample, Stroke};

    #[test]
    fn undo_and_redo_restore_exact_document_state() {
        let mut document = Document::new("history");
        let mut history = DocumentHistory::new(&document).unwrap();

        let layer_id = document.add_layer(Layer::new("ink"));
        document
            .add_stroke(Stroke::new(
                layer_id,
                "pencil",
                vec![PointSample {
                    x: 1.0,
                    y: 2.0,
                    pressure: 0.8,
                    tilt_x: 0.0,
                    tilt_y: 0.0,
                    timestamp_ms: 10,
                }],
            ))
            .unwrap();
        let authored = document.clone();
        history.commit(&document).unwrap();

        let undone = history.undo().unwrap();
        assert!(undone.layers.is_empty());
        assert!(undone.strokes.is_empty());

        let redone = history.redo().unwrap();
        assert_eq!(redone, authored);
    }
}
