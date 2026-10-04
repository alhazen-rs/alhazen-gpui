//! GPUI integration for `video-core`: a `VideoPlayer` entity and a `video_view` element.

mod element;
mod player;

pub use element::{VideoView, video_view};
pub use player::VideoPlayer;
pub use video_core::{self, PlayerConfig, PlayerEvent, PlayerState, Source};
