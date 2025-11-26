use crate::icon::{icon_high, icon_low, icon_medium, icon_off};
use cosmic::widget::icon;

/// Get the appropriate brightness icon based on brightness level
pub fn brightness_icon(brightness: f32) -> icon::Handle {
    if brightness > 0.66 {
        icon_high()
    } else if brightness > 0.33 {
        icon_medium()
    } else if brightness > 0.0 {
        icon_low()
    } else {
        icon_off()
    }
}
