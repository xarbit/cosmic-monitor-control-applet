# Simple RPM spec for local builds (without cargo vendor)
Name:           cosmic-monitor-control-applet
Version:        0.2.1
Release:        1%{?dist}
Summary:        External Monitor Control Applet for COSMIC Desktop

License:        GPL-3.0-only
URL:            https://github.com/xarbit/cosmic-monitor-control-applet
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.70
BuildRequires:  cargo
BuildRequires:  systemd-rpm-macros
BuildRequires:  git
# just is installed via cargo, not as RPM
# BuildRequires:  just

# Runtime dependencies
Requires:       hidapi
Requires:       ddcutil

%description
External Monitor Control Applet for the COSMIC™ desktop environment.
Control external monitors using:
- DDC/CI protocol for standard external monitors via I2C
- Native USB HID support for Apple Studio Display and LG UltraFine displays
- Brightness profiles for saving/restoring settings across multiple monitors
- Automatic brightness sync with COSMIC keyboard brightness keys
- Monitor name labels in UI for easy identification
- Quick toggle for system dark mode

%prep
%autosetup -n %{name}-%{version}

%build
just build-release

%install
# Install binary and desktop files
just --set rootdir %{buildroot} --set prefix %{_prefix} install

# Install udev rules for Apple displays and I2C permissions
install -Dm0644 data/udev/99-apple-displays.rules \
    %{buildroot}%{_udevrulesdir}/99-apple-displays.rules
install -Dm0644 data/udev/45-i2c-permissions.rules \
    %{buildroot}%{_udevrulesdir}/45-i2c-permissions.rules

%pre
# Create i2c group if it doesn't exist (for DDC/CI displays)
getent group i2c >/dev/null || groupadd -r i2c

%post
%udev_rules_update

# Display post-install message
cat <<'EOF'

╔═══════════════════════════════════════════════════════════════════════════╗
║  External Monitor Control Applet - Post-Installation Setup               ║
╚═══════════════════════════════════════════════════════════════════════════╝

For DDC/CI displays (standard external monitors):
  Add your user to the i2c group:
    sudo usermod -aG i2c $USER

For Apple HID displays (Studio Display, Pro Display XDR, LG UltraFine):
  Permissions are handled automatically via udev rules.

After adding yourself to the i2c group, log out and log back in for changes
to take effect.

The COSMIC panel will restart to load the applet.

EOF

# Restart COSMIC panel to recognize the new applet
if [ $1 -eq 1 ] ; then
    # Only on install (not upgrade)
    killall cosmic-panel 2>/dev/null || true
fi

%postun
%udev_rules_update
# Restart COSMIC panel after uninstall
if [ $1 -eq 0 ] ; then
    # Only on uninstall (not upgrade)
    killall cosmic-panel 2>/dev/null || true
fi

%files
%license LICENSE
%doc README.md
%{_bindir}/%{name}
%{_datadir}/applications/io.github.xarbit.CosmicMonitorControlApplet.desktop
%{_datadir}/icons/hicolor/scalable/apps/io.github.xarbit.CosmicMonitorControlApplet-symbolic.svg
%{_datadir}/metainfo/io.github.xarbit.CosmicMonitorControlApplet.metainfo.xml
%{_udevrulesdir}/99-apple-displays.rules
%{_udevrulesdir}/45-i2c-permissions.rules

%changelog
* Tue Nov 26 2024 Jason Scurtu <github@mail.scurtu.me> - 0.2.1-1
- Rename project to cosmic-monitor-control-applet
- Add brightness profiles feature with collapsible UI
- Add comprehensive about page with clickable links and XDG portal support
- Update credits to properly attribute COSMIC Utils and all contributors
- Replace F1/F2 references with keyboard brightness keys for accuracy

* Tue Nov 26 2024 Jason Scurtu <github@mail.scurtu.me> - 0.2.0-1
- Fix dual-applet crash when clicking on multiple panels simultaneously (#36)
- Implement global singleton DisplayManager to prevent I2C conflicts
- Update all dependencies to latest versions (libcosmic, tokio, zbus, tracing)
- Refactor UI spacing to use COSMIC theme constants
- Add libudev-dev to build documentation (#15)
- Improve portable monitor diagnostics (#34)
- Add once_cell dependency for singleton pattern
- All spacing now adapts to COSMIC design system

* Tue Nov 26 2024 Jason Scurtu <github@mail.scurtu.me> - 0.1.0-2
- Production quality refactoring
- Add comprehensive error handling with thiserror
- Add BrightnessCalculator to eliminate code duplication
- Fix async/blocking mixing patterns
- Add 9 unit tests for brightness calculations
- Add structured logging with field-based tracing
- Improve hotplug DDC/CI reliability (5 retries with increasing delays)
- Update ddc-hi to 0.4.1

* Mon Nov 25 2024 Jason Scurtu <github@mail.scurtu.me> - 0.1.0-1
- Initial package
- Add native Apple Studio Display support via USB HID protocol
- Add protocol-based architecture supporting DDC/CI and Apple HID
- Add udev rules for Apple display USB permissions
- Support for multiple display protocols simultaneously
