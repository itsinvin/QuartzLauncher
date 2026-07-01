use gpui::{hsla, px};
use gpui_component::Theme;

/// Minecraft Block of Quartz — off-white crystal accents on a cool dark base.
pub fn apply_quartz_branding(theme: &mut Theme) {
    let colors = &mut theme.colors;

    // Deep charcoal-blue base (launcher dark mode)
    colors.background = hsla(220.0, 0.14, 0.08, 1.0);
    colors.foreground = hsla(210.0, 0.10, 0.94, 1.0);
    colors.muted = hsla(220.0, 0.10, 0.14, 1.0);
    colors.muted_foreground = hsla(220.0, 0.06, 0.58, 1.0);
    colors.border = hsla(220.0, 0.08, 0.24, 1.0);
    colors.input = hsla(220.0, 0.10, 0.16, 1.0);

    // Quartz white primary — like the block texture highlight
    colors.accent = hsla(45.0, 0.08, 0.82, 1.0);
    colors.accent_foreground = hsla(220.0, 0.20, 0.12, 1.0);

    colors.primary = hsla(0.0, 0.0, 0.88, 1.0);
    colors.primary_foreground = hsla(220.0, 0.18, 0.10, 1.0);
    colors.primary_hover = hsla(0.0, 0.0, 0.94, 1.0);
    colors.primary_active = hsla(0.0, 0.0, 0.78, 1.0);

    colors.secondary = hsla(220.0, 0.12, 0.16, 1.0);
    colors.secondary_foreground = hsla(210.0, 0.08, 0.85, 1.0);

    colors.success = hsla(142.0, 0.55, 0.40, 1.0);
    colors.success_foreground = hsla(0.0, 0.0, 0.98, 1.0);

    colors.info = hsla(195.0, 0.70, 0.50, 1.0);
    colors.info_foreground = hsla(0.0, 0.0, 0.98, 1.0);

    colors.warning = hsla(38.0, 0.85, 0.50, 1.0);
    colors.danger = hsla(0.0, 0.65, 0.48, 1.0);

    colors.ring = hsla(45.0, 0.12, 0.75, 1.0);
    colors.link = hsla(0.0, 0.0, 0.82, 1.0);
    colors.link_hover = hsla(195.0, 0.75, 0.65, 1.0);
    colors.selection = hsla(45.0, 0.15, 0.55, 0.45);

    // Sidebar — slightly lifted panel with quartz stripe accent
    colors.sidebar = hsla(220.0, 0.14, 0.10, 1.0);
    colors.sidebar_foreground = hsla(210.0, 0.08, 0.88, 1.0);
    colors.sidebar_border = hsla(220.0, 0.10, 0.20, 1.0);
    colors.sidebar_accent = hsla(220.0, 0.10, 0.18, 1.0);
    colors.sidebar_accent_foreground = hsla(0.0, 0.0, 0.92, 1.0);
    colors.sidebar_primary = hsla(0.0, 0.0, 0.85, 1.0);
    colors.sidebar_primary_foreground = hsla(220.0, 0.18, 0.10, 1.0);

    colors.popover = hsla(220.0, 0.14, 0.12, 1.0);
    colors.list_hover = hsla(220.0, 0.12, 0.18, 1.0);
    colors.list_active = hsla(220.0, 0.10, 0.22, 1.0);

    colors.chart_1 = hsla(0.0, 0.0, 0.80, 1.0);
    colors.chart_2 = hsla(195.0, 0.65, 0.52, 1.0);
    colors.chart_3 = hsla(142.0, 0.50, 0.42, 1.0);
    colors.chart_4 = hsla(38.0, 0.75, 0.52, 1.0);
    colors.chart_5 = hsla(270.0, 0.40, 0.60, 1.0);

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
