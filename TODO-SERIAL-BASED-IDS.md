# TODO: Implement Serial Number-Based Display IDs

## Problem Statement

**Current Issue**: DDC/CI display IDs are unstable and based on I2C bus paths (e.g., `/dev/i2c-5`), which can change when:
- Monitors are unplugged/replugged
- System is rebooted
- USB-C/Thunderbolt ports are switched
- Graphics drivers are updated
- Different USB-C/DP ports are used

**Impact**:
- User profiles break (brightness settings lost)
- Per-monitor config settings lost (gamma curves, sync enable/disable, min brightness)
- Cache misses force slow DDC/CI re-initialization

## Current ID Schemes

### DDC/CI Displays (`src/protocols/ddc_ci.rs`)
- **Current**: `self.display.info.id` from `ddc-hi` library
- **Format**: I2C bus path-based (unstable)
- **Example**: `22789` (maps to `/dev/i2c-X`)

### Apple HID Displays (`src/protocols/apple_hid/device.rs`)
- **Current**: `format!("apple-hid-{}", self.serial)`
- **Format**: USB serial number-based (stable and reliable)
- **Example**: `apple-hid-00008030-001179A83E22802E`

## Proposed Solution

### Stable ID Format for DDC/CI
Use EDID serial numbers from cosmic-randr correlation:
- **New format**: `ddc-{edid_serial_hex}` (e.g., `ddc-0x00000003`)
- **Fallback**: If serial unavailable, use `ddc-{model}-{i2c_id}` with warning

### Implementation Steps

1. **Add `edid_serial` field to `MonitorInfo`** ✅ (Started)
   - File: `src/monitor/backend.rs`
   - Status: Field added, needs fixing constructors

2. **Early cosmic-randr correlation in enumeration**
   - File: `src/monitor/enumeration.rs`
   - Goal: Get EDID serials BEFORE creating DisplayBackend instances
   - Challenge: Current flow creates backends first, then correlates

3. **Modify DisplayId generation for DDC/CI**
   - File: `src/protocols/ddc_ci.rs`
   - Option A: Pass serial to constructor, store in struct
   - Option B: Change `DisplayProtocol::id()` to take serial parameter
   - Option C: Create wrapper that overrides ID after construction

4. **Config Migration**
   - File: `src/config.rs`
   - Need: Migrate old I2C-based IDs to new serial-based IDs
   - Challenge: Can't automatically map old IDs to new without user intervention
   - Possible approach: Config version field, attempt to re-match by model name

5. **Update all MonitorInfo constructors**
   - Files: `src/monitor/enumeration.rs` (lines 126, 192)
   - Files: `src/monitor/subscription.rs` (line 86)
   - Add `edid_serial: Option<String>` field to all instances

6. **Cache key updates**
   - File: `src/monitor/subscription.rs`
   - Cache already uses DisplayId as key, should work automatically

7. **Profile storage resilience**
   - File: `src/config.rs`
   - Already uses DisplayId as key in `HashMap<DisplayId, u16>`
   - Migration will be the main challenge

## Benefits

- **Reliable profile persistence** across reboots/replugs
- **Stable per-monitor settings** (gamma, sync, min brightness)
- **Better cache hits** reducing DDC/CI initialization delays
- **Multiple identical monitors** distinguished by serial number
- **Improved cosmic-randr correlation** using serial number matching tier

## Breaking Changes

### User Impact
- All existing DDC/CI monitor profiles will need to be recreated
- All per-monitor settings (gamma, sync, min brightness) will reset to defaults
- Existing saved profiles with old IDs won't apply to monitors

### Migration Strategy Options

**Option 1: Clean Break (Simpler)**
- Bump config version
- Detect old config on load
- Show notification: "Monitor configuration format updated. Please reconfigure your monitors."
- Clear old monitor-specific settings
- User recreates profiles

**Option 2: Assisted Migration (Complex)**
- Keep both old and new ID formats in config
- On startup, attempt to match old IDs to new IDs by:
  - Model name matching
  - Position matching (if only one monitor of that model)
- Prompt user for confirmation on ambiguous matches
- Gradually phase out old IDs

**Option 3: Hybrid Approach (Recommended)**
- Version config file
- For DDC monitors with serial numbers: use new stable IDs
- For DDC monitors without serials: keep old ID with warning logged
- Notify user once that some settings may be affected
- Provide UI to manually reassociate profiles if needed

## Data Sources

### EDID Serial Numbers Available
- **Source**: cosmic-randr KDL output (`cosmic-randr list --kdl`)
- **Format**: Hex string (e.g., `"0x112E647C"`)
- **Already implemented**: `src/randr.rs` lines 30-94 (KDL parsing)
- **Coverage**: All Wayland outputs with EDID data

### Current Infrastructure
- Serial number extraction: ✅ Working
- Serial number storage in OutputInfo: ✅ Working
- Serial number matching tier: ✅ Implemented
- MonitorInfo edid_serial field: 🚧 In progress

## Files to Modify

1. ✅ `src/monitor/backend.rs` - Add edid_serial to MonitorInfo
2. 🚧 `src/monitor/enumeration.rs` - Early randr correlation, pass serials
3. 🚧 `src/protocols/ddc_ci.rs` - Use serial in ID generation
4. 🚧 `src/monitor/subscription.rs` - Update MonitorInfo constructors
5. 🚧 `src/config.rs` - Add config version, migration logic
6. 🚧 `src/app/update.rs` - Handle migration notifications/UI

## Testing Requirements

- [ ] Single DDC monitor: verify stable ID across reboot
- [ ] Multiple DDC monitors: verify unique IDs when serials differ
- [ ] Multiple identical monitors: verify serial-based differentiation
- [ ] Monitor hotplug: verify ID remains stable when unplugged/replugged
- [ ] Port switching: verify ID stable when moving to different USB-C/DP port
- [ ] Profile save/load: verify profiles work across reboots
- [ ] Settings persistence: verify gamma/sync/min brightness survive reboot
- [ ] Migration: verify old configs upgrade gracefully
- [ ] No serial available: verify fallback behavior

## Known Limitations

- Monitors without EDID serial numbers will still have unstable IDs
- Migration will be disruptive for existing users (one-time reset)
- Requires cosmic-randr to be available (should be fine on COSMIC desktop)

## Alternative Approaches Considered

### 1. Use ddc-hi library EDID access directly
- **Pro**: Don't depend on cosmic-randr
- **Con**: ddc-hi may not expose EDID serial in public API
- **Con**: Would require forking/patching ddc-hi

### 2. Manual EDID I2C reads
- **Pro**: Complete control
- **Con**: Complex, requires low-level I2C protocol knowledge
- **Con**: Duplicate effort (cosmic-randr already does this)

### 3. Connector name as stable ID component
- **Pro**: More readable IDs
- **Con**: Connectors can change (DP-2 → DP-3 on monitor swap)
- **Con**: Doesn't help with the fundamental instability issue

### 4. Hybrid: Serial + Model as ID
- **Format**: `ddc-{manufacturer}-{model}-{serial}`
- **Pro**: More human-readable
- **Pro**: Better for logs/debugging
- **Con**: Longer IDs
- **Con**: Special characters in model names could cause issues

## Current Status

- [x] Serial number extraction from cosmic-randr (implemented)
- [x] Serial number matching infrastructure (implemented)
- [x] Add edid_serial to MonitorInfo struct (completed)
- [x] Fix MonitorInfo constructor call sites (completed - commit 9e427a3)
- [x] Implement early randr correlation in enumeration (completed - commit 9e427a3)
- [x] Modify DDC/CI ID generation to use serials (completed - commit 9e427a3)
- [x] Implement config migration strategy (completed - commit a458b96)
- [x] Add user notification for migration (completed - migrations.rs)
- [ ] Comprehensive testing (in progress)

## References

- cosmic-randr integration: Commit 781081a
- Serial number-based IDs: Commit 9e427a3
- Config migration: Commits a458b96, ae8453b
- Migration module: `src/migrations.rs`
- Serial matching: `src/randr.rs` lines 149-256
- Current ID generation:
  - DDC: `src/protocols/ddc_ci.rs` line 49-58
  - Apple HID: `src/protocols/apple_hid/device.rs` line 200

## Implementation Summary (December 2024)

All core features have been successfully implemented:

**Stable Display IDs**:
- DDC/CI displays now use `ddc-{serial}` format (e.g., `ddc-0x112E647C`)
- Apple HID unchanged: `apple-hid-{usb_serial}`
- Fallback to old I2C IDs with warnings if serial unavailable

**Architecture**:
- Early cosmic-randr query in enumeration (before backend creation)
- EDID serials extracted from KDL format via `cosmic-randr list --kdl`
- Serial passed to DdcCiDisplay via `set_edid_serial()` before `id()` call
- Multi-tier matching ensures correct serial assignment

**Migration**:
- Config version bumped to 2
- Dedicated `migrations.rs` module for clean separation
- Detects old numeric IDs and logs comprehensive warning
- Clean break strategy (no auto-migration)

**Testing Remaining**:
- Verify profile persistence across reboot
- Test hotplug with new stable IDs
- Confirm settings (gamma, sync, min brightness) survive reboot
- Multi-monitor scenarios with identical models
