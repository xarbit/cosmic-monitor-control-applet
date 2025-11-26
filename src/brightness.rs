// SPDX-License-Identifier: GPL-3.0-only
//! Brightness calculation logic
//!
//! This module provides shared brightness calculation logic used by both
//! the daemon and UI sync components to ensure consistent behavior.

use crate::config::Config;

/// Handles brightness calculations with gamma correction and minimum brightness
pub struct BrightnessCalculator<'a> {
    config: &'a Config,
}

impl<'a> BrightnessCalculator<'a> {
    /// Create a new brightness calculator with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration containing per-monitor settings
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }

    /// Calculate brightness for a specific display
    ///
    /// This method applies gamma correction and minimum brightness clamping
    /// based on per-monitor configuration.
    ///
    /// # Arguments
    ///
    /// * `cosmic_percentage` - Brightness percentage from COSMIC (0-100)
    /// * `display_id` - The unique display identifier
    ///
    /// # Returns
    ///
    /// Final brightness value (0-100) after gamma correction and min brightness clamping
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cosmic_ext_applet_external_monitor_brightness::brightness::BrightnessCalculator;
    /// use cosmic_ext_applet_external_monitor_brightness::config::Config;
    ///
    /// let config = Config::default();
    /// let calculator = BrightnessCalculator::new(&config);
    /// let brightness = calculator.calculate_for_display(50, "display-123");
    /// assert!(brightness >= 0 && brightness <= 100);
    /// ```
    pub fn calculate_for_display(&self, cosmic_percentage: u16, display_id: &str) -> u16 {
        // Convert percentage to slider value (0.0-1.0)
        let slider_value = (cosmic_percentage as f32 / 100.0).clamp(0.0, 1.0);

        // Apply gamma correction for this monitor
        let gamma = self.config.get_gamma_map(display_id);
        let mut gamma_corrected = crate::app::get_mapped_brightness(slider_value, gamma);

        // Apply minimum brightness clamp
        let min_brightness = self.config.get_min_brightness(display_id);
        if gamma_corrected < min_brightness {
            tracing::debug!(
                display_id = %display_id,
                calculated = %gamma_corrected,
                min = %min_brightness,
                "Clamping brightness to minimum"
            );
            gamma_corrected = min_brightness;
        }

        gamma_corrected
    }

    /// Check if brightness sync is enabled for a display
    ///
    /// # Arguments
    ///
    /// * `display_id` - The unique display identifier
    ///
    /// # Returns
    ///
    /// `true` if keyboard brightness sync is enabled for this display
    pub fn is_sync_enabled(&self, display_id: &str) -> bool {
        self.config.is_sync_enabled(display_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> Config {
        Config::default()
    }

    #[test]
    fn test_basic_calculation() {
        let config = create_test_config();
        let calculator = BrightnessCalculator::new(&config);

        // Test basic calculation (no gamma, no min)
        let result = calculator.calculate_for_display(50, "test-display");
        assert!(result <= 100);
    }

    #[test]
    fn test_min_brightness_clamping() {
        let mut config = create_test_config();
        // Set minimum brightness to 10%
        config.monitors.insert(
            "test-display".to_string(),
            crate::config::MonitorConfig {
                min_brightness: 10,
                gamma_map: 1.0,
                sync_with_brightness_keys: true,
            },
        );

        let calculator = BrightnessCalculator::new(&config);

        // Test that 0% gets clamped to 10%
        let result = calculator.calculate_for_display(0, "test-display");
        assert_eq!(result, 10);
    }

    #[test]
    fn test_max_brightness() {
        let config = create_test_config();
        let calculator = BrightnessCalculator::new(&config);

        // Test that 100% stays at 100%
        let result = calculator.calculate_for_display(100, "test-display");
        assert_eq!(result, 100);
    }

    #[test]
    fn test_out_of_range_input() {
        let config = create_test_config();
        let calculator = BrightnessCalculator::new(&config);

        // Test that values > 100 are handled
        let result = calculator.calculate_for_display(150, "test-display");
        assert!(result <= 100);
    }

    #[test]
    fn test_sync_enabled() {
        let mut config = create_test_config();
        config.monitors.insert(
            "enabled-display".to_string(),
            crate::config::MonitorConfig {
                min_brightness: 0,
                gamma_map: 1.0,
                sync_with_brightness_keys: true,
            },
        );
        config.monitors.insert(
            "disabled-display".to_string(),
            crate::config::MonitorConfig {
                min_brightness: 0,
                gamma_map: 1.0,
                sync_with_brightness_keys: false,
            },
        );

        let calculator = BrightnessCalculator::new(&config);

        assert!(calculator.is_sync_enabled("enabled-display"));
        assert!(!calculator.is_sync_enabled("disabled-display"));
    }
}
