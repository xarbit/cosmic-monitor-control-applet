#!/bin/bash
# Helper script to install the applet and restart the COSMIC panel

set -e  # Exit on error

APPID="io.github.xarbit.CosmicMonitorControlApplet"
BIN_SRC="target/release/cosmic-monitor-control-applet"
BIN_DST="/usr/bin/cosmic-monitor-control-applet"
DESKTOP_DST="/usr/share/applications/${APPID}.desktop"
ICON_DST="/usr/share/icons/hicolor/scalable/apps/${APPID}-symbolic.svg"
METAINFO_DST="/usr/share/metainfo/${APPID}.metainfo.xml"

echo "Building release version..."
cargo build --release

echo ""
echo "Installing applet (requires sudo)..."
sudo install -Dm0755 "$BIN_SRC" "$BIN_DST"
sudo install -Dm0644 res/desktop_entry.desktop "$DESKTOP_DST"
sudo install -Dm0644 res/icons/display-symbolic.svg "$ICON_DST"
sudo install -Dm0644 res/metainfo.xml "$METAINFO_DST"

echo ""
echo "Killing existing COSMIC panel and applet processes..."
pkill -9 cosmic-panel || true
pkill -9 -f cosmic-monitor || true
pkill -9 -f cosmic-applet || true

echo ""
echo "Waiting 3 seconds for all processes to fully stop..."
sleep 3

# Verify everything is killed
if pgrep cosmic-panel > /dev/null; then
    echo "Warning: Some cosmic-panel processes still running, killing again..."
    pkill -9 cosmic-panel
    sleep 2
fi

echo ""
echo "Starting COSMIC panel..."
cosmic-panel > /dev/null 2>&1 &
sleep 2

echo ""
echo "Done! The applet is now installed."
echo ""
echo "The applet will appear on all panels (COSMIC default behavior)."
echo "Only one daemon instance will run to avoid resource conflicts."
echo ""
echo "To test hotplug:"
echo "  1. Unplug your external display"
echo "  2. Plug it back in - panel should NOT crash"
echo "  3. Watch logs: journalctl --user -f | grep cosmic-monitor"
echo ""
