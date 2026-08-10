use bmp_core::{Document, RasterReference};
use bmp_raster::{decode_asset, RasterDecodeError};
use bmp_storage::AssetBlob;
use std::collections::BTreeMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImportPlacement {
    pub center_x: f64,
    pub center_y: f64,
    pub max_world_width: f64,
    pub max_world_height: f64,
}

impl ImportPlacement {
    pub fn centered(
        center_x: f64,
        center_y: f64,
        max_world_width: f64,
        max_world_height: f64,
    ) -> Self {
        Self {
            center_x,
            center_y,
            max_world_width,
            max_world_height,
        }
    }
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("import placement must be finite and positive")]
    InvalidPlacement,
    #[error(transparent)]
    Decode(#[from] RasterDecodeError),
}

pub fn import_raster_reference(
    document: &mut Document,
    assets: &mut BTreeMap<Uuid, AssetBlob>,
    name: impl Into<String>,
    media_type: impl Into<String>,
    bytes: Vec<u8>,
    placement: ImportPlacement,
) -> Result<Uuid, ImportError> {
    validate_placement(placement)?;

    let name = name.into();
    let media_type = media_type.into();
    let asset = AssetBlob::new(name.clone(), media_type.clone(), bytes);
    let decoded = decode_asset(&asset)?;

    let source_width = f64::from(decoded.width);
    let source_height = f64::from(decoded.height);
    let scale = (placement.max_world_width / source_width)
        .min(placement.max_world_height / source_height)
        .min(1.0);

    let mut reference =
        RasterReference::new(asset.id, name, media_type, decoded.width, decoded.height);
    reference.center_x = placement.center_x;
    reference.center_y = placement.center_y;
    reference.world_width = source_width * scale;
    reference.world_height = source_height * scale;
    reference.z_index = document
        .raster_references
        .values()
        .map(|existing| existing.z_index)
        .max()
        .unwrap_or(-1)
        .saturating_add(1);

    let reference_id = reference.id;
    assets.insert(asset.id, asset);
    document.add_raster_reference(reference);
    Ok(reference_id)
}

fn validate_placement(placement: ImportPlacement) -> Result<(), ImportError> {
    if !placement.center_x.is_finite()
        || !placement.center_y.is_finite()
        || !placement.max_world_width.is_finite()
        || !placement.max_world_height.is_finite()
        || placement.max_world_width <= 0.0
        || placement.max_world_height <= 0.0
    {
        return Err(ImportError::InvalidPlacement);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageFormat, RgbaImage};
    use std::io::Cursor;

    fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        let rgba = RgbaImage::from_pixel(width, height, image::Rgba([10, 20, 30, 255]));
        let image = DynamicImage::ImageRgba8(rgba);
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, ImageFormat::Png).unwrap();
        bytes.into_inner()
    }

    #[test]
    fn import_creates_linked_asset_and_reference_atomically() {
        let mut document = Document::new("import");
        let mut assets = BTreeMap::new();
        let reference_id = import_raster_reference(
            &mut document,
            &mut assets,
            "wide.png",
            "image/png",
            png_bytes(400, 200),
            ImportPlacement::centered(50.0, -25.0, 300.0, 300.0),
        )
        .unwrap();

        let reference = &document.raster_references[&reference_id];
        assert_eq!(reference.center_x, 50.0);
        assert_eq!(reference.center_y, -25.0);
        assert_eq!(reference.world_width, 300.0);
        assert_eq!(reference.world_height, 150.0);
        assert_eq!(reference.z_index, 0);
        assert_eq!(assets[&reference.asset_id].name, "wide.png");
    }

    #[test]
    fn newer_imports_are_stacked_above_existing_references() {
        let mut document = Document::new("stack");
        let mut assets = BTreeMap::new();
        let first_id = import_raster_reference(
            &mut document,
            &mut assets,
            "first.png",
            "image/png",
            png_bytes(2, 2),
            ImportPlacement::centered(0.0, 0.0, 100.0, 100.0),
        )
        .unwrap();
        let second_id = import_raster_reference(
            &mut document,
            &mut assets,
            "second.png",
            "image/png",
            png_bytes(2, 2),
            ImportPlacement::centered(0.0, 0.0, 100.0, 100.0),
        )
        .unwrap();

        assert!(
            document.raster_references[&second_id].z_index
                > document.raster_references[&first_id].z_index
        );
    }

    #[test]
    fn import_preserves_small_images_at_native_world_size() {
        let mut document = Document::new("import");
        let mut assets = BTreeMap::new();
        let reference_id = import_raster_reference(
            &mut document,
            &mut assets,
            "small.png",
            "image/png",
            png_bytes(20, 10),
            ImportPlacement::centered(0.0, 0.0, 500.0, 500.0),
        )
        .unwrap();
        let reference = &document.raster_references[&reference_id];
        assert_eq!(reference.world_width, 20.0);
        assert_eq!(reference.world_height, 10.0);
    }

    #[test]
    fn corrupt_image_does_not_mutate_document_or_assets() {
        let mut document = Document::new("import");
        let before = document.clone();
        let mut assets = BTreeMap::new();
        let result = import_raster_reference(
            &mut document,
            &mut assets,
            "broken.png",
            "image/png",
            vec![1, 2, 3, 4],
            ImportPlacement::centered(0.0, 0.0, 100.0, 100.0),
        );
        assert!(result.is_err());
        assert_eq!(document, before);
        assert!(assets.is_empty());
    }

    #[test]
    fn invalid_placement_does_not_decode_or_mutate() {
        let mut document = Document::new("import");
        let before = document.clone();
        let mut assets = BTreeMap::new();
        let result = import_raster_reference(
            &mut document,
            &mut assets,
            "image.png",
            "image/png",
            png_bytes(2, 2),
            ImportPlacement::centered(0.0, 0.0, 0.0, 100.0),
        );
        assert!(matches!(result, Err(ImportError::InvalidPlacement)));
        assert_eq!(document, before);
        assert!(assets.is_empty());
    }
}
