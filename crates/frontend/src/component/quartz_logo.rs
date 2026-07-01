use gpui::{img, prelude::*, ImageSource, IntoElement, ObjectFit, Pixels, RenderOnce, Resource, Window};

/// The Minecraft Block of Quartz texture used as the Quartz Launcher logo.
#[derive(IntoElement, Clone, Copy)]
pub struct QuartzLogo {
    size: Pixels,
}

impl QuartzLogo {
    pub fn new(size: Pixels) -> Self {
        Self { size }
    }
}

impl RenderOnce for QuartzLogo {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        img(ImageSource::Resource(Resource::Embedded(
            "images/quartz_block.png".into(),
        )))
        .size(self.size)
        .min_w(self.size)
        .min_h(self.size)
        .object_fit(ObjectFit::Contain)
    }
}
