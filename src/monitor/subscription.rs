use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cosmic::iced::{
    futures::{SinkExt, Stream},
    stream,
};
use tokio::sync::watch::Receiver;

use crate::app::AppMsg;

use super::backend::{DisplayBackend, DisplayId, EventToSub};
use super::enumeration::enumerate_displays;

enum State {
    Waiting,
    Fetch(Option<tokio::sync::watch::Sender<EventToSub>>),
    Ready(
        HashMap<DisplayId, Arc<Mutex<DisplayBackend>>>,
        tokio::sync::watch::Sender<EventToSub>,
        Receiver<EventToSub>,
    ),
}

pub fn sub() -> impl Stream<Item = AppMsg> {
    stream::channel(100, |mut output| async move {
        let mut state = State::Fetch(None); // Start immediately, no waiting
        let mut failed_attempts = 0;
        // Cache of successfully initialized displays
        let mut display_cache: HashMap<DisplayId, Arc<Mutex<DisplayBackend>>> = HashMap::new();

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
                            let mut guard = backend.lock().unwrap();
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

                    output
                        .send(AppMsg::SubscriptionReady((res, tx.clone())))
                        .await
                        .unwrap();

                    // Reset failed_attempts after successful enumeration
                    failed_attempts = 0;

                    state = State::Ready(all_displays, tx, rx);
                }
                State::Ready(displays, tx, rx) => {
                    rx.changed().await.unwrap();

                    let last = rx.borrow_and_update().clone();
                    match last {
                        EventToSub::Refresh => {
                            for (id, display) in displays {
                                let display_clone = display.clone();
                                let id_clone = id.clone();

                                // Read brightness in spawn_blocking with retry logic
                                let res = tokio::task::spawn_blocking(move || {
                                    let mut display_guard = display_clone.lock().unwrap();

                                    // Retry once if first attempt fails (DDC/CI may be busy)
                                    match display_guard.get_brightness() {
                                        Ok(v) => Ok(v),
                                        Err(e) => {
                                            // DDC/CI may still be processing previous command
                                            // Wait longer to ensure it's ready
                                            std::thread::sleep(std::time::Duration::from_millis(100));
                                            match display_guard.get_brightness() {
                                                Ok(v) => Ok(v),
                                                Err(e2) => Err(e2)
                                            }
                                        }
                                    }
                                }).await.unwrap();

                                match res {
                                    Ok(value) => {
                                        output
                                            .send(AppMsg::BrightnessWasUpdated(
                                                id_clone,
                                                value,
                                            ))
                                            .await
                                            .unwrap();
                                    }
                                    Err(err) => error!("{:?}", err),
                                }
                            }
                        }
                        EventToSub::Set(id, value) => {
                            debug_assert!(value <= 100);
                            let display = displays.get_mut(&id).unwrap().clone();

                            let j = tokio::task::spawn_blocking(move || {
                                if let Err(err) = display
                                    .lock()
                                    .unwrap()
                                    .set_brightness(value)
                                {
                                    error!("{:?}", err);
                                }
                            });

                            j.await.unwrap();
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                        EventToSub::ReEnumerate => {
                            // Update cache with current displays before re-enumerating
                            display_cache = displays.clone();

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
