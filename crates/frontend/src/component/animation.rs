use std::time::Instant;

use gpui::Window;

const PAGE_FADE_MS: f32 = 180.0;

#[inline]
fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// Opacity for a page crossfade started at `start`. Returns `None` when finished.
pub fn page_fade_opacity(start: Instant) -> Option<f32> {
    let elapsed_ms = start.elapsed().as_secs_f32() * 1000.0;
    if elapsed_ms >= PAGE_FADE_MS {
        return None;
    }
    let t = ease_out_cubic(elapsed_ms / PAGE_FADE_MS);
    Some(0.72 + 0.28 * t)
}

pub fn request_next_frame(window: &mut Window) {
    window.request_animation_frame();
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
        return format!("{}m ago", diff.num_minutes()).into();
    }
    if diff.num_hours() < 24 {
        return format!("{}h ago", diff.num_hours()).into();
    }
    if diff.num_days() < 7 {
        return format!("{}d ago", diff.num_days()).into();
    }

    dt.format("%b %d, %Y").to_string().into()
}
