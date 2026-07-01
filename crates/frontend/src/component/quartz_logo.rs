use gpui::{prelude::*, App, IntoElement, Pixels, RenderOnce, Window};
use once_cell::sync::Lazy;
use schema::unique_bytes::UniqueBytes;

use crate::png_render_cache::{self, ImageTransformation};

static QUARTZ_BLOCK: Lazy<UniqueBytes> = Lazy::new(|| {
    UniqueBytes::new(include_bytes!("../../../assets/images/quartz_block.png"))
});

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
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let pixels = self.size.0.round().max(1.0) as u32;
        let transform = ImageTransformation::Resize {
            width: pixels,
            height: pixels,
        };

        png_render_cache::render_with_transform(QUARTZ_BLOCK.clone(), transform, cx)
            .size(self.size)
            .min_w(self.size)
            .min_h(self.size)
    }
}
