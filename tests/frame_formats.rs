//! The frame formats the widget asks alhazen-core for. BGRA everywhere: GPUI 0.2.2's
//! `paint_surface` only takes 8-bit full-range 4:2:0 pixel buffers (it asserts the format) and
//! draws them with a fixed BT.601 matrix, so VideoToolbox's own buffers can't go to it as they
//! are. On macOS the BGRA is converted on the GPU by VideoToolbox's pixel transfer instead.

use alhazen_gpui::alhazen_core::FrameFormat;

#[test]
fn the_widget_asks_for_bgra() {
    assert_eq!(alhazen_gpui::frame_formats(), vec![FrameFormat::Bgra]);
}
