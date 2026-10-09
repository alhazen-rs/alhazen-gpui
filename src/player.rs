use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use gpui::{App, Context, EventEmitter, RenderImage, Task};
use alhazen_core::{Metadata, Player, PlayerConfig, PlayerEvent, PlayerState, Source, VideoFrame};

/// Paints a replaced frame image stays in the sprite atlas before it is dropped. GPUI's Blade
/// atlas destroys an image's GPU texture as soon as it is removed, while frames already submitted
/// may still be sampling it ("GPU has crashed" at 4K60). A few frames of slack lets them finish.
const RETIRE_AFTER: usize = 3;

/// Images that left the screen, oldest first; each is dropped once `RETIRE_AFTER` newer ones
/// have replaced it.
pub(crate) struct Retired<T>(VecDeque<T>);

impl<T> Default for Retired<T> {
    fn default() -> Self {
        Self(VecDeque::new())
    }
}

impl<T> Retired<T> {
    /// Retires `old`; returns the images now safe to drop.
    pub fn push(&mut self, old: T) -> Vec<T> {
        self.0.push_back(old);
        let excess = self.0.len().saturating_sub(RETIRE_AFTER);
        self.0.drain(..excess).collect()
    }

    pub fn drain_all(&mut self) -> Vec<T> {
        self.0.drain(..).collect()
    }
}

/// Per-second playback diagnostics, printed to stderr when `ALHAZEN_DEBUG` is set.
pub(crate) struct DebugStats {
    since: std::time::Instant,
    new_frames: u32,
    paint_cost: Duration,
    worst_paint: Duration,
    dropped_at_start: u64,
}

impl DebugStats {
    fn from_env() -> Option<Self> {
        std::env::var_os("ALHAZEN_DEBUG").map(|_| Self {
            since: std::time::Instant::now(),
            new_frames: 0,
            paint_cost: Duration::ZERO,
            worst_paint: Duration::ZERO,
            dropped_at_start: 0,
        })
    }
}

/// How often player events are forwarded to GPUI.
const EVENT_POLL: Duration = Duration::from_millis(16);

/// A video player entity. Create with `cx.new(|cx| VideoPlayer::new(source, config, cx))`
/// and render with [`crate::video_view`].
pub struct VideoPlayer {
    player: Option<Arc<Player>>,
    state: PlayerState,
    /// The image currently in GPUI's sprite atlas, and the frame data it was built from.
    image: Option<(Arc<RenderImage>, Arc<[u8]>)>,
    /// Pts of the frame in `image`.
    image_pts: Option<Duration>,
    /// Replaced images still in the atlas until the GPU is surely done with them.
    retired: Retired<Arc<RenderImage>>,
    debug: Option<DebugStats>,
    /// Display size (device pixels) last passed to the player.
    output_size: Option<(u32, u32)>,
    /// Volume settings, kept here so they apply even before the player has opened.
    volume: f32,
    muted: bool,
    /// Tags of the open media.
    metadata: Option<Metadata>,
    /// The cover art, decoded (BGRA, like video frames) once the media has opened.
    cover: Option<Arc<RenderImage>>,
    _tasks: Vec<Task<()>>,
}

impl EventEmitter<PlayerEvent> for VideoPlayer {}

impl VideoPlayer {
    /// Starts opening `source` on the background executor; state is `Loading` until done.
    pub fn new(source: Source, config: PlayerConfig, cx: &mut Context<Self>) -> Self {
        cx.on_release(|this: &mut Self, cx: &mut App| {
            if let Some((image, _)) = this.image.take() {
                cx.drop_image(image, None);
            }
            if let Some(cover) = this.cover.take() {
                cx.drop_image(cover, None);
            }
            for image in this.retired.drain_all() {
                cx.drop_image(image, None);
            }
        })
        .detach();

        let open = cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { Player::open(source, config) })
                .await;
            let events = this
                .update(cx, |this, cx| {
                    let events = match result {
                        Ok(player) => {
                            let events = player.events();
                            player.set_volume(this.volume);
                            player.set_muted(this.muted);
                            this.state = player.state();
                            let metadata = player.metadata();
                            if let Some(picture) = metadata.as_ref().and_then(|m| m.cover.clone()) {
                                // Decode off the UI thread; paint once ready.
                                let task = cx.spawn(async move |this, cx| {
                                    let cover = cx.background_executor().spawn(async move { crate::cover::decode_cover(&picture.data) }).await;
                                    let _ = this.update(cx, |this, cx| {
                                        this.cover = cover;
                                        cx.notify();
                                    });
                                });
                                this._tasks.push(task);
                            }
                            this.metadata = metadata;
                            this.player = Some(Arc::new(player));
                            Some(events)
                        }
                        Err(e) => {
                            let e = Arc::new(e);
                            this.state = PlayerState::Error(e.clone());
                            cx.emit(PlayerEvent::Error(e));
                            None
                        }
                    };
                    cx.emit(PlayerEvent::StateChanged(this.state.clone()));
                    cx.notify();
                    events
                })
                .ok()
                .flatten();
            let Some(events) = events else { return };
            // Forward player events until the entity is released.
            loop {
                cx.background_executor().timer(EVENT_POLL).await;
                let batch: Vec<_> = events.try_iter().collect();
                if batch.is_empty() {
                    continue;
                }
                let alive = this.update(cx, |this, cx| {
                    for event in batch {
                        if let PlayerEvent::StateChanged(state) = &event {
                            this.state = state.clone();
                        }
                        cx.emit(event);
                    }
                    cx.notify();
                });
                if alive.is_err() {
                    return;
                }
            }
        });
        Self {
            player: None,
            state: PlayerState::Loading,
            image: None,
            image_pts: None,
            retired: Retired::default(),
            debug: DebugStats::from_env(),
            output_size: None,
            volume: 1.0,
            muted: false,
            metadata: None,
            cover: None,
            _tasks: vec![open],
        }
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        if let Some(p) = &self.player {
            p.play();
            self.state = p.state();
            cx.notify();
        }
    }

    pub fn pause(&mut self, cx: &mut Context<Self>) {
        if let Some(p) = &self.player {
            p.pause();
            self.state = p.state();
            cx.notify();
        }
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        if self.is_playing() { self.pause(cx) } else { self.play(cx) }
    }

    pub fn seek(&mut self, to: Duration, cx: &mut Context<Self>) {
        if let Some(p) = &self.player {
            p.seek(to);
            self.state = p.state();
            cx.notify();
        }
    }

    /// 0.0..=1.0 (clamped). Applied instantly.
    pub fn set_volume(&mut self, volume: f32, cx: &mut Context<Self>) {
        self.volume = if volume.is_nan() { 0.0 } else { volume.clamp(0.0, 1.0) };
        if let Some(p) = &self.player {
            p.set_volume(self.volume);
        }
        cx.notify();
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn set_muted(&mut self, muted: bool, cx: &mut Context<Self>) {
        self.muted = muted;
        if let Some(p) = &self.player {
            p.set_muted(muted);
        }
        cx.notify();
    }

    pub fn is_muted(&self) -> bool {
        self.muted
    }

    /// The underlying `alhazen_core::Player` once opened (`None` while loading or after an open
    /// error), for anything this entity does not wrap.
    /// Tags of the open media (title, artist, album, …), when it has any.
    pub fn metadata(&self) -> Option<Metadata> {
        self.metadata.clone()
    }

    /// The embedded cover art, decoded. `None` until it is decoded, and when there is none.
    pub fn cover(&self) -> Option<Arc<RenderImage>> {
        self.cover.clone()
    }

    pub fn player(&self) -> Option<&Arc<Player>> {
        self.player.as_ref()
    }

    /// `false` while loading.
    pub fn has_video(&self) -> bool {
        self.player.as_ref().is_some_and(|p| p.has_video())
    }

    /// Whether sound is playing (`false` while loading, without audio, or without an output).
    pub fn has_audio(&self) -> bool {
        self.player.as_ref().is_some_and(|p| p.has_audio())
    }

    pub fn state(&self) -> PlayerState {
        self.player.as_ref().map(|p| p.state()).unwrap_or_else(|| self.state.clone())
    }

    pub fn is_playing(&self) -> bool {
        matches!(self.state(), PlayerState::Playing | PlayerState::Buffering)
    }

    pub fn position(&self) -> Duration {
        self.player.as_ref().map(|p| p.position()).unwrap_or_default()
    }

    /// `false` while loading, or for streams that cannot seek; hide seek controls then.
    pub fn is_seekable(&self) -> bool {
        self.player.as_ref().is_some_and(|p| p.is_seekable())
    }

    pub fn duration(&self) -> Option<Duration> {
        self.player.as_ref().and_then(|p| p.duration())
    }

    pub fn video_size(&self) -> Option<(u32, u32)> {
        self.player.as_ref().and_then(|p| p.video_size())
    }

    /// HLS: the stream's variants (qualities); empty for other media and while loading.
    #[cfg(feature = "hls")]
    pub fn variants(&self) -> Vec<alhazen_core::hls::VariantInfo> {
        self.player.as_ref().map(|p| p.variants()).unwrap_or_default()
    }

    /// HLS: the variant playing now (an index into `variants()`).
    #[cfg(feature = "hls")]
    pub fn current_variant(&self) -> Option<usize> {
        self.player.as_ref().and_then(|p| p.current_variant())
    }

    /// HLS: plays this variant from the next segment on; `Variant::Auto` (the default) adapts to
    /// the connection.
    #[cfg(feature = "hls")]
    pub fn set_variant(&mut self, v: alhazen_core::hls::Variant, cx: &mut Context<Self>) {
        if let Some(p) = &self.player {
            p.set_variant(v);
        }
        cx.notify();
    }

    /// Tells the player how large the video is displayed (device pixels), so 4K frames shown
    /// in a small window are scaled down before conversion and upload.
    pub(crate) fn set_display_size(&mut self, size: (u32, u32)) {
        let Some(player) = self.player.as_ref() else { return };
        if self.output_size != Some(size) {
            self.output_size = Some(size);
            player.set_max_output_size(Some(size));
        }
    }

    /// The image to paint for the current frame, older images the caller must now remove from
    /// the sprite atlas, and whether the frame is new.
    pub(crate) fn frame_image(&mut self) -> (Option<Arc<RenderImage>>, Vec<Arc<RenderImage>>, bool) {
        let current = || self.image.as_ref().map(|(i, _)| i.clone());
        let Some(frame) = self.player.as_ref().and_then(|p| p.current_frame()) else {
            return (current(), Vec::new(), false);
        };
        let VideoFrame::Cpu { width, height, bgra, pts } = frame;
        if let Some((image, data)) = &self.image
            && Arc::ptr_eq(data, &bgra)
        {
            return (Some(image.clone()), Vec::new(), false);
        }
        let Some(buffer) = image::RgbaImage::from_raw(width, height, bgra.to_vec()) else {
            return (current(), Vec::new(), false);
        };
        self.image_pts = Some(pts);
        // GPUI's RenderImage expects BGRA data even though the container type says RGBA.
        let image = Arc::new(RenderImage::new([image::Frame::new(buffer)]));
        let drop = match self.image.replace((image.clone(), bgra)) {
            Some((previous, _)) => self.retired.push(previous),
            None => Vec::new(),
        };
        (Some(image), drop, true)
    }
}

impl VideoPlayer {
    /// Records the UI-thread cost of painting a new frame (`ALHAZEN_DEBUG` only) and prints a
    /// summary once a second: frames shown, paint cost, A/V offset, drops and the video backend.
    pub(crate) fn debug_paint(&mut self, new_frame: bool, cost: Duration) {
        let frame_pts = self.image_pts;
        let Some(d) = self.debug.as_mut() else { return };
        if new_frame {
            d.new_frames += 1;
            d.paint_cost += cost;
            d.worst_paint = d.worst_paint.max(cost);
        }
        if d.since.elapsed() < Duration::from_secs(1) {
            return;
        }
        let Some(player) = self.player.as_ref() else { return };
        let stats = player.stats();
        let position = player.position();
        // Only meaningful while playing: around a seek the old frame is compared with the new position.
        let playing = player.state() == PlayerState::Playing;
        let offset_ms = frame_pts.filter(|_| playing).map(|p| position.as_secs_f64() * 1e3 - p.as_secs_f64() * 1e3);
        eprintln!(
            "[alhazen] pos {:7.2}s | shown {:3} fps | paint avg {:5.1} ms, worst {:5.1} ms | frame behind clock {} | dropped {:3}/s | state {:?} | video {}",
            position.as_secs_f64(),
            d.new_frames,
            d.paint_cost.as_secs_f64() * 1e3 / d.new_frames.max(1) as f64,
            d.worst_paint.as_secs_f64() * 1e3,
            offset_ms.map_or("-".into(), |o| format!("{o:6.1} ms")),
            stats.frames_dropped - d.dropped_at_start,
            player.state(),
            stats.video_backend.unwrap_or("-"),
        );
        *d = DebugStats { since: std::time::Instant::now(), new_frames: 0, paint_cost: Duration::ZERO, worst_paint: Duration::ZERO, dropped_at_start: stats.frames_dropped };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retired_images_are_dropped_only_after_newer_ones_replaced_them() {
        let mut r = Retired::default();
        assert!(r.push(1).is_empty());
        assert!(r.push(2).is_empty());
        assert!(r.push(3).is_empty(), "the GPU may still sample the last {RETIRE_AFTER}");
        assert_eq!(r.push(4), [1]);
        assert_eq!(r.push(5), [2]);
        assert_eq!(r.drain_all(), [3, 4, 5]);
    }
}
