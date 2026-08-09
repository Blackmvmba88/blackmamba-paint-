use bmp_core::{Document, DocumentError, PathObject, PathOrigin};
use bmp_raster::{decode_asset, RasterDecodeError};
use bmp_reference::{editable_path_to_world, ReferenceError};
use bmp_storage::AssetBlob;
use bmp_trace::{trace_line, PixelCoord, TraceAmbiguity, TraceConfig, TraceError};
use std::collections::BTreeMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct TracePulseRequest {
    pub reference_id: Uuid,
    pub layer_id: Uuid,
    pub seed: PixelCoord,
    pub config: TraceConfig,
    pub brush_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TracePulseExtraction {
    pub path_id: Uuid,
    pub reference_id: Uuid,
    pub seed: PixelCoord,
    pub seed_node_index: usize,
    pub confidence: f32,
    pub ambiguities: Vec<TraceAmbiguity>,
}

#[derive(Debug, Error)]
pub enum TracePulseServiceError {
    #[error("raster reference {0} does not exist")]
    MissingReference(Uuid),
    #[error("asset {0} does not exist")]
    MissingAsset(Uuid),
    #[error(transparent)]
    Decode(#[from] RasterDecodeError),
    #[error(transparent)]
    Trace(#[from] TraceError),
    #[error(transparent)]
    Reference(#[from] ReferenceError),
    #[error(transparent)]
    Document(#[from] DocumentError),
}

pub fn extract_tracepulse_path(
    document: &mut Document,
    assets: &BTreeMap<Uuid, AssetBlob>,
    request: TracePulseRequest,
) -> Result<TracePulseExtraction, TracePulseServiceError> {
    let reference = document
        .raster_references
        .get(&request.reference_id)
        .cloned()
        .ok_or(TracePulseServiceError::MissingReference(
            request.reference_id,
        ))?;
    let asset = assets
        .get(&reference.asset_id)
        .ok_or(TracePulseServiceError::MissingAsset(reference.asset_id))?;

    let image = decode_asset(asset)?;
    if image.width != reference.pixel_width || image.height != reference.pixel_height {
        return Err(RasterDecodeError::DimensionMismatch {
            reference_width: reference.pixel_width,
            reference_height: reference.pixel_height,
            decoded_width: image.width,
            decoded_height: image.height,
        }
        .into());
    }

    let traced = trace_line(&image, request.seed, request.config)?;
    let world_path = editable_path_to_world(&reference, &traced.path)?;
    let path_id = world_path.id;
    let mut object = PathObject::new(
        request.layer_id,
        world_path,
        PathOrigin::TracePulse {
            reference_id: reference.id,
            seed_pixel_x: traced.seed.x,
            seed_pixel_y: traced.seed.y,
            seed_node_index: traced.seed_node_index,
        },
    );
    object.brush_id = request.brush_id;

    document.add_path(object)?;

    Ok(TracePulseExtraction {
        path_id,
        reference_id: reference.id,
        seed: traced.seed,
        seed_node_index: traced.seed_node_index,
        confidence: traced.confidence,
        ambiguities: traced.ambiguities,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmp_core::{Layer, RasterReference};
    use bmp_reference::pixel_center_to_world;
    use image::{DynamicImage, ImageFormat, RgbaImage};
    use std::io::Cursor;

    fn line_png() -> Vec<u8> {
        let mut image = RgbaImage::from_pixel(5, 3, image::Rgba([255, 255, 255, 255]));
        for x in 0..5 {
            image.put_pixel(x, 1, image::Rgba([0, 0, 0, 255]));
        }
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(image)
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    fn project() -> (Document, BTreeMap<Uuid, AssetBlob>, Uuid, Uuid) {
        let mut document = Document::new("TracePulse service");
        let layer_id = document.add_layer(Layer::new("TracePulse"));
        let asset = AssetBlob::new("line.png", "image/png", line_png());
        let asset_id = asset.id;
        let mut reference = RasterReference::new(asset_id, "line.png", "image/png", 5, 3);
        reference.center_x = 100.0;
        reference.center_y = -50.0;
        reference.world_width = 50.0;
        reference.world_height = 30.0;
        let reference_id = document.add_raster_reference(reference);
        let assets = BTreeMap::from([(asset_id, asset)]);
        (document, assets, reference_id, layer_id)
    }

    #[test]
    fn exact_seed_becomes_persistent_world_path_with_provenance() {
        let (mut document, assets, reference_id, layer_id) = project();
        let seed = PixelCoord::new(2, 1);
        let extraction = extract_tracepulse_path(
            &mut document,
            &assets,
            TracePulseRequest {
                reference_id,
                layer_id,
                seed,
                config: TraceConfig::default(),
                brush_id: Some("graphite".into()),
            },
        )
        .unwrap();

        let path = &document.paths[&extraction.path_id];
        assert_eq!(path.geometry.nodes.len(), 5);
        assert_eq!(
            document.layers[&layer_id].path_ids,
            vec![extraction.path_id]
        );
        assert!(matches!(
            path.origin,
            PathOrigin::TracePulse {
                reference_id: id,
                seed_pixel_x: 2,
                seed_pixel_y: 1,
                seed_node_index,
            } if id == reference_id && seed_node_index == extraction.seed_node_index
        ));

        let reference = &document.raster_references[&reference_id];
        let expected_seed = pixel_center_to_world(reference, seed).unwrap();
        let actual_seed = path.geometry.nodes[extraction.seed_node_index].position;
        assert!((actual_seed.x - expected_seed.x).abs() < 1e-9);
        assert!((actual_seed.y - expected_seed.y).abs() < 1e-9);
    }

    #[test]
    fn missing_asset_does_not_mutate_document_paths() {
        let (mut document, _assets, reference_id, layer_id) = project();
        let before = document.clone();
        let result = extract_tracepulse_path(
            &mut document,
            &BTreeMap::new(),
            TracePulseRequest {
                reference_id,
                layer_id,
                seed: PixelCoord::new(2, 1),
                config: TraceConfig::default(),
                brush_id: None,
            },
        );
        assert!(matches!(
            result,
            Err(TracePulseServiceError::MissingAsset(_))
        ));
        assert_eq!(document, before);
    }

    #[test]
    fn invalid_target_layer_leaves_path_map_unchanged() {
        let (mut document, assets, reference_id, _layer_id) = project();
        let before_paths = document.paths.clone();
        let result = extract_tracepulse_path(
            &mut document,
            &assets,
            TracePulseRequest {
                reference_id,
                layer_id: Uuid::new_v4(),
                seed: PixelCoord::new(2, 1),
                config: TraceConfig::default(),
                brush_id: None,
            },
        );
        assert!(matches!(
            result,
            Err(TracePulseServiceError::Document(
                DocumentError::MissingLayer(_)
            ))
        ));
        assert_eq!(document.paths, before_paths);
    }
}
