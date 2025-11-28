// Copyright 2024 Jason Scurtu
// SPDX-License-Identifier: GPL-3.0-only

//! Integration with cosmic-randr to get Wayland output information
//!
//! This module provides functionality to correlate DDC/CI and Apple HID displays
//! with COSMIC's Wayland output information (connector names, serial numbers, etc.)

use std::collections::HashMap;
use std::process::Command;
use tracing::{debug, error, info, warn};

/// Display mode information (resolution and refresh rate)
#[derive(Debug, Clone)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    /// Refresh rate in millihertz (60000 = 60Hz)
    pub refresh_rate: u32,
}

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
    /// Display position (x, y) in virtual desktop
    pub position: (i32, i32),
    /// HiDPI scale factor (1.0, 1.5, 2.0, etc.)
    pub scale: f32,
    /// Transform/rotation (normal, 90, 180, 270, flipped, etc.)
    pub transform: String,
    /// Current display mode (resolution and refresh rate)
    pub current_mode: Option<DisplayMode>,
}

/// Additional output information parsed from KDL
#[derive(Debug, Clone, Default)]
struct KdlOutputInfo {
    serial_number: Option<String>,
    position: Option<(i32, i32)>,
    scale: Option<f32>,
    transform: Option<String>,
    current_mode: Option<DisplayMode>,
}

/// Parse additional output information from cosmic-randr KDL output
/// Returns a map of connector name -> KdlOutputInfo
fn parse_kdl_output_info() -> HashMap<String, KdlOutputInfo> {
    let mut outputs = HashMap::new();

    // Run cosmic-randr list --kdl
    let output = match Command::new("cosmic-randr")
        .args(&["list", "--kdl"])
        .output()
    {
        Ok(out) => out,
        Err(e) => {
            warn!("Failed to run cosmic-randr list --kdl: {}", e);
            return outputs;
        }
    };

    if !output.status.success() {
        warn!("cosmic-randr list --kdl failed with status: {}", output.status);
        return outputs;
    }

    let kdl_str = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to parse cosmic-randr output as UTF-8: {}", e);
            return outputs;
        }
    };

    // Parse KDL
    let doc = match kdl_str.parse::<kdl::KdlDocument>() {
        Ok(d) => d,
        Err(e) => {
            warn!("Failed to parse KDL document: {}", e);
            return outputs;
        }
    };

    // Extract information from each output node
    for node in doc.nodes() {
        if node.name().value() == "output" {
            // Get connector name from first entry
            if let Some(connector) = node.entries().first() {
                if let Some(connector_name) = connector.value().as_string() {
                    let mut info = KdlOutputInfo::default();

                    // Look for child nodes with display info
                    if let Some(children) = node.children() {
                        for child in children.nodes() {
                            match child.name().value() {
                                "serial_number" => {
                                    if let Some(serial_entry) = child.entries().first() {
                                        if let Some(serial) = serial_entry.value().as_string() {
                                            info.serial_number = Some(serial.to_string());
                                        }
                                    }
                                }
                                "position" => {
                                    // position x y
                                    if let (Some(x_entry), Some(y_entry)) = (child.entries().get(0), child.entries().get(1)) {
                                        // Try to parse as integer
                                        let x = x_entry.value().as_integer().map(|i| i as i32);
                                        let y = y_entry.value().as_integer().map(|i| i as i32);
                                        if let (Some(x), Some(y)) = (x, y) {
                                            info.position = Some((x, y));
                                        }
                                    }
                                }
                                "scale" => {
                                    // scale 2.00
                                    if let Some(scale_entry) = child.entries().first() {
                                        // Try as float first, then as integer
                                        let scale = if let Some(f) = scale_entry.value().as_float() {
                                            Some(f as f32)
                                        } else if let Some(i) = scale_entry.value().as_integer() {
                                            Some(i as f32)
                                        } else {
                                            None
                                        };
                                        if let Some(scale) = scale {
                                            info.scale = Some(scale);
                                        }
                                    }
                                }
                                "transform" => {
                                    // transform "normal"
                                    if let Some(transform_entry) = child.entries().first() {
                                        if let Some(transform) = transform_entry.value().as_string() {
                                            info.transform = Some(transform.to_string());
                                        }
                                    }
                                }
                                "modes" => {
                                    // Find the current mode
                                    if let Some(mode_children) = child.children() {
                                        for mode_node in mode_children.nodes() {
                                            if mode_node.name().value() == "mode" {
                                                // Check if this is the current mode
                                                let is_current = mode_node.entries().iter()
                                                    .any(|e| e.name().map_or(false, |n| n.value() == "current")
                                                              && e.value().as_bool() == Some(true));

                                                if is_current {
                                                    // mode width height refresh_rate current=#true
                                                    if let (Some(w), Some(h), Some(r)) = (
                                                        mode_node.entries().get(0).and_then(|e| e.value().as_integer()),
                                                        mode_node.entries().get(1).and_then(|e| e.value().as_integer()),
                                                        mode_node.entries().get(2).and_then(|e| e.value().as_integer()),
                                                    ) {
                                                        info.current_mode = Some(DisplayMode {
                                                            width: w as u32,
                                                            height: h as u32,
                                                            refresh_rate: r as u32,
                                                        });
                                                    }
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    debug!("Parsed KDL info for {}: serial={:?}, pos={:?}, scale={:?}, transform={:?}, mode={:?}",
                           connector_name, info.serial_number, info.position, info.scale, info.transform, info.current_mode);
                    outputs.insert(connector_name.to_string(), info);
                }
            }
        }
    }

    outputs
}

/// Fetches all Wayland output information from cosmic-randr
pub async fn get_outputs() -> Result<HashMap<String, OutputInfo>, Box<dyn std::error::Error>> {
    info!("Fetching Wayland output information from cosmic-randr");

    let list = cosmic_randr_shell::list().await.map_err(|e| {
        error!("Failed to query cosmic-randr: {}", e);
        e
    })?;

    // Parse additional output information from KDL format
    let kdl_info = parse_kdl_output_info();

    let mut outputs = HashMap::new();

    for (_key, output) in list.outputs.iter() {
        // Get KDL-parsed info for this output (if available)
        let kdl = kdl_info.get(&output.name);

        debug!(
            "Found Wayland output: {} (enabled: {}, model: {}, serial: {:?})",
            output.name, output.enabled, output.model, kdl.and_then(|k| k.serial_number.as_ref())
        );

        let info = OutputInfo {
            connector_name: output.name.clone(),
            make: output.make.clone(),
            model: output.model.clone(),
            serial_number: kdl.and_then(|k| k.serial_number.clone()),
            enabled: output.enabled,
            physical_size: output.physical,
            position: kdl.and_then(|k| k.position).unwrap_or((0, 0)),
            scale: kdl.and_then(|k| k.scale).unwrap_or(1.0),
            transform: kdl.and_then(|k| k.transform.clone()).unwrap_or_else(|| "normal".to_string()),
            current_mode: kdl.and_then(|k| k.current_mode.clone()),
        };

        outputs.insert(output.name.clone(), info);
    }

    let serial_count = outputs.values().filter(|o| o.serial_number.is_some()).count();
    info!("Found {} Wayland output(s) from cosmic-randr ({} with serial numbers)",
          outputs.len(), serial_count);
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

/// Helper to map our rotation format to cosmic-randr transform format
fn map_transform_to_randr(transform: &str) -> &str {
    match transform {
        "normal" => "normal",
        "90" => "rotate90",
        "180" => "rotate180",
        "270" => "rotate270",
        "flipped" => "flipped",
        "flipped-90" => "flipped90",
        "flipped-180" => "flipped180",
        "flipped-270" => "flipped270",
        _ => {
            warn!("Unknown transform '{}', defaulting to 'normal'", transform);
            "normal"
        }
    }
}

/// Apply display scale via cosmic-randr
pub async fn apply_scale(connector_name: &str, current_mode: &DisplayMode, scale: f32) -> anyhow::Result<()> {
    info!("Applying scale {} to {}", scale, connector_name);

    let output = tokio::process::Command::new("cosmic-randr")
        .args([
            "mode",
            connector_name,
            &current_mode.width.to_string(),
            &current_mode.height.to_string(),
            "--scale",
            &scale.to_string(),
        ])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to apply scale: {}", stderr);
    }

    info!("Successfully applied scale {} to {}", scale, connector_name);
    Ok(())
}

/// Apply display transform/rotation via cosmic-randr
pub async fn apply_transform(connector_name: &str, current_mode: &DisplayMode, transform: &str) -> anyhow::Result<()> {
    let randr_transform = map_transform_to_randr(transform);
    info!("Applying transform {} (cosmic-randr: {}) to {}", transform, randr_transform, connector_name);

    let output = tokio::process::Command::new("cosmic-randr")
        .args([
            "mode",
            connector_name,
            &current_mode.width.to_string(),
            &current_mode.height.to_string(),
            "--transform",
            randr_transform,
        ])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to apply transform: {}", stderr);
    }

    info!("Successfully applied transform {} to {}", transform, connector_name);
    Ok(())
}

/// Apply display position via cosmic-randr
pub async fn apply_position(connector_name: &str, x: i32, y: i32) -> anyhow::Result<()> {
    info!("Applying position ({}, {}) to {}", x, y, connector_name);

    let output = tokio::process::Command::new("cosmic-randr")
        .args([
            "position",
            connector_name,
            &x.to_string(),
            &y.to_string(),
        ])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to apply position: {}", stderr);
    }

    info!("Successfully applied position ({}, {}) to {}", x, y, connector_name);
    Ok(())
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
                position: (0, 0),
                scale: 2.0,
                transform: "normal".to_string(),
                current_mode: Some(DisplayMode { width: 5120, height: 2880, refresh_rate: 60000 }),
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
                position: (1280, 0),
                scale: 2.0,
                transform: "normal".to_string(),
                current_mode: Some(DisplayMode { width: 5120, height: 2880, refresh_rate: 60000 }),
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
