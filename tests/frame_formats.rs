//! The frame formats the widget asks alhazen-core for: zero-copy pixel buffers on macOS (drawn
//! with `paint_surface`), BGRA elsewhere.

use alhazen_gpui::alhazen_core::FrameFormat;

#[test]
fn the_widget_asks_for_what_gpui_can_draw() {
    let expected = if cfg!(target_os = "macos") { vec![FrameFormat::Native, FrameFormat::Bgra] } else { vec![FrameFormat::Bgra] };
    assert_eq!(alhazen_gpui::frame_formats(), expected);
}
