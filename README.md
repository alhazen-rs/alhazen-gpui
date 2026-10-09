# alhazen-gpui

[![crates.io](https://img.shields.io/crates/v/alhazen-gpui.svg)](https://crates.io/crates/alhazen-gpui)
[![docs.rs](https://img.shields.io/docsrs/alhazen-gpui)](https://docs.rs/alhazen-gpui)
[![CI](https://github.com/alhazen-rs/alhazen-gpui/actions/workflows/ci.yml/badge.svg)](https://github.com/alhazen-rs/alhazen-gpui/actions/workflows/ci.yml)

**Native video playback for [GPUI](https://www.gpui.rs/) apps.** A `VideoPlayer` entity and a
`video_view` element, powered by the [alhazen-core](https://github.com/alhazen-rs/alhazen-core)
media engine.

```rust
let video = cx.new(|cx| VideoPlayer::new(Source::parse("movie.mkv")?, PlayerConfig::default(), cx));
// in render():
video_view(video.clone()).size_full()
```

- **Plays common formats with nothing installed:** MP4, MOV, MKV and WebM, with AV1, VP9, VP8,
  ProRes, Opus, Vorbis, FLAC and PCM in pure Rust on every platform.
- **On Windows:** H.264, HEVC, VP9 and AV1 decode on the GPU, plus AAC, MP3, AC-3 and ALAC,
  through Windows' own decoders.
- **On Linux with an NVIDIA GPU:** H.264, HEVC, VP8, VP9 and AV1 decode on the GPU (NVDEC), scaled
  to the size you display them at. Loaded from the driver at runtime: nothing to install.
- **Everything else** goes through the user's `ffmpeg`, if installed. It's found at runtime,
  never linked.
- **Local files and HTTP(S) streaming,** with seeking.
- **Smooth playback:** audio-driven sync, late-frame dropping, decoding at the size you display,
  and automatic fallback to a faster decoder.

## Quick start

```toml
[dependencies]
gpui = "0.2.2"
alhazen-gpui = "0.4"

# Decoding unoptimized is many times slower than real time: optimize the decoders even in
# debug builds.
[profile.dev.package.rav1d]
opt-level = 3
[profile.dev.package.vp9-mt]
opt-level = 3
```

```rust
use gpui::{div, prelude::*, Context, Entity, Window};
use alhazen_gpui::{video_view, PlayerConfig, PlayerEvent, Source, VideoPlayer};

struct Screen {
    video: Entity<VideoPlayer>,
}

impl Screen {
    fn new(cx: &mut Context<Self>) -> Self {
        let source = Source::parse("https://example.com/movie.webm").unwrap();
        let config = PlayerConfig { autoplay: true, ..Default::default() };
        let video = cx.new(|cx| VideoPlayer::new(source, config, cx));

        // Repaint when the player changes (position, state, volume, ...).
        cx.observe(&video, |_, _, cx| cx.notify()).detach();
        // Non-fatal problems, e.g. "no decoder for this audio track; playing video only".
        cx.subscribe(&video, |_, _, event: &PlayerEvent, _| {
            if let PlayerEvent::Warning(w) = event {
                eprintln!("video warning: {w}");
            }
        })
        .detach();
        Self { video }
    }
}

impl Render for Screen {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(video_view(self.video.clone()).size_full())
            .child(
                div().id("toggle").child("Play/Pause").on_click(cx.listener(|this, _, _, cx| {
                    this.video.update(cx, |v, cx| v.toggle(cx));
                })),
            )
    }
}
```

## The example player

[`examples/play.rs`](examples/play.rs) is a complete player window: progress bar, seeking,
volume and a status line, in plain GPUI divs. It's meant to be copied:

```bash
cargo run --release --example play -- path/to/video.mkv
cargo run --release --example play -- https://example.com/clip.webm
```

## API

### `VideoPlayer` (a GPUI entity)

Create it with `cx.new(|cx| VideoPlayer::new(source, config, cx))`. Opening happens in the
background; `state()` is `Loading` until then.

| Control (takes `cx`) | Query |
|---|---|
| `play`, `pause`, `toggle` | `state`, `is_playing` |
| `seek(Duration)` | `position`, `duration`, `is_seekable` |
| `set_volume(0.0..=1.0)`, `set_muted(bool)` | `volume`, `is_muted` |
| | `has_video`, `has_audio`, `video_size` |
| | `player()`: the engine's `Player` (stats, events) once opened |

Events (`cx.subscribe`) are the engine's `PlayerEvent`s: `StateChanged`, `FrameReady`,
`Warning`, `Error`, `Ended`.

### `video_view(player)` (an element)

Styled like a `div` (`.size_full()`, `.w(px(640.))`, `.rounded_md()`, ...). It letterboxes by
default; change that with `.object_fit(gpui::ObjectFit::Cover)` (or `Fill`, `ScaleDown`,
`None`). It tells the engine the size it's drawn at, so large videos are decoded down to that
size before colour conversion.

### Configuration

`PlayerConfig` and `Source` come from alhazen-core and are re-exported here. The engine itself
is available as `alhazen_gpui::alhazen_core`. See
[alhazen-core's README](https://github.com/alhazen-rs/alhazen-core#configuration) for every
setting: audio output, GPU preference, backend order, ffmpeg path, buffering.

## Cargo features

The same as alhazen-core's, forwarded to it:

| Feature | Default | Adds |
|---|---|---|
| `native` | ✅ | Pure-Rust decoders and the MP4/MOV demuxer. |
| `http` | ✅ | HTTP(S) sources. |
| `audio-output` | ✅ | Sound through the default output device. |
| `ffmpeg-cli` | ✅ | The user's ffmpeg, found at runtime. |
| `media-foundation` | ✅ | Windows' (GPU) decoders; nothing on other platforms. |
| `nvdec` | ✅ | NVIDIA GPU decoding on Linux, loaded from the driver at runtime; nothing on other platforms. |
| `native-aac` | ✅ | AAC-LC, HE-AAC and HE-AACv2 in pure Rust (rusty_aac). |

## Debugging playback

Set `ALHAZEN_DEBUG=1` to print one line per second to stderr: position, frames shown per
second, paint cost, how far the video is behind the clock, dropped frames, state and which
decoder is in use. For example:

```
[alhazen] pos   12.04s | shown  60 fps | paint avg   1.2 ms, worst   2.0 ms | frame behind clock    3.0 ms | dropped   0/s | state Playing | video media-foundation
```

## Platform requirements

- **Rust** 1.92+.
- **x86-64:** [`nasm`](https://www.nasm.us/) on `PATH` (rav1d's AV1 assembly).
- **Linux:** GPUI's requirements (Vulkan, `libxkbcommon`, Wayland/X11 headers) and ALSA headers
  for sound (`libasound2-dev`).
- **Windows 10/11, macOS:** nothing else.
- **GPU decoding on Linux:** an NVIDIA GPU with its proprietary driver (optional; without it,
  playback uses the other decoders).

## License

Apache License, Version 2.0 ([LICENSE](LICENSE)). If you distribute an app that contains
alhazen-gpui, include the [NOTICE](NOTICE) text (Apache-2.0 §4(d)), for example in an "About"
or "Licenses" screen.

Part of [Alhazen](https://github.com/alhazen-rs).
