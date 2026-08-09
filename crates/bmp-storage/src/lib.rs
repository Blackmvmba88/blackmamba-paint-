use bmp_core::{Document, DocumentError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const BMPAINT_MAGIC: &str = "BLACKMAMBA_PAINT";
pub const BMPAINT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BmpaintPackage {
    pub magic: String,
    pub version: u32,
    pub document: Document,
}

impl BmpaintPackage {
    pub fn from_document(document: Document) -> Self {
        Self {
            magic: BMPAINT_MAGIC.to_string(),
            version: BMPAINT_VERSION,
            document,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, StorageError> {
        Ok(serde_json::to_vec_pretty(self)?)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, StorageError> {
        let package: Self = serde_json::from_slice(bytes)?;
        if package.magic != BMPAINT_MAGIC {
            return Err(StorageError::InvalidMagic(package.magic));
        }
        if package.version > BMPAINT_VERSION {
            return Err(StorageError::UnsupportedVersion(package.version));
        }
        Ok(package)
    }
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("invalid .bmpaint magic: {0}")]
    InvalidMagic(String),
    #[error("unsupported .bmpaint version: {0}")]
    UnsupportedVersion(u32),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Document(#[from] DocumentError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmp_core::{Layer, PointSample, Stroke};

    #[test]
    fn bmpaint_round_trip_preserves_world_coordinates_and_time() {
        let mut document = Document::new("Package Test");
        document.camera.x = 123_456_789.25;
        document.camera.y = -987_654_321.5;
        let layer_id = document.add_layer(Layer::new("ink"));
        document
            .add_stroke(Stroke::new(
                layer_id,
                "graphite",
                vec![
                    PointSample {
                        x: 123_456_790.0,
                        y: -987_654_320.0,
                        pressure: 0.2,
                        tilt_x: 0.0,
                        tilt_y: 0.0,
                        timestamp_ms: 100,
                    },
                    PointSample {
                        x: 123_456_800.0,
                        y: -987_654_310.0,
                        pressure: 0.9,
                        tilt_x: 0.0,
                        tilt_y: 0.0,
                        timestamp_ms: 420,
                    },
                ],
            ))
            .unwrap();

        let package = BmpaintPackage::from_document(document.clone());
        let bytes = package.encode().unwrap();
        let restored = BmpaintPackage::decode(&bytes).unwrap();
        assert_eq!(restored.document, document);
    }
}
