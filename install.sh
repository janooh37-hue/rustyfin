#!/usr/bin/env bash
set -euo pipefail

BINARY_NAME="rustyfin"
SYMLINK_NAME="rf"
INSTALL_DIR="$HOME/.cargo/bin"

echo "Building RustyFin (release mode)..."
cargo build --release

echo ""

# Ensure install directory exists
mkdir -p "$INSTALL_DIR"

# Find the built binary
BINARY_PATH="$(dirname "$0")/target/release/$BINARY_NAME"
if [ ! -f "$BINARY_PATH" ]; then
    echo "ERROR: Built binary not found at $BINARY_PATH"
    exit 1
fi

# Copy binary
echo "Installing $BINARY_NAME to $INSTALL_DIR/"
cp "$BINARY_PATH" "$INSTALL_DIR/$BINARY_NAME"
chmod +x "$INSTALL_DIR/$BINARY_NAME"

# Create symlink
echo "Creating symlink: $SYMLINK_NAME -> $BINARY_NAME"
ln -sf "$INSTALL_DIR/$BINARY_NAME" "$INSTALL_DIR/$SYMLINK_NAME"

echo ""
echo "Installation complete! Run \`rustyfin\` or \`rf\` to start."

# Check if ~/.cargo/bin is in PATH
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
    echo ""
    echo "WARNING: $INSTALL_DIR is not in your PATH."
    echo "Add it by running:"
    echo ""
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    echo ""
    echo "Or add that line to your ~/.bashrc or ~/.zshrc to make it permanent."
fi
