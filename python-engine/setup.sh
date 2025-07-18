#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPONENT_DIR="/usr/share/ibus/component"

echo "Thaime Python IBus Setup"
echo "============================="

# Check if running as root for system installation
if [[ $EUID -eq 0 ]]; then
    echo "Running as root - installing system-wide"
    USER_MODE=false
else
    echo "Running as user - installing to user directory"
    USER_MODE=true
    COMPONENT_DIR="$HOME/.local/share/ibus/component"
fi

# Install dependencies (Ubuntu / Debian)
echo "1. Installing system dependencies..."
if command -v apt >/dev/null 2>&1; then
    if [[ $USER_MODE == false ]]; then
        apt update
        apt install -y ibus ibus-gtk3 python3-gi gir1.2-ibus-1.0 libibus-1.0-dev
    else
        echo "   Please install dependencies manually as root:"
        echo "   sudo apt install -y ibus ibus-gtk3 python3-gi gir1.2-ibus-1.0 libibus-1.0-dev"
    fi
else
    echo "   Please install IBus and Python GI bindings manually"
fi

# Test Python dependencies
echo "2. Testing Python dependencies..."
python3 -c "import gi; gi.require_version('IBus', '1.0'); from gi.repository import IBus; print('   ✓ IBus Python bindings available')" || {
    echo "   ✗ IBus Python bindings not available"
    exit 1
}

# Create component directory
echo "3. Creating component directory..."
mkdir -p "$COMPONENT_DIR"
echo "   ✓ Component directory: $COMPONENT_DIR"

# Update XML file with correct path
echo "4. Installing component configuration..."
XML_FILE="$SCRIPT_DIR/thaime-python.xml"
TEMP_XML="/tmp/thaime-python.xml"

# Update the exec path in XML
sed "s|<exec>.*</exec>|<exec>$SCRIPT_DIR/ibus-engine-thaime-python --ibus</exec>|" "$XML_FILE" > "$TEMP_XML"

# Copy to component directory
if [[ $USER_MODE == false ]]; then
    cp "$TEMP_XML" "$COMPONENT_DIR/"
else
    cp "$TEMP_XML" "$COMPONENT_DIR/"
fi
rm "$TEMP_XML"
echo "   ✓ Component configuration installed"

# Make launcher executable
echo "5. Setting up launcher..."
chmod +x "$SCRIPT_DIR/ibus-engine-thaime-python"
echo "   ✓ Launcher script made executable"

# Test engine import
echo "6. Testing engine modules..."
cd "$SCRIPT_DIR"
python3 -c "import main, factory, engine; print('   ✓ All modules import successfully')" || {
    echo "   ✗ Module import failed"
    exit 1
}

echo ""
echo "Installation completed successfully!"
echo ""
echo "Next steps:"
echo "1. Restart IBus daemon:"
echo "   ibus-daemon -drx"
echo ""
echo "2. Verify engine registration:"
echo "   ibus list-engine | grep thaime"
echo ""
echo "3. Select the engine:"
echo "   ibus engine thaime-python"
echo ""
echo "4. Test keystroke logging:"
echo "   cd $SCRIPT_DIR && python test_ime_functionality.py"
echo ""
echo "The engine is now ready to use!"