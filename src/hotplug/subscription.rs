use std::time::Duration;
use cosmic::iced::{
    futures::{SinkExt, Stream},
    stream,
};

use crate::app::AppMsg;
use super::udev_monitor::UdevMonitor;

/// Subscription for automatic display hotplug detection
///
/// Uses a dedicated blocking thread for udev monitoring because MonitorSocket is not Send.
/// Communicates with the async UI task via a channel.
pub fn hotplug_subscription() -> impl Stream<Item = AppMsg> {
    stream::channel(10, |mut output| async move {
        // Create a channel to communicate from blocking thread to async task
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        // Spawn a dedicated blocking thread for udev monitoring
        std::thread::spawn(move || {
            // Create udev monitor in the blocking thread
            let monitor = match UdevMonitor::new() {
                Ok(m) => m,
                Err(e) => {
                    error!("Failed to initialize display hotplug monitoring: {}", e);
                    return;
                }
            };

            // Run the monitor - this blocks indefinitely
            let _err = monitor.run(|_event| {
                // Send notification to async task (non-blocking)
                match tx.try_send(()) {
                    Ok(_) => true, // Continue monitoring
                    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                        debug!("Hotplug channel full, skipping event (will debounce)");
                        true // Continue monitoring
                    }
                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                        error!("Hotplug channel closed, stopping monitor");
                        false // Stop monitoring
                    }
                }
            });

            error!("Display hotplug monitoring stopped");
        });

        // Async task receives notifications from blocking thread
        while rx.recv().await.is_some() {
            info!("Hotplug event received, debouncing...");

            // Debounce: drain all pending events
            while rx.try_recv().is_ok() {
                // Drain the channel
            }

            // Wait for hardware AND Wayland/iced to fully stabilize
            // DDC/CI displays need time to become available after hotplug
            // Wayland compositor also needs time to clean up display resources
            tokio::time::sleep(Duration::from_millis(5000)).await;

            // Trigger re-enumeration with cache (keeps existing working displays)
            info!("Hotplug settled, sending AppMsg::HotplugDetected");
            if output.send(AppMsg::HotplugDetected).await.is_err() {
                error!("Failed to send hotplug message");
            }
        }

        info!("Hotplug monitoring channel closed");
    })
}
