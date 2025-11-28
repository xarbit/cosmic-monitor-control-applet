# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A COSMIC desktop applet for controlling external monitor brightness via DDC/CI and Apple HID protocols. Supports standard DDC/CI monitors and native Apple displays (Studio Display, Pro Display XDR) and LG UltraFine displays. Includes automatic brightness synchronization with COSMIC keyboard brightness keys, brightness profiles, and dark mode toggle.

## Build Commands

```bash
# Build release version (default)
just build-release
# or
cargo build --release

# Build debug version
just build-debug
# or
cargo build

# Build without Apple HID support (DDC/CI only)
cargo build --release --no-default-features

# Run tests
just test
# or
cargo test --workspace --all-features

# Format and lint
just fmt        # Format Rust code
just fix        # Auto-fix clippy issues
just pull       # Full pre-PR workflow: fmt, prettier, fix, test, fmt-just
```

## Feature Flags

- `apple-hid-displays` (default): Enables USB HID support for Apple/LG displays
- `brightness-sync-daemon` (default): Enables background daemon for keyboard brightness key sync

Both features are enabled by default. Disable with `--no-default-features`.

## Architecture

### Protocol-Based Design

The codebase uses a protocol-based architecture in `src/protocols/` with a common `DisplayProtocol` trait:

- **DDC/CI** (`protocols/ddc_ci.rs`): Standard I2C-based monitor control
- **Apple HID** (`protocols/apple_hid/`): USB HID protocol for Apple displays

Each protocol implements `DisplayProtocol` with `get_brightness()` and `set_brightness()` methods.

### Display Manager Singleton

`monitor::DisplayManager` (in `src/monitor/manager.rs`) provides centralized display management:

- Global singleton using `Arc<RwLock<HashMap>>` to ensure only one I2C connection per physical monitor
- Prevents DDC/CI protocol timing violations from concurrent access
- Shared between UI subscription and brightness sync daemon
- Multiple applet instances (multi-panel setups) share the same backend

### Display Enumeration

`monitor::enumeration` handles async display detection:

- **DDC/CI**: Concurrent enumeration of `/dev/i2c-*` devices using `ddc-hi` library
- **Apple HID**: USB device enumeration via `hidapi` with device-specific implementations in `src/devices/`
- **Wayland Integration**: Correlates displays with COSMIC outputs via `cosmic-randr` for connector names
- Runs asynchronously to prevent UI blocking during startup

### Wayland Output Integration

`src/randr.rs` integrates with `cosmic-randr-shell` to provide enhanced display information:

- Queries Wayland outputs during enumeration to get connector names (DP-1, HDMI-2, USB-C, etc.)
- Intelligent model name matching that strips manufacturer prefixes for reliable correlation
- Displays connector names in UI: "Apple Studio Display (DP-3)"
- Gracefully degrades if cosmic-randr is unavailable or fails
- Only matches enabled outputs to avoid showing disabled displays

### Brightness Sync Daemon

When `brightness-sync-daemon` feature is enabled, `src/daemon.rs` implements a background daemon:

- Listens to COSMIC Settings Daemon via D-Bus (`com.system76.CosmicSettingsDaemon`)
- Applies brightness changes to external monitors when keyboard brightness keys are pressed
- Per-monitor sync enable/disable via config (`sync_with_brightness_keys`)
- Debounces rapid changes (50ms) to prevent overwhelming DDC/CI displays
- Uses parallel brightness application with retry logic (DDC/CI requires 40ms between commands)

### Configuration System

`src/config.rs` uses COSMIC config system (`cosmic-config`):

- Per-monitor settings: gamma curves, sync enable/disable, minimum brightness
- Brightness profiles: save/restore brightness across multiple monitors
- Config persisted in XDG config directory
- Default gamma: 1.8 for Apple HID displays, 1.0 for DDC/CI

### UI Architecture

Built with `libcosmic` applet framework:

- `src/app/`: Core application state and message handling
  - `state.rs`: Application state with monitor list and popup state
  - `messages.rs`: Message types for UI events
  - `update.rs`: Message handlers and state updates
  - `popup.rs`: Popup window management
- `src/view/`: UI components (monitor items, profiles, settings, etc.)
- Uses Iced subscription model for async updates
- Multiple subscriptions: monitor enumeration, hotplug detection, config changes, UI sync

### Hotplug Detection

`src/hotplug/` implements automatic monitor detection:

- Uses `udev` for Linux device monitoring
- Subscription-based: notifies UI when displays are added/removed
- Monitors both I2C and USB HID devices

### Device-Specific Implementations

`src/devices/` contains device-specific HID implementations:

- `apple/studio_display.rs`: Apple Studio Display (VID: 0x05ac, PID: 0x1114)
- `apple/pro_display_xdr.rs`: Pro Display XDR (VID: 0x05ac, PID: 0x1112)
- `lg/ultrafine_4k.rs`: LG UltraFine 4K (VID: 0x043e, PID: 0x9a40)
- `lg/ultrafine_5k.rs`: LG UltraFine 5K (VID: 0x043e, PID: 0x9a39)

Each defines USB identifiers and display-specific configuration.

## Important Implementation Details

### DDC/CI Timing

The DDC/CI protocol requires 40ms minimum between commands. The codebase implements:

- 50ms delay before retry on failure
- 200ms delay after brightness changes before reading to prevent errors
- Parallel brightness application with spawn_blocking to avoid blocking async runtime

### Brightness Calculations

`src/brightness.rs` contains `BrightnessCalculator` for gamma curve application:

- Gamma correction: `output = input^gamma` (gamma range: 0.3-3.0)
- Minimum brightness enforcement to prevent displays going too dim
- Consistent calculations shared between UI and daemon

### Permission Handling

`src/permissions.rs` checks for required permissions:

- **DDC/CI**: Read/write access to `/dev/i2c-*` devices
- **Apple HID**: Access to USB HID devices via udev rules
- UI displays helpful permission warnings with links to troubleshooting

### Async Patterns

- Use `tokio::spawn_blocking` for blocking I/O (DDC/CI, HID)
- `Arc<tokio::sync::Mutex>` for display backends (supports both async and blocking contexts)
- Subscriptions for long-running async tasks (monitor enumeration, daemon)
- `blocking_lock()` when accessing `tokio::Mutex` from `spawn_blocking` context

## Testing

The codebase has limited unit tests. When adding tests:

- DDC/CI operations require physical hardware, so most testing is integration/manual
- Mock `DisplayProtocol` implementations for unit testing higher-level logic
- Test brightness calculations in `brightness.rs` as these are pure functions

## Logging

Uses `tracing` with journald integration:

- Log level controlled via `RUST_LOG` environment variable
- Default: `error` globally, `info` for this crate
- Noisy DDC/CI errors from `ddc_hi` library are filtered at error level
- Bridge from `log` crate to `tracing` for library compatibility

## Dependencies

Key runtime dependencies:

- **libcosmic**: COSMIC desktop UI framework (from git)
- **cosmic-randr-shell**: Wayland output information for connector names (from git)
- **ddc-hi**: DDC/CI protocol implementation
- **hidapi**: USB HID access (when `apple-hid-displays` enabled)
- **zbus**: D-Bus communication for brightness sync daemon
- **udev**: Device hotplug detection
- **tokio**: Async runtime
