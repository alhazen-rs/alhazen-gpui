//! GPUI integration for `alhazen-core`: a `VideoPlayer` entity and a `video_view` element.

mod cover;
mod element;
mod player;

pub use element::{VideoView, video_view};
pub use player::VideoPlayer;
pub use alhazen_core::{self, Metadata, Picture, PlayerConfig, PlayerEvent, PlayerState, Source};

/// The frame formats the widget asks alhazen-core for (it sets `PlayerConfig::frame_formats` to
/// these): VideoToolbox's own pixel buffers on macOS, drawn without a copy, then BGRA.
pub fn frame_formats() -> Vec<alhazen_core::FrameFormat> {
    use alhazen_core::FrameFormat;
    if cfg!(target_os = "macos") { vec![FrameFormat::Native, FrameFormat::Bgra] } else { vec![FrameFormat::Bgra] }
}
