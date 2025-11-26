/// Display hotplug detection using udev
///
/// This module provides automatic detection of display plug/unplug events.
/// It monitors udev for DRM and I2C device changes and notifies the UI
/// when displays are added or removed.

mod udev_monitor;
mod subscription;

pub use subscription::hotplug_subscription;
