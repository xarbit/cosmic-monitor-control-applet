use std::collections::{HashMap, HashSet};
use std::time::Duration;

use cosmic::iced::{
    futures::{SinkExt, Stream},
    stream,
};
use tokio::sync::watch::Receiver;

use crate::app::AppMsg;

use super::backend::{DisplayBackend, DisplayId, EventToSub};
use super::enumeration::enumerate_displays;
use super::manager::DisplayManager;

enum State {
    Waiting,
    Fetch(Option<tokio::sync::watch::Sender<EventToSub>>),
    Ready(
        tokio::sync::watch::Sender<EventToSub>,
        Receiver<EventToSub>,
    ),
}

pub fn sub(display_manager: DisplayManager) -> impl Stream<Item = AppMsg> {
    stream::channel(100, |mut output| async move {
        let mut state = State::Fetch(None); // Start immediately, no waiting
        let mut failed_attempts = 0;
        // Cache of successfully initialized displays (now managed by DisplayManager)
        let mut display_cache: HashMap<DisplayId, std::sync::Arc<tokio::sync::Mutex<DisplayBackend>>> = HashMap::new();

        loop {
            match &mut state {
                State::Waiting => {
                    // Only wait 100ms between retries, no exponential backoff
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    state = State::Fetch(None);
                }
                State::Fetch(existing_sender) => {
                    // Build set of known display IDs from cache
                    let known_ids: HashSet<DisplayId> = display_cache.keys().cloned().collect();
                    let is_re_enumerate = !display_cache.is_empty();

                    info!("Enumerating displays (cached: {}, re-enumerate: {})", display_cache.len(), is_re_enumerate);
                    let (mut res, new_displays, some_failed) = enumerate_displays(&known_ids).await;

                    // Merge: Add all newly enumerated displays to results
                    let mut all_displays = new_displays;

                    // Add cached displays back to results and all_displays
                    // Get current brightness for all cached displays
                    for (id, backend) in &display_cache {
                        // Get current brightness from cached backend
                        // This is fast since we skip the initialization/wake-up sequence
                        let (keep, name, brightness) = {
                            let mut guard = backend.lock().await;
                            match guard.get_brightness() {
                                Ok(b) => (true, guard.name(), b),
                                Err(_) => (false, String::new(), 0),
                            }
                        };

                        if keep {
                            res.insert(id.clone(), super::backend::MonitorInfo { name, brightness });
                            all_displays.insert(id.clone(), backend.clone());
                            if is_re_enumerate {
                                info!("Using cached display (quick read): {} (brightness: {})", id, brightness);
                            } else {
                                info!("Kept cached display: {} (brightness: {})", id, brightness);
                            }
                        } else {
                            info!("Removed stale cached display: {}", id);
                        }
                    }

                    // Update cache with all working displays (cached + new)
                    display_cache = all_displays.clone();

                    // Update the shared DisplayManager with all working displays
                    display_manager.update_displays(all_displays.clone()).await;

                    if some_failed {
                        failed_attempts += 1;
                    }

                    // If we have at least one monitor, send it to the UI immediately
                    // and retry failed monitors in the background
                    if !res.is_empty() {
                        // We have at least one working monitor, proceed to ready state
                    } else if some_failed && failed_attempts < 3 {
                        // No monitors detected yet, retry up to 3 times
                        state = State::Waiting;
                        continue;
                    }

                    let (tx, rx) = if let Some(sender) = existing_sender.take() {
                        // Reuse existing sender for re-enumeration
                        let rx = sender.subscribe();
                        (sender, rx)
                    } else {
                        // Create new channel for initial enumeration
                        let (tx, mut rx) = tokio::sync::watch::channel(EventToSub::Refresh);
                        rx.mark_unchanged();
                        (tx, rx)
                    };

                    if let Err(e) = output
                        .send(AppMsg::SubscriptionReady((res, tx.clone())))
                        .await
                    {
                        error!("Failed to send SubscriptionReady: {:?}", e);
                        // Channel closed, exit subscription
                        return;
                    }

                    // Reset failed_attempts after successful enumeration
                    failed_attempts = 0;

                    state = State::Ready(tx, rx);
                }
                State::Ready(tx, rx) => {
                    if let Err(e) = rx.changed().await {
                        error!("Monitor subscription channel closed: {:?}", e);
                        // Channel closed, exit subscription
                        return;
                    }

                    let last = rx.borrow_and_update().clone();
                    match last {
                        EventToSub::Refresh => {
                            // Get all display IDs from the DisplayManager
                            let display_ids = display_manager.get_all_ids().await;

                            for id in display_ids {
                                let display = match display_manager.get(&id).await {
                                    Some(d) => d,
                                    None => continue,
                                };
                                let id_clone = id.clone();

                                // Read brightness in spawn_blocking with retry logic
                                // Note: We use spawn_blocking to move blocking I/O off the async runtime
                                let res = tokio::task::spawn_blocking(move || {
                                    // Use blocking_lock() to acquire the lock from a blocking context
                                    // This is the proper way to lock tokio::Mutex from within spawn_blocking
                                    let mut display_guard = display.blocking_lock();

                                    // Retry once if first attempt fails (DDC/CI may be busy)
                                    match display_guard.get_brightness() {
                                        Ok(v) => Ok(v),
                                        Err(_e) => {
                                            // DDC/CI may still be processing previous command
                                            // Wait longer to ensure it's ready
                                            std::thread::sleep(std::time::Duration::from_millis(100));
                                            match display_guard.get_brightness() {
                                                Ok(v) => Ok(v),
                                                Err(e2) => Err(e2)
                                            }
                                        }
                                    }
                                }).await;

                                let res = match res {
                                    Ok(r) => r,
                                    Err(e) => {
                                        error!("spawn_blocking join error: {:?}", e);
                                        continue;
                                    }
                                };

                                match res {
                                    Ok(value) => {
                                        if let Err(e) = output
                                            .send(AppMsg::BrightnessWasUpdated(
                                                id_clone.clone(),
                                                value,
                                            ))
                                            .await
                                        {
                                            error!("Failed to send BrightnessWasUpdated for {}: {:?}", id_clone, e);
                                            return;
                                        }
                                    }
                                    Err(err) => {
                                        error!(
                                            display_id = %id_clone,
                                            error = ?err,
                                            "Failed to get brightness"
                                        );
                                    }
                                }
                            }
                        }
                        EventToSub::Set(id, value) => {
                            debug_assert!(value <= 100);
                            info!(">>> SUBSCRIPTION: Received Set command for {} = {}%", id, value);

                            let display = match display_manager.get(&id).await {
                                Some(d) => d,
                                None => {
                                    error!(
                                        display_id = %id,
                                        "Display not found in manager"
                                    );
                                    continue;
                                }
                            };

                            let id_clone = id.clone();
                            let value_clone = value;

                            // Set brightness in spawn_blocking to move blocking I/O off async runtime
                            let j = tokio::task::spawn_blocking(move || {
                                // Use blocking_lock() to acquire the lock from a blocking context
                                // This is the proper way to lock tokio::Mutex from within spawn_blocking
                                let mut display_guard = display.blocking_lock();

                                info!(">>> SUBSCRIPTION: Setting {} to {}%", id_clone, value_clone);
                                match display_guard.set_brightness(value_clone) {
                                    Ok(_) => {
                                        info!(">>> SUBSCRIPTION: Successfully set {} to {}%", id_clone, value_clone);
                                    }
                                    Err(err) => {
                                        error!(
                                            display_id = %id_clone,
                                            brightness = %value_clone,
                                            error = ?err,
                                            "Failed to set brightness"
                                        );
                                    }
                                }
                            });

                            if let Err(e) = j.await {
                                error!("spawn_blocking join error for Set: {:?}", e);
                            }
                            info!(">>> SUBSCRIPTION: Completed Set for {} = {}%", id, value);
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                        EventToSub::SetBatch(commands) => {
                            info!(">>> SUBSCRIPTION: Received SetBatch with {} commands", commands.len());

                            // Process all brightness commands
                            for (id, value) in commands {
                                debug_assert!(value <= 100);
                                info!(">>> SUBSCRIPTION: Processing batch command for {} = {}%", id, value);

                                let display = match display_manager.get(&id).await {
                                    Some(d) => d,
                                    None => {
                                        error!(
                                            display_id = %id,
                                            "Display not found in manager (batch)"
                                        );
                                        continue;
                                    }
                                };

                                let id_clone = id.clone();
                                let value_clone = value;

                                // Set brightness in spawn_blocking
                                let j = tokio::task::spawn_blocking(move || {
                                    let mut display_guard = display.blocking_lock();

                                    info!(">>> SUBSCRIPTION: Setting {} to {}% (batch)", id_clone, value_clone);
                                    match display_guard.set_brightness(value_clone) {
                                        Ok(_) => {
                                            info!(">>> SUBSCRIPTION: Successfully set {} to {}% (batch)", id_clone, value_clone);
                                        }
                                        Err(err) => {
                                            error!(
                                                display_id = %id_clone,
                                                brightness = %value_clone,
                                                error = ?err,
                                                "Failed to set brightness (batch)"
                                            );
                                        }
                                    }
                                });

                                if let Err(e) = j.await {
                                    error!("spawn_blocking join error for SetBatch: {:?}", e);
                                }
                                info!(">>> SUBSCRIPTION: Completed batch command for {} = {}%", id, value);
                                tokio::time::sleep(Duration::from_millis(50)).await;
                            }

                            info!(">>> SUBSCRIPTION: SetBatch completed");
                        }
                        EventToSub::ReEnumerate => {
                            // Cache is maintained by DisplayManager now
                            // Just keep local cache for re-enumeration optimization

                            // Transition back to Fetch state with existing sender
                            // The display_cache will be used to avoid re-probing known displays
                            info!("ReEnumerate event received (hotplug), re-enumerating with cache ({} displays)", display_cache.len());
                            state = State::Fetch(Some(tx.clone()));
                        }
                        EventToSub::ReEnumerateFull => {
                            // Clear cache for manual refresh - user wants full re-scan
                            info!("ReEnumerateFull event received (manual refresh), clearing cache and doing full probe");
                            display_cache.clear();

                            // Transition back to Fetch state with existing sender
                            // Empty cache will cause all displays to be probed
                            state = State::Fetch(Some(tx.clone()));
                        }
                    }
                }
            }
        }
    })
}
