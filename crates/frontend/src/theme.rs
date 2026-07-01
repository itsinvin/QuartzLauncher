use gpui::{hsla, px};
use gpui_component::Theme;

/// Quartz crystal + Minecraft-inspired dark palette.
pub fn apply_quartz_branding(theme: &mut Theme) {
    let colors = &mut theme.colors;

    colors.background = hsla(240.0, 0.12, 0.07, 1.0);
    colors.foreground = hsla(220.0, 0.15, 0.92, 1.0);
    colors.muted = hsla(240.0, 0.10, 0.14, 1.0);
    colors.muted_foreground = hsla(240.0, 0.08, 0.58, 1.0);
    colors.border = hsla(260.0, 0.18, 0.22, 1.0);
    colors.input = hsla(260.0, 0.16, 0.20, 1.0);

    colors.accent = hsla(270.0, 0.55, 0.52, 1.0);
    colors.accent_foreground = hsla(0.0, 0.0, 0.98, 1.0);

    colors.primary = hsla(270.0, 0.60, 0.48, 1.0);
    colors.primary_foreground = hsla(0.0, 0.0, 0.98, 1.0);
    colors.primary_hover = hsla(270.0, 0.58, 0.55, 1.0);
    colors.primary_active = hsla(270.0, 0.62, 0.42, 1.0);

    colors.secondary = hsla(240.0, 0.12, 0.16, 1.0);
    colors.secondary_foreground = hsla(220.0, 0.12, 0.85, 1.0);

    colors.success = hsla(142.0, 0.55, 0.38, 1.0);
    colors.success_foreground = hsla(0.0, 0.0, 0.98, 1.0);

    colors.info = hsla(195.0, 0.75, 0.48, 1.0);
    colors.info_foreground = hsla(0.0, 0.0, 0.98, 1.0);

    colors.warning = hsla(38.0, 0.85, 0.50, 1.0);
    colors.danger = hsla(0.0, 0.65, 0.48, 1.0);

    colors.ring = hsla(190.0, 0.85, 0.55, 1.0);
    colors.link = hsla(270.0, 0.70, 0.72, 1.0);
    colors.link_hover = hsla(190.0, 0.80, 0.65, 1.0);
    colors.selection = hsla(270.0, 0.45, 0.35, 0.55);

    colors.sidebar = hsla(245.0, 0.14, 0.09, 1.0);
    colors.sidebar_foreground = hsla(220.0, 0.12, 0.88, 1.0);
    colors.sidebar_border = hsla(260.0, 0.16, 0.18, 1.0);
    colors.sidebar_accent = hsla(270.0, 0.40, 0.22, 1.0);
    colors.sidebar_accent_foreground = hsla(270.0, 0.55, 0.88, 1.0);
    colors.sidebar_primary = hsla(270.0, 0.55, 0.52, 1.0);
    colors.sidebar_primary_foreground = hsla(0.0, 0.0, 0.98, 1.0);

    colors.popover = hsla(245.0, 0.14, 0.11, 1.0);
    colors.list_hover = hsla(270.0, 0.25, 0.18, 1.0);
    colors.list_active = hsla(270.0, 0.35, 0.24, 1.0);

    colors.chart_1 = hsla(270.0, 0.60, 0.55, 1.0);
    colors.chart_2 = hsla(190.0, 0.70, 0.50, 1.0);
    colors.chart_3 = hsla(142.0, 0.50, 0.42, 1.0);
    colors.chart_4 = hsla(38.0, 0.75, 0.52, 1.0);
    colors.chart_5 = hsla(320.0, 0.55, 0.55, 1.0);

    theme.radius = px(8.0);
    theme.radius_lg = px(12.0);
}

pub fn apply_saved_theme(cx: &mut gpui::App) {
    let theme_name = crate::interface_config::InterfaceConfig::get(cx).active_theme.trim();
    if theme_name.is_empty() {
        return;
    }

    let Some(theme) = gpui_component::ThemeRegistry::global(cx)
        .themes()
        .get(&gpui::SharedString::new(theme_name))
        .cloned()
    else {
        return;
    };

    gpui_component::Theme::global_mut(cx).apply_config(&theme);
    apply_quartz_branding(gpui_component::Theme::global_mut(cx));
}
