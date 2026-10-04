use std::sync::Arc;
use std::time::Duration;

use gpui::{App, Context, EventEmitter, RenderImage, Task};
use video_core::{Player, PlayerConfig, PlayerEvent, PlayerState, Source, VideoFrame};

/// How often player events are forwarded to GPUI.
const EVENT_POLL: Duration = Duration::from_millis(16);

/// A video player entity. Create with `cx.new(|cx| VideoPlayer::new(source, config, cx))`
/// and render with [`crate::video_view`].
pub struct VideoPlayer {
    player: Option<Arc<Player>>,
    state: PlayerState,
    /// The image currently in GPUI's sprite atlas, and the frame data it was built from.
    image: Option<(Arc<RenderImage>, Arc<[u8]>)>,
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
                            this.state = player.state();
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
        Self { player: None, state: PlayerState::Loading, image: None, _tasks: vec![open] }
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

    pub fn state(&self) -> PlayerState {
        self.player.as_ref().map(|p| p.state()).unwrap_or_else(|| self.state.clone())
    }

    pub fn is_playing(&self) -> bool {
        matches!(self.state(), PlayerState::Playing | PlayerState::Buffering)
    }

    pub fn position(&self) -> Duration {
        self.player.as_ref().map(|p| p.position()).unwrap_or_default()
    }

    pub fn duration(&self) -> Option<Duration> {
        self.player.as_ref().and_then(|p| p.duration())
    }

    pub fn video_size(&self) -> Option<(u32, u32)> {
        self.player.as_ref().and_then(|p| p.video_size())
    }

    /// The image to paint for the current frame. Returns the new image and, when the frame
    /// changed, the previous image, which the caller must remove from the sprite atlas.
    pub(crate) fn frame_image(&mut self) -> (Option<Arc<RenderImage>>, Option<Arc<RenderImage>>) {
        let Some(frame) = self.player.as_ref().and_then(|p| p.current_frame()) else {
            return (self.image.as_ref().map(|(i, _)| i.clone()), None);
        };
        let VideoFrame::Cpu { width, height, bgra, .. } = frame;
        if let Some((image, data)) = &self.image
            && Arc::ptr_eq(data, &bgra)
        {
            return (Some(image.clone()), None);
        }
        let Some(buffer) = image::RgbaImage::from_raw(width, height, bgra.to_vec()) else {
            return (self.image.as_ref().map(|(i, _)| i.clone()), None);
        };
        // GPUI's RenderImage expects BGRA data even though the container type says RGBA.
        let image = Arc::new(RenderImage::new([image::Frame::new(buffer)]));
        let previous = self.image.replace((image.clone(), bgra)).map(|(i, _)| i);
        (Some(image), previous)
    }
}
