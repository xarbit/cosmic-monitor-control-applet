use std::os::fd::AsRawFd;

/// Monitors udev for display hotplug events
///
/// This runs in a dedicated blocking thread because udev's MonitorSocket is not Send.
/// It uses libc::poll() to wait for events on the udev socket.
pub struct UdevMonitor {
    socket: udev::MonitorSocket,
}

impl UdevMonitor {
    /// Create a new udev monitor for display events
    ///
    /// Monitors DRM subsystem with device type filter for connectors
    /// This significantly reduces false positives from other DRM events
    pub fn new() -> Result<Self, std::io::Error> {
        let socket = udev::MonitorBuilder::new()?
            .match_subsystem_devtype("drm", "drm_minor")?
            .listen()?;

        Ok(Self { socket })
    }

    /// Run the monitoring loop, calling the callback for each event
    ///
    /// This function blocks indefinitely, polling the udev socket.
    /// Returns only if there's a poll error.
    pub fn run<F>(self, mut callback: F) -> std::io::Error
    where
        F: FnMut(udev::Event) -> bool, // Returns true to continue, false to stop
    {
        info!("Display hotplug monitoring started (monitoring drm subsystem with device type filter)");

        let fd = self.socket.as_raw_fd();

        loop {
            // Use poll to wait for socket to be readable
            let mut poll_fd = libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            };

            debug!("Waiting for udev events...");

            // Block until socket has data (negative timeout = wait forever)
            let poll_result = unsafe { libc::poll(&mut poll_fd, 1, -1) };

            if poll_result < 0 {
                let err = std::io::Error::last_os_error();
                error!("Poll error: {}", err);
                return err;
            }

            if poll_result == 0 {
                // Timeout (shouldn't happen with -1 timeout)
                debug!("Poll timeout");
                continue;
            }

            debug!("Poll returned {}, revents: {}", poll_result, poll_fd.revents);

            // Socket is ready, check for events
            if let Some(event) = self.socket.iter().next() {
                info!("udev event: type={:?}, subsystem={:?}, devtype={:?}, syspath={:?}",
                      event.event_type(),
                      event.subsystem(),
                      event.devtype(),
                      event.syspath());

                match event.event_type() {
                    udev::EventType::Add | udev::EventType::Remove | udev::EventType::Change => {
                        info!("Display event detected: {:?} at {:?}",
                              event.event_type(), event.syspath());

                        // Call the callback - if it returns false, stop monitoring
                        if !callback(event) {
                            info!("Display hotplug monitoring stopped by callback");
                            return std::io::Error::new(
                                std::io::ErrorKind::Interrupted,
                                "Stopped by callback"
                            );
                        }
                    }
                    _ => {}
                }
            } else {
                debug!("Poll indicated ready but no event available");
            }
        }
    }
}
