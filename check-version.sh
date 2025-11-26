#!/bin/bash
# Check if the installed applet has the daemon lock code

echo "Checking installed applet..."
if strings /usr/bin/cosmic-monitor-control-applet | grep -q "Acquired daemon lock"; then
    echo "✓ Installed binary has daemon lock code"
else
    echo "✗ Installed binary MISSING daemon lock code - needs reinstall!"
fi

echo ""
echo "Checking built applet..."
if strings target/release/cosmic-monitor-control-applet | grep -q "Acquired daemon lock"; then
    echo "✓ Built binary has daemon lock code"
else
    echo "✗ Built binary MISSING daemon lock code - rebuild needed!"
fi

echo ""
echo "Binary timestamps:"
stat -c "Installed: %y" /usr/bin/cosmic-monitor-control-applet
stat -c "Built:     %y" target/release/cosmic-monitor-control-applet

echo ""
echo "To install the latest version, run:"
echo "  ./install-and-restart-panel.sh"
