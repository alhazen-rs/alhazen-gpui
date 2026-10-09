//! Cover art: an embedded picture (JPEG/PNG) decoded for painting.

use std::sync::Arc;

use gpui::RenderImage;

/// Decodes `data` into a gpui image (BGRA, like video frames); `None` when it is not an image.
pub(crate) fn decode_cover(data: &[u8]) -> Option<Arc<RenderImage>> {
    let mut image = image::load_from_memory(data).ok()?.into_rgba8();
    for px in image.pixels_mut() {
        px.0.swap(0, 2);
    }
    Some(Arc::new(RenderImage::new([image::Frame::new(image)])))
}

#[cfg(test)]
mod tests {
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
