#!/usr/bin/env bash
set -e

BINARY_NAME="rustyfin"
SYMLINK_NAME="rf"

echo "=== Installing RustyFin ==="
echo

# Build release binary
echo "Building release binary..."
cargo build --release 2>&1 | tail -5
echo

# Find the built binary
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BINARY_PATH="$SCRIPT_DIR/target/release/$BINARY_NAME"
if [ ! -f "$BINARY_PATH" ]; then
    echo "ERROR: Binary not found at $BINARY_PATH"
    exit 1
fi

# Determine install directory - try multiple options
INSTALL_DIR=""

# Option 1: ~/.cargo/bin (standard for Rust tools)
CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
# Option 2: ~/.local/bin (standard for user binaries)
LOCAL_BIN="$HOME/.local/bin"

# Try cargo bin first, then local bin
for dir in "$CARGO_BIN" "$LOCAL_BIN"; do
    if mkdir -p "$dir" 2>/dev/null; then
        # Test we can actually write to it
        if touch "$dir/.rustyfin_test" 2>/dev/null; then
            rm -f "$dir/.rustyfin_test"
            INSTALL_DIR="$dir"
            break
        fi
    fi
done

if [ -z "$INSTALL_DIR" ]; then
    echo "ERROR: Could not find a writable install directory."
    echo "Tried: $CARGO_BIN, $LOCAL_BIN"
    echo
    echo "You can manually copy the binary:"
    echo "  cp $BINARY_PATH /usr/local/bin/$BINARY_NAME"
    echo "  ln -sf /usr/local/bin/$BINARY_NAME /usr/local/bin/$SYMLINK_NAME"
    exit 1
fi

# Copy binary (remove old one first to avoid "Text file busy" if it's running)
echo "Installing $BINARY_NAME to $INSTALL_DIR/"
rm -f "$INSTALL_DIR/$BINARY_NAME" 2>/dev/null || true
cp "$BINARY_PATH" "$INSTALL_DIR/$BINARY_NAME"
chmod +x "$INSTALL_DIR/$BINARY_NAME"

# Create shorthand symlink
ln -sf "$INSTALL_DIR/$BINARY_NAME" "$INSTALL_DIR/$SYMLINK_NAME"
echo "Created shorthand: $SYMLINK_NAME -> $BINARY_NAME"

echo
echo "=== Installation complete! ==="
echo
echo "  Type 'rustyfin' or 'rf' to start the TUI."
echo

# Check if install dir is in PATH
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
    echo "NOTE: $INSTALL_DIR is not in your PATH yet."
    echo "Add it by running:"
    echo
    SHELL_NAME="$(basename "$SHELL")"
    case "$SHELL_NAME" in
        zsh)  RC_FILE="~/.zshrc" ;;
        bash) RC_FILE="~/.bashrc" ;;
        fish) RC_FILE="~/.config/fish/config.fish" ;;
        *)    RC_FILE="~/.profile" ;;
    esac
    echo "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> $RC_FILE"
    echo "  source $RC_FILE"
    echo
    echo "Or for this session only:"
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    echo
fi
