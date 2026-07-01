use gpui::{prelude::*, App, IntoElement, Pixels, RenderOnce, Window};
use once_cell::sync::Lazy;
use schema::unique_bytes::UniqueBytes;

use crate::png_render_cache::{self, ImageTransformation};

static QUARTZ_BLOCK: Lazy<UniqueBytes> = Lazy::new(|| {
    let file = crate::Assets::get("images/quartz_block.png")
        .expect("quartz_block.png missing from embedded assets");
    UniqueBytes::new(file.data.as_ref())
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
        let pixels = u32::from(self.size).max(1);
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
