//! gpui-video's default build carries video-core's default backends (the workspace dependency
//! turns video-core's defaults off, so each one must be forwarded explicitly).

use gpui_video::video_core::backend::Registry;

#[test]
fn default_build_includes_ffmpeg_cli() {
    let names = Registry::with_ffmpeg(&Default::default()).names();
    assert!(names.contains(&"ffmpeg-cli"), "backends: {names:?}");
}

#[cfg(windows)]
#[test]
fn default_build_includes_media_foundation() {
    let names = Registry::with_ffmpeg(&Default::default()).names();
    assert!(names.contains(&"media-foundation"), "backends: {names:?}");
}
