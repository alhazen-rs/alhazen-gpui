//! Smoke tests for the GPUI entity (no window, no rendering).
#![cfg(feature = "native")]

use gpui::{AppContext, TestAppContext};
use gpui_video::video_core::audio::{AudioOutputConfig, NullOutput};
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
        assert!(v.is_seekable());
        assert_eq!(v.duration(), Some(std::time::Duration::from_secs(2)));
    });
}

#[gpui::test]
fn volume_set_while_loading_applies_once_open(cx: &mut TestAppContext) {
    let config = PlayerConfig { audio_output: AudioOutputConfig::Null(NullOutput::new(48_000, 2)), ..Default::default() };
    let video = cx.new(|cx| VideoPlayer::new(fixture("av1_with_audio.webm"), config, cx));
    video.update(cx, |v, cx| {
        v.set_volume(0.25, cx);
        v.set_muted(true, cx);
    });
    cx.run_until_parked();
    video.read_with(cx, |v, _| {
        assert!(v.has_video() && v.has_audio());
        assert_eq!(v.volume(), 0.25);
        assert!(v.is_muted());
    });
}

#[gpui::test]
fn missing_file_becomes_error_state(cx: &mut TestAppContext) {
    let video = cx.new(|cx| VideoPlayer::new(fixture("missing.webm"), PlayerConfig::default(), cx));
    cx.run_until_parked();
    video.read_with(cx, |v, _| assert!(v.state().is_error()));
}

#[test]
fn default_build_can_play_sound() {
    assert!(
        gpui_video::video_core::audio::output_available(),
        "gpui-video must enable video-core's audio-output feature by default"
    );
}
