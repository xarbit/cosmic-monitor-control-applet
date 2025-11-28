// Copyright 2024 Jason Scurtu
// SPDX-License-Identifier: GPL-3.0-only

//! Integration with cosmic-randr to get Wayland output information
//!
//! This module provides functionality to correlate DDC/CI and Apple HID displays
//! with COSMIC's Wayland output information (connector names, serial numbers, etc.)

use std::collections::HashMap;
use std::process::Command;
use tracing::{debug, error, info, warn};

/// Information about a Wayland output from cosmic-randr
#[derive(Debug, Clone)]
pub struct OutputInfo {
    /// Output connector name (e.g., "DP-2", "HDMI-1", "eDP-1")
    pub connector_name: String,
    /// Manufacturer name from EDID
    pub make: Option<String>,
    /// Model name from EDID
    pub model: String,
    /// Serial number (if available)
    pub serial_number: Option<String>,
    /// Whether this output is currently enabled
    pub enabled: bool,
    /// Physical size in millimeters
    pub physical_size: (u32, u32),
}

/// Parse serial numbers from cosmic-randr KDL output
/// Returns a map of connector name -> serial number
fn parse_serial_numbers_from_kdl() -> HashMap<String, String> {
    let mut serials = HashMap::new();

    // Run cosmic-randr list --kdl
    let output = match Command::new("cosmic-randr")
        .args(&["list", "--kdl"])
        .output()
    {
        Ok(out) => out,
        Err(e) => {
            warn!("Failed to run cosmic-randr list --kdl: {}", e);
            return serials;
        }
    };

    if !output.status.success() {
        warn!("cosmic-randr list --kdl failed with status: {}", output.status);
        return serials;
    }

    let kdl_str = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to parse cosmic-randr output as UTF-8: {}", e);
            return serials;
        }
    };

    // Parse KDL
    let doc = match kdl_str.parse::<kdl::KdlDocument>() {
        Ok(d) => d,
        Err(e) => {
            warn!("Failed to parse KDL document: {}", e);
            return serials;
        }
    };

    // Extract serial numbers from each output node
    for node in doc.nodes() {
        if node.name().value() == "output" {
            // Get connector name from first entry
            if let Some(connector) = node.entries().first() {
                if let Some(connector_name) = connector.value().as_string() {
                    // Look for serial_number child node
                    if let Some(children) = node.children() {
                        for child in children.nodes() {
                            if child.name().value() == "serial_number" {
                                if let Some(serial_entry) = child.entries().first() {
                                    if let Some(serial) = serial_entry.value().as_string() {
                                        debug!("Found serial for {}: {}", connector_name, serial);
                                        serials.insert(connector_name.to_string(), serial.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    serials
}

/// Fetches all Wayland output information from cosmic-randr
pub async fn get_outputs() -> Result<HashMap<String, OutputInfo>, Box<dyn std::error::Error>> {
    info!("Fetching Wayland output information from cosmic-randr");

    let list = cosmic_randr_shell::list().await.map_err(|e| {
        error!("Failed to query cosmic-randr: {}", e);
        e
    })?;

    // Parse serial numbers from KDL format
    let serial_numbers = parse_serial_numbers_from_kdl();

    let mut outputs = HashMap::new();

    for (key, output) in list.outputs.iter() {
        let serial = serial_numbers.get(&output.name).cloned();

        debug!(
            "Found Wayland output: {} (enabled: {}, model: {}, serial: {:?})",
            output.name, output.enabled, output.model, serial
        );

        let info = OutputInfo {
            connector_name: output.name.clone(),
            make: output.make.clone(),
            model: output.model.clone(),
            serial_number: serial,
            enabled: output.enabled,
            physical_size: output.physical,
        };

        outputs.insert(output.name.clone(), info);
    }

    info!("Found {} Wayland output(s) from cosmic-randr ({} with serial numbers)",
          outputs.len(), serial_numbers.len());
    Ok(outputs)
}

/// Attempts to correlate a display model name with a Wayland output
///
/// This uses fuzzy matching on the model name to find the best match
/// If serial_number is provided, it will be used as an additional matching criterion
pub fn find_matching_output(
    model_name: &str,
    outputs: &HashMap<String, OutputInfo>,
) -> Option<OutputInfo> {
    find_matching_output_with_serial(model_name, None, outputs)
}

/// Attempts to correlate a display with a Wayland output using model name and optional serial
///
/// Serial number matching is used to distinguish between multiple identical displays
pub fn find_matching_output_with_serial(
    model_name: &str,
    edid_serial: Option<&str>,
    outputs: &HashMap<String, OutputInfo>,
) -> Option<OutputInfo> {
    // Extract manufacturer and model parts from the full name
    // e.g., "Apple Inc. Studio Display" -> manufacturer: "Apple", model: "Studio Display"
    let parts: Vec<&str> = model_name.split_whitespace().collect();
    let manufacturer = if !parts.is_empty() &&
        (parts[0].eq_ignore_ascii_case("Apple") ||
         parts[0].eq_ignore_ascii_case("Dell") ||
         parts[0].eq_ignore_ascii_case("LG") ||
         parts[0].eq_ignore_ascii_case("Samsung")) {
        Some(parts[0])
    } else {
        None
    };

    let clean_model = model_name
        .split_whitespace()
        .filter(|word| {
            // Skip manufacturer-like words
            !word.eq_ignore_ascii_case("Inc.")
                && !word.eq_ignore_ascii_case("Computer")
                && !word.eq_ignore_ascii_case("Corp")
                && !word.eq_ignore_ascii_case("Ltd")
                && !word.eq_ignore_ascii_case("Apple")
                && !word.eq_ignore_ascii_case("Dell")
                && !word.eq_ignore_ascii_case("LG")
                && !word.eq_ignore_ascii_case("Samsung")
        })
        .collect::<Vec<_>>()
        .join(" ");

    // First try exact match on manufacturer, model, AND serial (if provided) - most reliable!
    if let (Some(mfr), Some(serial)) = (manufacturer, edid_serial) {
        for output in outputs.values() {
            if output.enabled {
                if let (Some(output_make), Some(output_serial)) = (&output.make, &output.serial_number) {
                    if output_make.to_lowercase().contains(&mfr.to_lowercase()) &&
                       output.model.eq_ignore_ascii_case(&clean_model) &&
                       output_serial == serial {
                        debug!("Exact make+model+serial match: {} (serial: {}) -> {}",
                               model_name, serial, output.connector_name);
                        return Some(output.clone());
                    }
                }
            }
        }
    }

    // Second try: exact match on manufacturer and model (without serial)
    if let Some(mfr) = manufacturer {
        for output in outputs.values() {
            if output.enabled {
                if let Some(ref output_make) = output.make {
                    // Check if manufacturer matches (case-insensitive, substring match for "Apple Computer Inc" vs "Apple")
                    if output_make.to_lowercase().contains(&mfr.to_lowercase()) &&
                       output.model.eq_ignore_ascii_case(&clean_model) {
                        debug!("Exact make+model match: {} -> {}", model_name, output.connector_name);
                        return Some(output.clone());
                    }
                }
            }
        }
    }

    // Third try: exact model match only (case-insensitive) - only enabled outputs
    // Also try without spaces for model names like "StudioDisplay" vs "Studio Display"
    let clean_model_no_spaces = clean_model.replace(" ", "");
    for output in outputs.values() {
        if output.enabled {
            let output_model_no_spaces = output.model.replace(" ", "");
            if output.model.eq_ignore_ascii_case(&clean_model) ||
               output_model_no_spaces.eq_ignore_ascii_case(&clean_model_no_spaces) {
                debug!("Exact model match: {} -> {}", model_name, output.connector_name);
                return Some(output.clone());
            }
        }
    }

    // Fourth try: partial match with manufacturer check
    if let Some(mfr) = manufacturer {
        for output in outputs.values() {
            if output.enabled {
                if let Some(ref output_make) = output.make {
                    if output_make.to_lowercase().contains(&mfr.to_lowercase()) &&
                       output.model.to_lowercase().contains(&clean_model.to_lowercase()) {
                        debug!("Partial make+model match: {} -> {}", model_name, output.connector_name);
                        return Some(output.clone());
                    }
                }
            }
        }
    }

    // Last resort: try partial model-only match (output model contains our model)
    // This prevents "StudioDisplay" from matching "Display"
    for output in outputs.values() {
        if output.enabled && output.model.to_lowercase().contains(&clean_model.to_lowercase()) {
            debug!("Partial model match (output contains input): {} -> {}", model_name, output.connector_name);
            return Some(output.clone());
        }
    }

    warn!("No matching output found for model: {}", model_name);
    None
}

/// Attempts to find a Wayland output by manufacturer and model
pub fn find_output_by_make_model(
    make: Option<&str>,
    model: &str,
    outputs: &HashMap<String, OutputInfo>,
) -> Option<OutputInfo> {
    // Try exact match on both make and model
    if let Some(manufacturer) = make {
        for output in outputs.values() {
            if let Some(ref output_make) = output.make {
                if output_make.eq_ignore_ascii_case(manufacturer)
                    && output.model.eq_ignore_ascii_case(model)
                {
                    debug!(
                        "Exact make+model match: {}/{} -> {}",
                        manufacturer, model, output.connector_name
                    );
                    return Some(output.clone());
                }
            }
        }
    }

    // Fall back to model-only matching
    find_matching_output(model, outputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_outputs() -> HashMap<String, OutputInfo> {
        let mut outputs = HashMap::new();

        outputs.insert(
            "DP-2".to_string(),
            OutputInfo {
                connector_name: "DP-2".to_string(),
                make: Some("Apple Computer Inc".to_string()),
                model: "StudioDisplay".to_string(),
                serial_number: Some("0x112E647C".to_string()),
                enabled: false,
                physical_size: (600, 330),
            },
        );

        outputs.insert(
            "DP-3".to_string(),
            OutputInfo {
                connector_name: "DP-3".to_string(),
                make: Some("Apple Computer Inc".to_string()),
                model: "StudioDisplay".to_string(),
                serial_number: Some("0x112E647D".to_string()),
                enabled: true,
                physical_size: (600, 330),
            },
        );

        outputs
    }

    #[test]
    fn test_exact_model_match() {
        let outputs = create_test_outputs();
        let result = find_matching_output("StudioDisplay", &outputs);
        assert!(result.is_some());
    }

    #[test]
    fn test_case_insensitive_match() {
        let outputs = create_test_outputs();
        let result = find_matching_output("studiodisplay", &outputs);
        assert!(result.is_some());
    }

    #[test]
    fn test_partial_match() {
        let outputs = create_test_outputs();
        let result = find_matching_output("Studio", &outputs);
        assert!(result.is_some());
    }

    #[test]
    fn test_make_model_match() {
        let outputs = create_test_outputs();
        let result = find_output_by_make_model(
            Some("Apple Computer Inc"),
            "StudioDisplay",
            &outputs,
        );
        assert!(result.is_some());
    }
}
