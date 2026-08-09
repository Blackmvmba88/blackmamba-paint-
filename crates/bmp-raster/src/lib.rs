use bmp_core::RasterReference;
use bmp_storage::{AssetBlob, BmpaintPackage};
use bmp_trace::{RasterImage, Rgba, TraceError};
use image::ImageReader;
use std::io::Cursor;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum RasterDecodeError {
    #[error("asset {0} does not exist in the .bmpaint package")]
    MissingAsset(Uuid),
    #[error(
        "reference dimensions {reference_width}x{reference_height} do not match decoded asset {decoded_width}x{decoded_height}"
    )]
    DimensionMismatch {
        reference_width: u32,
        reference_height: u32,
        decoded_width: u32,
        decoded_height: u32,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Image(#[from] image::ImageError),
    #[error(transparent)]
    Trace(#[from] TraceError),
}

pub fn decode_asset(asset: &AssetBlob) -> Result<RasterImage, RasterDecodeError> {
    let decoded = ImageReader::new(Cursor::new(asset.bytes.as_slice()))
        .with_guessed_format()?
        .decode()?
        .to_rgba8();
    let (width, height) = decoded.dimensions();
    let pixels = decoded
        .as_raw()
        .chunks_exact(4)
        .map(|pixel| Rgba {
            r: pixel[0],
            g: pixel[1],
            b: pixel[2],
            a: pixel[3],
        })
        .collect();
    Ok(RasterImage::new(width, height, pixels)?)
}

pub fn decode_reference(
    package: &BmpaintPackage,
    reference: &RasterReference,
) -> Result<RasterImage, RasterDecodeError> {
    let asset = package
        .asset(reference.asset_id)
        .ok_or(RasterDecodeError::MissingAsset(reference.asset_id))?;
    let image = decode_asset(asset)?;
    if image.width != reference.pixel_width || image.height != reference.pixel_height {
        return Err(RasterDecodeError::DimensionMismatch {
            reference_width: reference.pixel_width,
            reference_height: reference.pixel_height,
            decoded_width: image.width,
            decoded_height: image.height,
        });
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmp_core::Document;
    use image::{DynamicImage, ImageFormat, RgbaImage};

    fn two_pixel_png() -> Vec<u8> {
        let rgba = RgbaImage::from_raw(2, 1, vec![255, 0, 0, 255, 0, 128, 255, 64]).unwrap();
        let image = DynamicImage::ImageRgba8(rgba);
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, ImageFormat::Png).unwrap();
        bytes.into_inner()
    }

    #[test]
    fn package_asset_decodes_to_tracepulse_rgba_exactly() {
        let asset = AssetBlob::new("pixels.png", "image/png", two_pixel_png());
        let decoded = decode_asset(&asset).unwrap();
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 1);
        assert_eq!(
            decoded.pixel(bmp_trace::PixelCoord::new(0, 0)).unwrap(),
            Rgba {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            }
        );
        assert_eq!(
            decoded.pixel(bmp_trace::PixelCoord::new(1, 0)).unwrap(),
            Rgba {
                r: 0,
                g: 128,
                b: 255,
                a: 64,
            }
        );
    }

    #[test]
    fn reference_resolves_its_asset_and_validates_dimensions() {
        let asset = AssetBlob::new("pixels.png", "image/png", two_pixel_png());
        let asset_id = asset.id;
        let mut package = BmpaintPackage::from_document(Document::new("decode"));
        package.add_asset(asset);
        let reference = RasterReference::new(asset_id, "pixels.png", "image/png", 2, 1);
        let decoded = decode_reference(&package, &reference).unwrap();
        assert_eq!((decoded.width, decoded.height), (2, 1));
    }

    #[test]
    fn stale_reference_dimensions_are_rejected() {
        let asset = AssetBlob::new("pixels.png", "image/png", two_pixel_png());
        let asset_id = asset.id;
        let mut package = BmpaintPackage::from_document(Document::new("decode"));
        package.add_asset(asset);
        let reference = RasterReference::new(asset_id, "pixels.png", "image/png", 3, 1);
        assert!(matches!(
            decode_reference(&package, &reference),
            Err(RasterDecodeError::DimensionMismatch {
                reference_width: 3,
                reference_height: 1,
                decoded_width: 2,
                decoded_height: 1,
            })
        ));
    }

    #[test]
    fn missing_asset_is_explicit() {
        let package = BmpaintPackage::from_document(Document::new("decode"));
        let missing_id = Uuid::new_v4();
        let reference = RasterReference::new(missing_id, "missing.png", "image/png", 1, 1);
        assert!(matches!(
            decode_reference(&package, &reference),
            Err(RasterDecodeError::MissingAsset(id)) if id == missing_id
        ));
    }
}
