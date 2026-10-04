//! Smoke tests for the GPUI entity (no window, no rendering).
#![cfg(feature = "native")]

use gpui::{AppContext, TestAppContext};
use gpui_video::{PlayerConfig, PlayerState, Source, VideoPlayer};

fn fixture(name: &str) -> Source {
    Source::parse(&format!("{}/../video-core/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

#[gpui::test]
fn opens_fixture_in_background(cx: &mut TestAppContext) {
    let video = cx.new(|cx| VideoPlayer::new(fixture("av1.webm"), PlayerConfig::default(), cx));
    video.read_with(cx, |v, _| assert_eq!(v.state(), PlayerState::Loading));
    cx.run_until_parked();
    video.read_with(cx, |v, _| {
        assert_eq!(v.state(), PlayerState::Paused);
        assert_eq!(v.video_size(), Some((320, 240)));
        assert_eq!(v.duration(), Some(std::time::Duration::from_secs(2)));
    });
}

#[gpui::test]
fn missing_file_becomes_error_state(cx: &mut TestAppContext) {
    let video = cx.new(|cx| VideoPlayer::new(fixture("missing.webm"), PlayerConfig::default(), cx));
    cx.run_until_parked();
    video.read_with(cx, |v, _| assert!(v.state().is_error()));
}
