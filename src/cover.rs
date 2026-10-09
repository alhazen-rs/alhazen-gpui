//! Cover art: an embedded picture (JPEG/PNG) decoded for painting.

use std::sync::Arc;

use gpui::RenderImage;

/// Covers larger than this (on their longer side) are scaled down to it.
const MAX_SIDE: u32 = 1024;

/// Decodes `data` into a gpui image (BGRA, like video frames); `None` when it is not an image.
pub(crate) fn decode_cover(data: &[u8]) -> Option<Arc<RenderImage>> {
    let mut image = image::load_from_memory(data).ok()?;
    // Covers are shown small: a huge one would otherwise be a huge texture.
    if image.width().max(image.height()) > MAX_SIDE {
        image = image.resize(MAX_SIDE, MAX_SIDE, image::imageops::FilterType::Triangle);
    }
    let mut image = image.into_rgba8();
    for px in image.pixels_mut() {
        px.0.swap(0, 2);
    }
    Some(Arc::new(RenderImage::new([image::Frame::new(image)])))
}

#[cfg(test)]
mod tests {
    #[test]
    fn huge_covers_are_scaled_down_keeping_their_shape() {
        let big = image::RgbaImage::from_pixel(3000, 2000, image::Rgba([0, 0, 255, 255]));
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(big).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        let size = super::decode_cover(&png).unwrap().size(0);
        assert_eq!((size.width.0, size.height.0), (1024, 683));
    }

    #[test]
    fn decodes_a_png_cover_to_bgra() {
        let image = super::decode_cover(include_bytes!("../tests/fixtures/cover.png")).unwrap();
        let size = image.size(0);
        assert_eq!((size.width.0, size.height.0), (16, 16));
        // ffmpeg's `color=red` is 253 after its YUV round trip: check "red, stored as BGRA".
        let [b, g, r, a] = image.as_bytes(0).unwrap()[..4] else { unreachable!() };
        assert!(b < 8 && g < 8 && r > 245 && a == 255, "BGRA {:?}", [b, g, r, a]);
        assert!(super::decode_cover(b"not an image").is_none());
    }
}
