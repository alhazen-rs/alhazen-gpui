//! GPUI integration for `alhazen-core`: a `VideoPlayer` entity and a `video_view` element.

mod cover;
mod element;
mod player;

pub use element::{VideoView, video_view};
pub use player::VideoPlayer;
pub use alhazen_core::{self, Metadata, Picture, PlayerConfig, PlayerEvent, PlayerState, Source};

/// The frame formats the widget asks alhazen-core for (it sets `PlayerConfig::frame_formats` to
/// these): BGRA, converted on the GPU where the platform decodes on it. (GPUI 0.2.2's
/// `paint_surface` only takes 8-bit full-range pixel buffers and draws them with a fixed BT.601
/// matrix, so VideoToolbox's own buffers are not handed to it.)
pub fn frame_formats() -> Vec<alhazen_core::FrameFormat> {
    vec![alhazen_core::FrameFormat::Bgra]
}
