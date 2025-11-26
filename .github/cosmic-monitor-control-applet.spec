Name:           cosmic-monitor-control-applet
Version:        0.2.1
Release:        1%{?dist}
Summary:        Control external monitors (DDC/CI and Apple displays) for COSMIC Desktop
License:        GPL-3.0-only
URL:            https://github.com/xarbit/cosmic-monitor-control-applet
Source0:        %{name}-%{version}.tar.gz

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
A COSMIC desktop applet for controlling external monitor brightness via DDC/CI
and Apple HID protocols. Supports standard DDC/CI monitors and native Apple
displays (Studio Display, Pro Display XDR) and LG UltraFine displays.

Features:
- Control brightness of external monitors from the COSMIC panel
- DDC/CI protocol support (most modern monitors)
- Native Apple display support via USB HID
- Automatic brightness sync with keyboard brightness keys
- Brightness profiles for multiple monitors
- Gamma curve adjustment per monitor
- Automatic hotplug detection

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
- Optimize hotplug detection and UI responsiveness
- Reduce DDC/CI delays for faster UI updates
- Improve wake-up logic for better startup detection
- Fix unplug detection with timeout-based removal
- Natural popup sizing

* Mon Nov 25 2024 Jason Scurtu <github@mail.scurtu.me> - 0.2.0-1
- Fix panel crashes during display hotplug events
- Add comprehensive error handling
- Implement singleton patterns for stability

* Sun Nov 24 2024 Jason Scurtu <github@mail.scurtu.me> - 0.1.0-1
- Initial release
