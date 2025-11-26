// SPDX-License-Identifier: GPL-3.0-only
//! Centralized display manager
//!
//! This module provides a singleton display manager that manages all external
//! displays. Both the UI and daemon access displays through this manager,
//! ensuring only one I2C connection per physical monitor.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::backend::{DisplayBackend, DisplayId};

/// Shared display manager instance
///
/// This manages all display backends and ensures only one I2C connection
/// per physical monitor. Both UI and daemon access displays through this.
pub struct DisplayManager {
    displays: Arc<RwLock<HashMap<DisplayId, Arc<tokio::sync::Mutex<DisplayBackend>>>>>,
}

impl DisplayManager {
    /// Create a new display manager
    pub fn new() -> Self {
        Self {
            displays: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get a reference to a display by ID
    pub async fn get(&self, id: &str) -> Option<Arc<tokio::sync::Mutex<DisplayBackend>>> {
        let displays = self.displays.read().await;
        displays.get(id).cloned()
    }

    /// Get all display IDs
    pub async fn get_all_ids(&self) -> Vec<DisplayId> {
        let displays = self.displays.read().await;
        displays.keys().cloned().collect()
    }

    /// Update the display map with new displays
    ///
    /// This merges new displays with existing ones, keeping existing
    /// connections alive and only adding new ones.
    pub async fn update_displays(&self, new_displays: HashMap<DisplayId, Arc<tokio::sync::Mutex<DisplayBackend>>>) {
        let mut displays = self.displays.write().await;

        // Keep existing displays that are still present
        let existing_ids: Vec<_> = displays.keys().cloned().collect();
        for id in existing_ids {
            if !new_displays.contains_key(&id) {
                // Display was removed
                displays.remove(&id);
                info!("Display {} removed from manager", id);
            }
        }

        // Add new displays
        for (id, backend) in new_displays {
            if !displays.contains_key(&id) {
                info!("Display {} added to manager", id);
                displays.insert(id, backend);
            }
        }
    }

    /// Clear all displays (for full re-enumeration)
    pub async fn clear(&self) {
        let mut displays = self.displays.write().await;
        displays.clear();
        info!("Display manager cleared");
    }

    /// Get count of managed displays
    pub async fn count(&self) -> usize {
        let displays = self.displays.read().await;
        displays.len()
    }
}

impl Clone for DisplayManager {
    fn clone(&self) -> Self {
        Self {
            displays: Arc::clone(&self.displays),
        }
    }
}

impl Default for DisplayManager {
    fn default() -> Self {
        Self::new()
    }
}
