//! GPUI integration for `alhazen-core`: a `VideoPlayer` entity and a `video_view` element.

mod cover;
mod element;
mod player;

pub use element::{VideoView, video_view};
pub use player::VideoPlayer;
pub use alhazen_core::{self, Metadata, Picture, PlayerConfig, PlayerEvent, PlayerState, Source};
