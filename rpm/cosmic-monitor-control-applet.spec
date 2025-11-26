Name:           cosmic-monitor-control-applet
Version:        0.2.1
Release:        1%{?dist}
Summary:        External Monitor Control Applet for COSMIC Desktop

License:        GPL-3.0-only
URL:            https://github.com/xarbit/cosmic-monitor-control-applet
Source0:        https://github.com/xarbit/%{name}/archive/v%{version}/%{name}-%{version}.tar.gz

ExclusiveArch:  %{rust_arches}

BuildRequires:  rust >= 1.80
BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  libi2c-devel
BuildRequires:  hidapi-devel
BuildRequires:  systemd-devel
BuildRequires:  pkgconfig(libudev)

Requires:       i2c-tools
Requires:       hidapi

%description
External Monitor Control Applet for the COSMIC™ desktop environment.
Control external monitors using:
- DDC/CI protocol for standard external monitors via I2C
- Native USB HID support for Apple Studio Display and LG UltraFine displays
- Brightness profiles for saving/restoring settings across multiple monitors
- Automatic brightness sync with COSMIC keyboard brightness keys
- Quick toggle for system dark mode

%prep
%setup -q

%build
cargo build --release --all-features

%install
install -Dm755 target/release/%{name} %{buildroot}%{_bindir}/%{name}

%files
%license LICENSE
%doc README.md CLAUDE.md
%{_bindir}/%{name}

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
- Update libcosmic to latest from main branch
- Update zbus to 5.12.0
- Add once_cell dependency for singleton pattern
- Add libudev-dev to build documentation (#15)
- Improve portable monitor diagnostics (#34)
- Add detailed logging for 0% brightness reports
- Add structured error logging with display context

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
