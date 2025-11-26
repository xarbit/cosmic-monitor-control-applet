// SPDX-License-Identifier: GPL-3.0-only
//! Centralized display manager
//!
//! This module provides a singleton display manager that manages all external
//! displays. Both the UI and daemon access displays through this manager,
//! ensuring only one I2C connection per physical monitor.
//!
//! # Architecture
//!
//! The DisplayManager maintains a singleton HashMap of display backends that is
//! shared between the UI subscription and the brightness sync daemon. This ensures
//! only one I2C connection exists per physical monitor, preventing DDC/CI protocol
//! timing violations that occur when multiple processes attempt to access the same
//! display simultaneously.
//!
//! # Thread Safety
//!
//! Uses `Arc<RwLock<HashMap>>` for concurrent access:
//! - Read operations (display enumeration, getting displays) use read locks
//! - Write operations (adding/removing displays) use write locks
//! - The DisplayManager itself uses `Arc` for cheap cloning across async contexts
//!
//! # Usage
//!
//! ```no_run
//! use cosmic_ext_applet_external_monitor_brightness::monitor::DisplayManager;
//!
//! # async fn example() {
//! let manager = DisplayManager::new();
//!
//! // Get a display
//! if let Some(display) = manager.get("display-123").await {
//!     let mut guard = display.lock().await;
//!     // Use display
//! }
//!
//! // Get all display IDs
//! let ids = manager.get_all_ids().await;
//! # }
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;

use super::backend::{DisplayBackend, DisplayId};

/// Global singleton instance of the display manager
///
/// This ensures that all applet instances (even across multiple panels) share
/// the same display backends, preventing I2C conflicts when multiple applet
/// instances try to access the same displays simultaneously.
static GLOBAL_DISPLAY_MANAGER: Lazy<Arc<RwLock<HashMap<DisplayId, Arc<tokio::sync::Mutex<DisplayBackend>>>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Shared display manager instance
///
/// This manages all display backends and ensures only one I2C connection
/// per physical monitor. Both UI and daemon access displays through this.
///
/// All instances of DisplayManager share the same underlying storage via
/// a global singleton, preventing conflicts when multiple applet instances
/// exist (e.g., one per panel in a multi-monitor setup).
pub struct DisplayManager {
    displays: Arc<RwLock<HashMap<DisplayId, Arc<tokio::sync::Mutex<DisplayBackend>>>>>,
}

impl DisplayManager {
    /// Create a new display manager reference
    ///
    /// All instances share the same global singleton storage, ensuring
    /// that multiple applet instances don't conflict with each other.
    pub fn new() -> Self {
        Self {
            displays: GLOBAL_DISPLAY_MANAGER.clone(),
        }
    }

    /// Get a reference to a display backend by ID
    ///
    /// # Arguments
    ///
    /// * `id` - The unique display identifier
    ///
    /// # Returns
    ///
    /// * `Some(Arc<Mutex<DisplayBackend>>)` if display exists
    /// * `None` if display not found
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example(manager: cosmic_ext_applet_external_monitor_brightness::monitor::DisplayManager) {
    /// if let Some(display) = manager.get("display-123").await {
    ///     let mut guard = display.lock().await;
    ///     let brightness = guard.get_brightness().unwrap();
    /// }
    /// # }
    /// ```
    pub async fn get(&self, id: &str) -> Option<Arc<tokio::sync::Mutex<DisplayBackend>>> {
        let displays = self.displays.read().await;
        displays.get(id).cloned()
    }

    /// Get all display IDs currently managed
    ///
    /// # Returns
    ///
    /// Vector of display IDs
    pub async fn get_all_ids(&self) -> Vec<DisplayId> {
        let displays = self.displays.read().await;
        displays.keys().cloned().collect()
    }

    /// Update the display map with newly enumerated displays
    ///
    /// This intelligently merges new displays with existing ones:
    /// - Keeps existing display connections alive (no re-initialization)
    /// - Adds newly discovered displays
    /// - Removes displays that are no longer present
    ///
    /// # Arguments
    ///
    /// * `new_displays` - HashMap of newly enumerated displays
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
    ///
    /// This removes all displays from the manager, forcing a complete
    /// re-initialization on the next enumeration. Useful for debugging
    /// or handling major system changes.
    ///
    /// # Note
    ///
    /// Currently unused but kept as part of the public API for future use
    /// cases such as manual refresh or recovery scenarios.
    #[allow(dead_code)]
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
