# External Monitor Control Applet for the COSMIC™ desktop

Control external monitor brightness via DDC/CI and Apple HID protocols. Native support for Apple displays (Studio Display, Pro Display XDR) and LG UltraFine displays via USB HID. Includes automatic brightness synchronization with COSMIC keyboard brightness keys, brightness profiles, and dark mode toggle.

> **Note**: This is a fork of [cosmic-ext-applet-external-monitor-brightness](https://github.com/cosmic-utils/cosmic-ext-applet-external-monitor-brightness) from the [COSMIC Utils](https://github.com/cosmic-utils) project, originally created by [@maciekk64](https://github.com/maciekk64), with significant enhancements and new features.

## Screenshots

<p align="center">
  <img src="res/screenshot1.png" width="600" alt="Main applet view with brightness controls">
  <br>
  <em>Main applet view with brightness controls and profiles</em>
</p>

<p align="center">
  <img src="res/screenshot2.png" width="600" alt="Brightness profiles">
  <br>
  <em>Brightness profiles for saving and restoring settings</em>
</p>

<p align="center">
  <img src="res/screenshot3.png" width="600" alt="Permission management view">
  <br>
  <em>Permission management view for I2C and HID access</em>
</p>

## Key Enhancements Over Original

This fork adds several major features and improvements:

### New Features
- **Apple HID Display Support**: Native USB HID protocol support for Apple displays (Studio Display, Pro Display XDR) and LG UltraFine 4K/5K
- **Brightness Profiles**: Save and restore brightness settings across multiple monitors with named profiles
- **Keyboard Brightness Key Synchronization**: Background daemon that automatically syncs COSMIC keyboard brightness keys to external monitors
- **Automatic Hotplug Detection**: Monitors are automatically detected and added/removed when connected/disconnected
- **Empty State UI**: Helpful guidance when no displays are detected

### Technical Improvements
- **Protocol-Based Architecture**: Modular design supporting multiple display protocols simultaneously
- **Async/Await Throughout**: Non-blocking UI with responsive controls
- **Better Error Handling**: Comprehensive permission checking and user-friendly error messages
- **XDG Portal Support**: URLs open via portals for Flatpak compatibility

## Features

- **DDC/CI Support**: Control brightness for standard external monitors using the DDC/CI protocol
  - Fast concurrent enumeration for quick startup
  - Per-monitor brightness control with gamma curve adjustment (0.3-3.0 range)
  - Minimum brightness settings to prevent displays from going too dim
- **Apple HID Display Support**: Native USB HID support for Apple displays
  - Supported displays: Studio Display, Pro Display XDR, LG UltraFine 4K/5K
  - Device-specific default gamma curves (1.8 for Apple displays, optimized for their native brightness response)
  - Direct brightness control via applet slider
  - Monitor name labels for easy identification
- **Keyboard Brightness Key Sync**: Automatic brightness synchronization with COSMIC keyboard brightness keys
  - Works with both DDC/CI and Apple HID displays
  - Per-monitor toggle to enable/disable sync
  - Configurable sync mode (all displays or primary only)
  - Lightweight background daemon
- **Enhanced UI**:
  - Icons in settings menu for better visual organization
  - Precise gamma control with +/- buttons (0.1 increments)
  - Clear display of current values
  - Monitor name labels for multi-monitor setups
- **Dark Mode Toggle**: Quickly toggle system dark mode
- **Async Architecture**: Non-blocking UI with responsive controls
- **Protocol-Based Architecture**: Modular design supporting multiple display protocols simultaneously

## Installation

### Dependencies

#### Build Dependencies
- Rust toolchain (cargo)
- System libraries:
  - `i2c-dev` headers (for DDC/CI support)
  - `hidapi` development files (for Apple HID display support)
  - `libudev` development files (for device detection)

#### Runtime Dependencies
- **For DDC/CI displays** (standard external monitors):
  - `i2c-tools` or `ddcutil`
  - User must be in the `i2c` group
- **For Apple HID displays** (Studio Display, Pro Display XDR, LG UltraFine):
  - `hidapi` library
  - Proper udev rules installed (see Troubleshooting section)

#### Package names by distribution:

**Fedora/RHEL:**
```bash
# Build dependencies
sudo dnf install rust cargo i2c-tools-devel hidapi-devel systemd-devel

# Runtime dependencies
sudo dnf install i2c-tools hidapi ddcutil
```

**Debian/Ubuntu:**
```bash
# Build dependencies
sudo apt install cargo libi2c-dev libhidapi-dev libudev-dev

# Runtime dependencies
sudo apt install i2c-tools libhidapi-libusb0 ddcutil
```

**Arch Linux:**
```bash
# Build dependencies
sudo pacman -S rust i2c-tools hidapi systemd

# Runtime dependencies (same as build)
sudo pacman -S i2c-tools hidapi ddcutil
```

### Building from Source

```bash
# Build with default features (includes Apple HID display support)
cargo build --release

# Build without Apple HID display support (DDC/CI only)
cargo build --release --no-default-features
```

### Feature Flags

- `apple-hid-displays` (default): Enables USB HID support for Apple displays (Studio Display, Pro Display XDR) and LG UltraFine displays

## Troubleshooting

### DDC/CI Displays

You may need to set up udev rules for I2C access if you encounter permission errors.
For this to work you need write access to `/dev/i2c-*`.

```bash
# Copy the I2C permissions udev rules
sudo cp data/udev/45-i2c-permissions.rules /etc/udev/rules.d/

# Reload udev rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

Alternatively, see [https://www.ddcutil.com/i2c_permissions/](https://www.ddcutil.com/i2c_permissions/) for more information.

### Apple HID Displays (Studio Display, Pro Display XDR, LG UltraFine)

On Linux, you need to set up udev rules to allow non-root access to the display's USB HID interface:

```bash
# Copy the Apple HID udev rules
sudo cp data/udev/99-apple-displays.rules /etc/udev/rules.d/

# Reload udev rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

After installation, you may need to unplug and replug the display, or reboot your system for the changes to take effect.

If you see permission errors in the logs, ensure the udev rules are properly installed.

### Keyboard Brightness Key Synchronization

The brightness sync daemon automatically syncs COSMIC keyboard brightness keys to external monitors. Features:
- Works with both DDC/CI and Apple HID displays
- Per-monitor toggle to enable/disable sync (right-click on monitor icon in settings)
- Syncs on startup and when brightness keys are pressed
- Runs in the background as a lightweight daemon

You can disable sync for specific monitors by:
1. Right-click the monitor icon to open settings
2. Toggle the "Sync with Keyboard brightness keys" switch

You can check if the daemon is running with:
```bash
RUST_LOG=info cosmic-monitor-control-applet 2>&1 | grep daemon
```

## Credits

**Maintained by**: [@xarbit](https://github.com/xarbit) (Jason Scurtu)

**Based on**: [cosmic-ext-applet-external-monitor-brightness](https://github.com/cosmic-utils/cosmic-ext-applet-external-monitor-brightness) from [COSMIC Utils](https://github.com/cosmic-utils)
- Originally created by [@maciekk64](https://github.com/maciekk64)
- Contributors: [@wiiznokes](https://github.com/wiiznokes), [@BrunoWallner](https://github.com/BrunoWallner), [@therealmate](https://github.com/therealmate), [@bittin](https://github.com/bittin), [@Gr3q](https://github.com/Gr3q), [@gCattt](https://github.com/gCattt), [@feikedonia](https://github.com/feikedonia)
- Licensed under GPL-3.0

**Apple HID Protocol**: Implementation based on [asdbctl](https://github.com/juliuszint/asdbctl) by [@juliuszint](https://github.com/juliuszint) (MIT License)
