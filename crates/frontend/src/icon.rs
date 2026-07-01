use gpui_component::{Icon, IconNamed};
use gpui::*;

gpui_component::icon_named!(QuartzIcon, "../../assets/icons");

impl RenderOnce for QuartzIcon {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        Icon::new(self)
    }
}

impl QuartzIcon {
    pub fn pause_play(pause: bool) -> Self {
        if pause {
            Self::Pause
        } else {
            Self::Play
        }
    }
}
