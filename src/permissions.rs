use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PermissionCheckResult {
    pub requirements: Vec<PermissionRequirement>,
}

#[derive(Debug, Clone)]
pub struct PermissionRequirement {
    pub name: String,
    pub description: String,
    pub status: RequirementStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RequirementStatus {
    Met,
    NotMet,
    NotApplicable,
    Partial,  // Some requirements met, but not all (informational, not blocking)
}

impl PermissionCheckResult {
    pub fn has_issues(&self) -> bool {
        self.requirements.iter().any(|r| r.status == RequirementStatus::NotMet)
    }

    pub fn summary(&self) -> String {
        let not_met: Vec<_> = self.requirements
            .iter()
            .filter(|r| r.status == RequirementStatus::NotMet)
            .collect();

        if not_met.is_empty() {
            let met_count = self.requirements.iter().filter(|r| r.status == RequirementStatus::Met).count();
            format!("✓ All {} requirements met", met_count)
        } else {
            format!("{} requirement(s) not met", not_met.len())
        }
    }
}

/// Check if the current user has the necessary permissions to access I2C devices
pub fn check_i2c_permissions() -> PermissionCheckResult {
    // Debug mode: Force showing mixed permission status
    if std::env::var("DEBUG_PERMISSIONS").is_ok() {
        info!("DEBUG_PERMISSIONS set, simulating permission issues");
        return PermissionCheckResult {
            requirements: vec![
                PermissionRequirement {
                    name: "I2C devices".to_string(),
                    description: "Found 13 I2C device(s)".to_string(),
                    status: RequirementStatus::Met,
                },
                PermissionRequirement {
                    name: "I2C read/write access".to_string(),
                    description: "Can only write to 0/13 device(s)".to_string(),
                    status: RequirementStatus::NotMet,
                },
                PermissionRequirement {
                    name: "i2c group".to_string(),
                    description: "User not in i2c group".to_string(),
                    status: RequirementStatus::NotMet,
                },
                PermissionRequirement {
                    name: "udev rules (I2C)".to_string(),
                    description: "I2C udev rules not found".to_string(),
                    status: RequirementStatus::NotMet,
                },
                PermissionRequirement {
                    name: "Apple HID devices".to_string(),
                    description: "No Apple displays detected".to_string(),
                    status: RequirementStatus::NotApplicable,
                },
                PermissionRequirement {
                    name: "udev rules (Apple)".to_string(),
                    description: "N/A - no Apple displays".to_string(),
                    status: RequirementStatus::NotApplicable,
                },
            ],
        };
    }

    let mut requirements = Vec::new();

    // 1. Check for I2C devices
    let i2c_devices = find_i2c_devices();
    requirements.push(PermissionRequirement {
        name: "I2C devices".to_string(),
        description: if i2c_devices.is_empty() {
            "No /dev/i2c-* devices found".to_string()
        } else {
            format!("Found {} I2C device(s)", i2c_devices.len())
        },
        status: if i2c_devices.is_empty() {
            RequirementStatus::NotMet
        } else {
            RequirementStatus::Met
        },
    });

    // 2. Check read/write access to I2C devices (DDC/CI needs both)
    let accessible_count = i2c_devices.iter()
        .filter(|d| can_write(d))
        .count();

    requirements.push(PermissionRequirement {
        name: "I2C read/write access".to_string(),
        description: if i2c_devices.is_empty() {
            "N/A".to_string()
        } else if accessible_count == i2c_devices.len() {
            format!("Can access all {} device(s)", accessible_count)
        } else if accessible_count > 0 {
            format!("Can access {}/{} device(s)", accessible_count, i2c_devices.len())
        } else {
            format!("Cannot access any I2C devices")
        },
        status: if i2c_devices.is_empty() {
            RequirementStatus::NotApplicable
        } else if accessible_count == i2c_devices.len() {
            RequirementStatus::Met
        } else if accessible_count > 0 {
            RequirementStatus::Partial  // Some access is OK, app will work
        } else {
            RequirementStatus::NotMet  // No access at all
        },
    });

    // 3. Check if user is in i2c group
    let in_i2c_group = is_in_i2c_group();
    requirements.push(PermissionRequirement {
        name: "i2c group".to_string(),
        description: if in_i2c_group {
            "User is in i2c group".to_string()
        } else {
            "User not in i2c group".to_string()
        },
        status: if in_i2c_group {
            RequirementStatus::Met
        } else {
            RequirementStatus::NotMet
        },
    });

    // 4. Check for I2C udev rules
    let i2c_rules_exist = Path::new("/etc/udev/rules.d/45-i2c-permissions.rules").exists()
        || Path::new("/usr/lib/udev/rules.d/45-i2c-permissions.rules").exists();
    requirements.push(PermissionRequirement {
        name: "udev rules (I2C)".to_string(),
        description: if i2c_rules_exist {
            "I2C udev rules installed".to_string()
        } else {
            "I2C udev rules not found".to_string()
        },
        status: if i2c_rules_exist {
            RequirementStatus::Met
        } else {
            RequirementStatus::NotMet
        },
    });

    // 5. Check for Apple HID devices (if applicable)
    #[cfg(feature = "apple-hid-displays")]
    {
        let apple_devices = find_apple_hid_devices();
        requirements.push(PermissionRequirement {
            name: "Apple HID devices".to_string(),
            description: if apple_devices.is_empty() {
                "No Apple displays detected".to_string()
            } else {
                format!("Found {} Apple display(s)", apple_devices.len())
            },
            status: if apple_devices.is_empty() {
                RequirementStatus::NotApplicable
            } else {
                RequirementStatus::Met
            },
        });

        // 6. Check for Apple udev rules (if Apple devices exist)
        let apple_rules_exist = Path::new("/etc/udev/rules.d/99-apple-displays.rules").exists()
            || Path::new("/usr/lib/udev/rules.d/99-apple-displays.rules").exists();
        requirements.push(PermissionRequirement {
            name: "udev rules (Apple)".to_string(),
            description: if apple_devices.is_empty() {
                "N/A - no Apple displays".to_string()
            } else if apple_rules_exist {
                "Apple udev rules installed".to_string()
            } else {
                "Apple udev rules not found".to_string()
            },
            status: if apple_devices.is_empty() {
                RequirementStatus::NotApplicable
            } else if apple_rules_exist {
                RequirementStatus::Met
            } else {
                RequirementStatus::NotMet
            },
        });
    }

    #[cfg(not(feature = "apple-hid-displays"))]
    {
        requirements.push(PermissionRequirement {
            name: "Apple HID devices".to_string(),
            description: "Feature not compiled".to_string(),
            status: RequirementStatus::NotApplicable,
        });

        requirements.push(PermissionRequirement {
            name: "udev rules (Apple)".to_string(),
            description: "Feature not compiled".to_string(),
            status: RequirementStatus::NotApplicable,
        });
    }

    PermissionCheckResult { requirements }
}

/// Find all I2C device files
fn find_i2c_devices() -> Vec<PathBuf> {
    let mut devices = Vec::new();

    for i in 0..256 {
        let path = PathBuf::from(format!("/dev/i2c-{}", i));
        if path.exists() {
            devices.push(path);
        }
    }

    devices
}

/// Check if we can write to a device
fn can_write(path: &Path) -> bool {
    fs::OpenOptions::new()
        .write(true)
        .open(path)
        .is_ok()
}

/// Check if current user is in the i2c group
fn is_in_i2c_group() -> bool {
    #[cfg(unix)]
    {
        use std::process::Command;

        match Command::new("groups").output() {
            Ok(output) => {
                match String::from_utf8(output.stdout) {
                    Ok(groups_str) => {
                        debug!("Groups output: '{}'", groups_str.trim());
                        let has_i2c = groups_str.split_whitespace().any(|g| {
                            debug!("  Checking group: '{}'", g);
                            g == "i2c"
                        });
                        debug!("Has i2c group: {}", has_i2c);
                        return has_i2c;
                    }
                    Err(e) => {
                        debug!("Failed to parse groups output: {}", e);
                    }
                }
            }
            Err(e) => {
                debug!("Failed to run groups command: {}", e);
            }
        }
    }

    false
}

/// Find Apple HID devices
#[cfg(feature = "apple-hid-displays")]
fn find_apple_hid_devices() -> Vec<String> {
    use crate::protocols::apple_hid::AppleHidDisplay;
    use crate::protocols::DisplayProtocol;

    match hidapi::HidApi::new() {
        Ok(api) => {
            match AppleHidDisplay::enumerate(&api) {
                Ok(displays) => displays.iter().map(|d| d.id()).collect(),
                Err(_) => Vec::new(),
            }
        }
        Err(_) => Vec::new(),
    }
}
