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
/// Only one instance across all applets will actually monitor - others will be no-ops.
pub fn hotplug_subscription() -> impl Stream<Item = AppMsg> {
    stream::channel(10, |mut output| async move {
        use std::fs::File;
        use std::os::unix::io::AsRawFd;

        // Try to acquire exclusive lock on hotplug monitor lock file
        // This ensures only one monitor runs across all applet instances
        let lock_path = dirs::runtime_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("cosmic-monitor-control-hotplug.lock");

        let lock_file = match File::create(&lock_path) {
            Ok(f) => f,
            Err(e) => {
                error!("Failed to create hotplug lock file: {}", e);
                return;
            }
        };

        // Try to acquire exclusive lock (non-blocking) using flock
        // flock is per-process, so multiple threads in same process can't conflict
        let lock_result = unsafe {
            libc::flock(lock_file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB)
        };

        if lock_result != 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EWOULDBLOCK) {
                info!("Hotplug monitor already running in another applet instance, this instance will poll for changes");
            } else {
                error!("Failed to acquire hotplug lock: {}", err);
                // Error acquiring lock, just sleep forever
                loop {
                    tokio::time::sleep(Duration::from_secs(3600)).await;
                }
            }

            // This instance is passive - watch the lock file for changes
            // When the active monitor processes a hotplug, the timestamp changes
            loop {
                // Check every 2 seconds if the lock file was modified (indicates hotplug activity)
                tokio::time::sleep(Duration::from_secs(2)).await;

                // Check if lock file was recently modified (within last 10 seconds)
                if let Ok(metadata) = std::fs::metadata(&lock_path) {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(elapsed) = modified.elapsed() {
                            if elapsed < Duration::from_secs(10) {
                                info!("Detected hotplug activity (lock file modified), triggering re-enumeration");
                                if output.send(AppMsg::HotplugDetected).await.is_err() {
                                    error!("Failed to send hotplug notification");
                                    return;
                                }
                                // Wait a bit before checking again to avoid spam
                                tokio::time::sleep(Duration::from_secs(10)).await;
                            }
                        }
                    }
                }
            }
        }

        info!("Acquired hotplug monitor lock, this instance will monitor display hotplug events");

        // Create a channel to communicate from blocking thread to async task
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        // Spawn a dedicated blocking thread for udev monitoring
        // Keep lock_file alive in the closure
        std::thread::spawn(move || {
            let _lock_guard = lock_file; // Keep lock alive
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
        let mut last_hotplug_time = std::time::Instant::now();
        #[allow(unused_assignments)]
        let mut is_processing = false;

        while rx.recv().await.is_some() {
            info!("Hotplug event received, debouncing...");

            // If already processing a hotplug, queue this event and wait
            if is_processing {
                warn!("Hotplug re-enumeration already in progress, queueing this event...");
                // Drain all immediate events and wait for next one
                while rx.try_recv().is_ok() {}
                // Wait for the next event after processing completes
                continue;
            }

            is_processing = true;

            // Debounce: drain all pending events
            let mut drained_count = 0;
            while rx.try_recv().is_ok() {
                drained_count += 1;
            }
            if drained_count > 0 {
                info!("Drained {} additional hotplug events", drained_count);
            }

            // Rate limiting: Ensure at least 1.5 seconds between re-enumerations
            let elapsed_since_last = last_hotplug_time.elapsed();
            if elapsed_since_last < Duration::from_millis(1500) {
                let additional_wait = Duration::from_millis(1500) - elapsed_since_last;
                info!("Rate limiting: waiting additional {:?} before re-enumeration", additional_wait);
                tokio::time::sleep(additional_wait).await;
            }

            // Wait for hardware to stabilize after hotplug
            // DDC/CI displays need time to become available after hotplug
            // Short delay since enumeration has built-in retries with timeouts
            info!("Waiting 1 second for hardware to stabilize...");
            tokio::time::sleep(Duration::from_millis(1000)).await;

            last_hotplug_time = std::time::Instant::now();

            // Touch the lock file to notify passive instances
            let _ = std::fs::File::create(&lock_path);

            // Trigger re-enumeration with cache (keeps existing working displays)
            info!("Hotplug settled, sending AppMsg::HotplugDetected");
            if output.send(AppMsg::HotplugDetected).await.is_err() {
                error!("Failed to send hotplug message");
            }

            is_processing = false;
            info!("Hotplug processing complete, ready for next event");
        }

        info!("Hotplug monitoring channel closed");
    })
}
