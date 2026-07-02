use std::time::Duration;

use gpui::{percentage, prelude::FluentBuilder as _, Animation, AnimationExt as _, App, SharedString, Transformation, Window};
use gpui_component::Icon;

use crate::icon::QuartzIcon;

#[inline]
pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

#[inline]
pub fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// Smoothly interpolate between two values.
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn format_last_played(unix_ms: Option<i64>) -> gpui::SharedString {
    let Some(unix_ms) = unix_ms.filter(|&ms| ms > 0) else {
        return t::instance::never_played().into();
    };

    let Some(dt) = chrono::DateTime::from_timestamp_millis(unix_ms) else {
        return t::instance::never_played().into();
    };

    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(dt);

    if diff.num_minutes() < 1 {
        return t::instance::last_played_just_now().into();
    }
    if diff.num_hours() < 1 {
        return t::instance::last_played_minutes_ago(diff.num_minutes()).into();
    }
    if diff.num_hours() < 24 {
        return t::instance::last_played_hours_ago(diff.num_hours()).into();
    }
    if diff.num_days() < 7 {
        return t::instance::last_played_days_ago(diff.num_days()).into();
    }

    dt.format("%b %d, %Y").to_string().into()
}

/// Gentle breathing scale for the welcome-screen logo.
pub fn animated_logo_scale(window: &mut Window, cx: &mut App) -> f32 {
    #[derive(Default)]
    struct Pulse {
        phase: f32,
    }

    let state = window.use_keyed_state("quartz-logo-pulse", cx, |_, _| Pulse { phase: 0.0 });
    let mut scale = 1.0;

    state.update(cx, |pulse, cx| {
        pulse.phase += 0.06;
        scale = 1.0 + 0.05 * pulse.phase.sin();
        window.request_animation_frame();
        cx.notify();
    });

    scale
}

pub fn format_playtime(total_secs: u64) -> SharedString {
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{hours}h {minutes}m").into()
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s").into()
    } else {
        format!("{seconds}s").into()
    }
}

pub fn refresh_icon(generation: u64) -> Icon {
    Icon::new(QuartzIcon::RefreshCcw)
        .with_animation(
            ("content-refresh", generation),
            Animation::new(Duration::from_millis(650)).with_easing(ease_out_cubic),
            |icon, delta| icon.transform(Transformation::rotate(percentage(delta))),
        )
}
