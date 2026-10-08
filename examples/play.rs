//! Minimal player: `cargo run --release --example play -- <file-or-url>`.
//! The controls are plain GPUI divs, meant as a copy-paste starting point.

use std::time::Duration;

use gpui::{
    App, Application, Bounds, Context, Entity, SharedString, Window, WindowBounds, WindowOptions,
    div, prelude::*, px, relative, rgb, size,
};
use alhazen_gpui::{PlayerConfig, PlayerEvent, PlayerState, Source, VideoPlayer, video_view};

struct PlayerWindow {
    video: Entity<VideoPlayer>,
    /// Latest warning from the player (e.g. no audio output), shown in the status bar.
    warning: Option<String>,
}

impl PlayerWindow {
    fn new(source: Source, cx: &mut Context<Self>) -> Self {
        let config = PlayerConfig { autoplay: true, ..Default::default() };
        let video = cx.new(|cx| VideoPlayer::new(source, config, cx));
        cx.observe(&video, |_, _, cx| cx.notify()).detach();
        cx.subscribe(&video, |this, _, event: &PlayerEvent, cx| {
            if let PlayerEvent::Warning(w) = event {
                eprintln!("warning: {w}");
                this.warning = Some(w.clone());
                cx.notify();
            }
        })
        .detach();
        Self { video, warning: None }
    }

    fn change_volume(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.video.update(cx, |v, cx| v.set_volume(v.volume() + delta, cx));
    }

    fn skip(&mut self, delta: i64, cx: &mut Context<Self>) {
        self.video.update(cx, |v, cx| {
            let pos = v.position().as_millis() as i64 + delta * 1000;
            v.seek(Duration::from_millis(pos.max(0) as u64), cx);
        });
    }
}

fn button(id: &'static str, label: impl Into<SharedString>) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .px_3()
        .py_1()
        .rounded_md()
        .bg(rgb(0x3a3a3a))
        .hover(|s| s.bg(rgb(0x505050)))
        .cursor_pointer()
        .child(label.into())
}

fn fmt(d: Duration) -> String {
    let s = d.as_secs();
    format!("{}:{:02}", s / 60, s % 60)
}

impl Render for PlayerWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let v = self.video.read(cx);
        let (position, duration, state) = (v.position(), v.duration(), v.state());
        let progress = duration
            .filter(|d| !d.is_zero())
            .map(|d| (position.as_secs_f32() / d.as_secs_f32()).clamp(0.0, 1.0))
            .unwrap_or(0.0);
        let status = match (&state, &self.warning) {
            (PlayerState::Error(e), _) => format!("Error: {e}"),
            (other, Some(w)) => format!("{other:?} — {w}"),
            (other, None) => format!("{other:?}"),
        };
        let play_label = if v.is_playing() { "Pause" } else { "Play" };
        let mute_label = if v.is_muted() { "Unmute" } else { "Mute" };
        let volume = format!("Vol {:.0}%", v.volume() * 100.0);
        let audio_only = !v.has_video() && v.has_audio();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x101010))
            .text_color(rgb(0xffffff))
            .child(if audio_only {
                div().flex_1().w_full().flex().items_center().justify_center().child("♪ audio only").into_any_element()
            } else {
                video_view(self.video.clone()).flex_1().w_full().into_any_element()
            })
            .child(div().h(px(4.)).w_full().bg(rgb(0x303030)).child(
                div().h_full().w(relative(progress)).bg(rgb(0xe04040)),
            ))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .p_2()
                    .items_center()
                    .child(button("back", "-5s").on_click(cx.listener(|this, _, _, cx| this.skip(-5, cx))))
                    .child(button("play", play_label).on_click(cx.listener(|this, _, _, cx| {
                        this.video.update(cx, |v, cx| v.toggle(cx))
                    })))
                    .child(button("fwd", "+5s").on_click(cx.listener(|this, _, _, cx| this.skip(5, cx))))
                    .child(button("mute", mute_label).on_click(cx.listener(|this, _, _, cx| {
                        this.video.update(cx, |v, cx| v.set_muted(!v.is_muted(), cx))
                    })))
                    .child(button("vol-down", "Vol -").on_click(cx.listener(|this, _, _, cx| this.change_volume(-0.1, cx))))
                    .child(button("vol-up", "Vol +").on_click(cx.listener(|this, _, _, cx| this.change_volume(0.1, cx))))
                    .child(volume)
                    .child(format!(
                        "{} / {}",
                        fmt(position),
                        duration.map(fmt).unwrap_or_else(|| "--:--".into())
                    ))
                    .child(div().flex_1())
                    .child(status),
            )
    }
}

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_else(|| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/av1.webm").to_string()
    });
    let source = Source::parse(&arg).expect("invalid source");
    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(960.), px(600.)), cx);
        cx.open_window(
            WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() },
            |_, cx| cx.new(|cx| PlayerWindow::new(source, cx)),
        )
        .unwrap();
        cx.activate(true);
    });
}
