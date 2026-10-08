//! alhazen-gpui's default build carries alhazen-core's default backends (the workspace dependency
//! turns alhazen-core's defaults off, so each one must be forwarded explicitly).

use alhazen_gpui::alhazen_core::backend::Registry;

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
