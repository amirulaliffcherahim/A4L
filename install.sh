#!/usr/bin/env bash
# A4L (Agentic 4 All) Standalone Installer
set -euo pipefail

A4L_HOME="${HOME}/.a4l"
BIN_DIR="${A4L_HOME}/bin"
INSTALL_DESKTOP=true

# Parse flags
for arg in "$@"; do
  case "$arg" in
    --no-desktop)
      INSTALL_DESKTOP=false
      ;;
    --help)
      echo "Usage: install.sh [--no-desktop]"
      exit 0
      ;;
  esac
done

echo "Installing A4L into ${A4L_HOME}..."
mkdir -p "${BIN_DIR}" "${A4L_HOME}/build"

# Build base binary using cargo
echo "Building a4l-base..."
cargo build --release -p a4l-cli

cp "target/release/a4l" "${BIN_DIR}/a4l-base"
cp "${BIN_DIR}/a4l-base" "${BIN_DIR}/a4l"

if [ "$INSTALL_DESKTOP" = true ]; then
  echo "Building a4l-desktop..."
  cargo build --release -p a4l-desktop || echo "Desktop build skipped (requires GUI libraries)"
  if [ -f "target/release/a4l-desktop" ]; then
    cp "target/release/a4l-desktop" "${BIN_DIR}/a4l-desktop"
  fi
fi

echo "✓ A4L installed successfully!"
echo "Add to PATH: export PATH=\"${BIN_DIR}:\$PATH\""
