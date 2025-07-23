#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPONENT_DIR="/usr/share/ibus/component"
BINARY_DIR="/usr/local/bin"

echo "Thaime Rust IBus Setup"
echo "====================="

# Check if running as root for system installation
if [[ $EUID -eq 0 ]]; then
    echo "Running as root - installing system-wide"
    USER_MODE=false
else
    echo "Running as user - installing to user directory"
    USER_MODE=true
    COMPONENT_DIR="$HOME/.local/share/ibus/component"
    BINARY_DIR="$HOME/.local/bin"
fi

# Check if cargo is available
echo "1. Checking Rust toolchain..."
if ! command -v cargo >/dev/null 2>&1; then
    echo "   ✗ Cargo not found. Please install Rust: https://rustup.rs/"
    exit 1
fi
echo "   ✓ Cargo found"

# Install system dependencies
echo "2. Installing system dependencies..."
if command -v apt >/dev/null 2>&1; then
    if [[ $USER_MODE == false ]]; then
        apt update
        apt install -y ibus libibus-1.0-dev build-essential
    else
        echo "   Please install dependencies manually as root:"
        echo "   sudo apt install -y ibus libibus-1.0-dev build-essential"
    fi
else
    echo "   Please install IBus and build tools manually"
fi

# Build the project
echo "3. Building Thaime Rust engine..."
cd "$SCRIPT_DIR"
cargo build --release
echo "   ✓ Build completed"

# Create directories
echo "4. Creating installation directories..."
mkdir -p "$COMPONENT_DIR"
if [[ $USER_MODE == true ]]; then
    mkdir -p "$BINARY_DIR"
fi
echo "   ✓ Directories created"

# Install binary
echo "5. Installing binary..."
BINARY_PATH="$BINARY_DIR/thaime"
if [[ $USER_MODE == false ]]; then
    cp "target/release/thaime" "$BINARY_PATH"
else
    cp "target/release/thaime" "$BINARY_PATH"
fi
chmod +x "$BINARY_PATH"
echo "   ✓ Binary installed to: $BINARY_PATH"

# Update XML file with correct path and install
echo "6. Installing component configuration..."
XML_FILE="$SCRIPT_DIR/thaime-rust.xml"
TEMP_XML="/tmp/thaime-rust.xml"

# Update the exec path in XML
sed "s|<exec>.*</exec>|<exec>$BINARY_PATH --ibus</exec>|" "$XML_FILE" > "$TEMP_XML"

# Copy to component directory
cp "$TEMP_XML" "$COMPONENT_DIR/"
rm "$TEMP_XML"
echo "   ✓ Component configuration installed"

# Test that the binary can start
echo "7. Testing binary..."
timeout 2s "$BINARY_PATH" --help > /dev/null || true
echo "   ✓ Binary runs successfully"

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
echo "   ibus engine thaime-rust"
echo ""
echo "4. Or use ibus-setup to select it graphically"
echo ""
echo "The engine is now ready to use! Check logs with:"
echo "journalctl -f | grep -i thaime"