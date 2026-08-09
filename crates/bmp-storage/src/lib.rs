use bmp_core::{Document, DocumentError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;
use uuid::Uuid;

pub const BMPAINT_MAGIC: &str = "BLACKMAMBA_PAINT";
pub const BMPAINT_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetBlob {
    pub id: Uuid,
    pub name: String,
    pub media_type: String,
    pub bytes: Vec<u8>,
}

impl AssetBlob {
    pub fn new(name: impl Into<String>, media_type: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            media_type: media_type.into(),
            bytes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BmpaintPackage {
    pub magic: String,
    pub version: u32,
    pub document: Document,
    #[serde(default)]
    pub assets: BTreeMap<Uuid, AssetBlob>,
}

impl BmpaintPackage {
    pub fn from_document(document: Document) -> Self {
        Self {
            magic: BMPAINT_MAGIC.to_string(),
            version: BMPAINT_VERSION,
            document,
            assets: BTreeMap::new(),
        }
    }

    pub fn add_asset(&mut self, asset: AssetBlob) -> Uuid {
        let id = asset.id;
        self.assets.insert(id, asset);
        id
    }

    pub fn asset(&self, id: Uuid) -> Option<&AssetBlob> {
        self.assets.get(&id)
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
    use bmp_core::{Layer, PointSample, RasterReference, Stroke};

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
        assert!(restored.assets.is_empty());
    }

    #[test]
    fn asset_bytes_round_trip_with_reference_identity() {
        let mut document = Document::new("Reference Package");
        let asset = AssetBlob::new(
            "reference.png",
            "image/png",
            vec![137, 80, 78, 71, 13, 10, 26, 10],
        );
        let asset_id = asset.id;
        document.add_raster_reference(RasterReference::new(
            asset_id,
            "reference.png",
            "image/png",
            640,
            480,
        ));

        let mut package = BmpaintPackage::from_document(document);
        package.add_asset(asset);
        let encoded = package.encode().unwrap();
        let restored = BmpaintPackage::decode(&encoded).unwrap();

        assert_eq!(restored.version, BMPAINT_VERSION);
        assert_eq!(
            restored.asset(asset_id).unwrap().bytes,
            vec![137, 80, 78, 71, 13, 10, 26, 10]
        );
        assert!(restored
            .document
            .raster_references
            .values()
            .any(|reference| reference.asset_id == asset_id));
    }

    #[test]
    fn version_one_package_without_assets_remains_readable() {
        let document = Document::new("Legacy Package");
        let legacy = serde_json::json!({
            "magic": BMPAINT_MAGIC,
            "version": 1,
            "document": document,
        });
        let restored = BmpaintPackage::decode(&serde_json::to_vec(&legacy).unwrap()).unwrap();
        assert_eq!(restored.version, 1);
        assert!(restored.assets.is_empty());
    }

    #[test]
    fn future_package_version_is_rejected() {
        let mut package = BmpaintPackage::from_document(Document::new("future"));
        package.version = BMPAINT_VERSION + 1;
        let encoded = serde_json::to_vec(&package).unwrap();
        assert!(matches!(
            BmpaintPackage::decode(&encoded),
            Err(StorageError::UnsupportedVersion(version)) if version == BMPAINT_VERSION + 1
        ));
    }
}
