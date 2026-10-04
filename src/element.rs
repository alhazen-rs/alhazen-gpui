use gpui::{
    App, Bounds, Corners, Element, ElementId, Entity, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, ObjectFit, Pixels, Refineable, Style, StyleRefinement, Styled, Window,
};

use crate::VideoPlayer;

/// Renders the current frame of a [`VideoPlayer`]. Style it like a `div` (`.size_full()`, etc.).
pub fn video_view(player: Entity<VideoPlayer>) -> VideoView {
    VideoView { player, style: StyleRefinement::default(), object_fit: ObjectFit::Contain }
}

pub struct VideoView {
    player: Entity<VideoPlayer>,
    style: StyleRefinement,
    object_fit: ObjectFit,
}

impl VideoView {
    pub fn object_fit(mut self, fit: ObjectFit) -> Self {
        self.object_fit = fit;
        self
    }
}

impl Styled for VideoView {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl IntoElement for VideoView {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for VideoView {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.refine(&self.style);
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let (image, previous) = self.player.update(cx, |p, _| p.frame_image());
        if let Some(previous) = previous {
            // Without this the sprite atlas grows by one frame per frame.
            let _ = window.drop_image(previous);
        }
        if let Some(image) = image {
            let fit = self.object_fit.get_bounds(bounds, image.size(0));
            let _ = window.paint_image(fit, Corners::default(), image, 0, false);
        }
        if self.player.read(cx).is_playing() {
            window.request_animation_frame();
        }
    }
}
