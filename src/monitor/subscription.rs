use std::collections::HashMap;
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

        loop {
            match &mut state {
                State::Waiting => {
                    // Only wait 100ms between retries, no exponential backoff
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    state = State::Fetch(None);
                }
                State::Fetch(existing_sender) => {
                    let (res, displays, some_failed) = enumerate_displays().await;

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

                    state = State::Ready(displays, tx, rx);
                }
                State::Ready(displays, tx, rx) => {
                    rx.changed().await.unwrap();

                    let last = rx.borrow_and_update().clone();
                    match last {
                        EventToSub::Refresh => {
                            for (id, display) in displays {
                                let res = display
                                    .lock()
                                    .unwrap()
                                    .get_brightness();

                                match res {
                                    Ok(value) => {
                                        output
                                            .send(AppMsg::BrightnessWasUpdated(
                                                id.clone(),
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
                            // Transition back to Fetch state with existing sender
                            // This will re-enumerate displays while keeping the same channel
                            info!("ReEnumerate event received, re-enumerating displays");
                            state = State::Fetch(Some(tx.clone()));
                        }
                    }
                }
            }
        }
    })
}
