use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _, StyledExt, WindowExt, button::{Button, ButtonVariants}, h_flex, v_flex
};

use crate::{component::quartz_logo::QuartzLogo, interface_config::InterfaceConfig};

pub fn should_show(cx: &App) -> bool {
    let Some(current) = option_env!("PANDORA_RELEASE_VERSION") else {
        return false;
    };
    InterfaceConfig::get(cx).last_seen_release_version.as_deref() != Some(current)
}

pub fn open(window: &mut Window, cx: &mut App) {
    let Some(current) = option_env!("PANDORA_RELEASE_VERSION") else {
        return;
    };

    window.open_dialog(cx, move |dialog, _window, cx| {
        dialog
            .title(t::whats_new::title())
            .overlay_closable(false)
            .child(
                v_flex()
                    .gap_4()
                    .p_2()
                    .child(
                        h_flex()
                            .gap_3()
                            .items_center()
                            .child(QuartzLogo::new(px(48.0)))
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(div().text_lg().font_semibold().child(t::whats_new::welcome()))
                                    .child(div().text_sm().text_color(cx.theme().muted_foreground).child(t::whats_new::version(current))),
                            ),
                    )
                    .child(div().text_sm().text_color(cx.theme().muted_foreground).child(t::whats_new::subtitle()))
                    .child(
                        v_flex()
                            .gap_2()
                            .child(feature_item(t::whats_new::feature_home()))
                            .child(feature_item(t::whats_new::feature_refresh_anim()))
                            .child(feature_item(t::whats_new::feature_import()))
                            .child(feature_item(t::whats_new::feature_mod_link()))
                            .child(feature_item(t::whats_new::feature_updates())),
                    ),
            )
            .footer(
                Button::new("whats_new_ok")
                    .success()
                    .label(t::common::ok())
                    .on_click(move |_, window, cx| {
                        InterfaceConfig::get_mut(cx).last_seen_release_version = Some(current.into());
                        window.close_dialog(cx);
                    }),
            )
    });
}

fn feature_item(text: impl Into<SharedString>) -> impl IntoElement {
    h_flex()
        .gap_2()
        .items_start()
        .child(div().text_sm().child("•"))
        .child(div().text_sm().child(text.into()))
}
