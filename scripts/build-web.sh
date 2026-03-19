#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
#
# Web demo build pipeline.
# Requires running fetch-data.sh first to populate data/ with binaries.
#
# Prerequisites:
#   - wasm-pack: https://rustwasm.github.io/wasm-pack/
#   - Rust toolchain with wasm32-unknown-unknown target
#
# Usage: ./scripts/build-web.sh

set -euo pipefail

# ---------------------------------------------------------------------------
# Shared helpers
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib-common.sh"

# --- Configuration ---
DICT_DIR="$WORKSPACE_ROOT/data/dict"

# --- Extract data version from thaime-data.toml ---
VERSION=$(parse_toml_value "version")
VTAG="${VERSION//./_}"
NGRAM_FILE=$(parse_artifact_field "ngram" "file")
NGRAM_FILE="${NGRAM_FILE%.gz}"

echo "=== THAIME build pipeline ==="
echo "Version: $VERSION ($VTAG)"
echo ""

# --- Step 0: Check prerequisites ---
if ! command -v wasm-pack >/dev/null 2>&1; then
    echo "Error: wasm-pack is required. Install from https://rustwasm.github.io/wasm-pack/"
    exit 1
fi

# --- Step 1: Rebuild workspace (embeds fresh dict) ---
echo "--- [1/3] Building workspace (release) ---"
cargo build -r --workspace

# --- Step 2: Build WASM ---
echo "--- [2/3] Building WASM package ---"
wasm-pack build "$WORKSPACE_ROOT/crates/thaime_wasm" --target web

# --- Step 3: Copy dict + ngram to web demo ---
echo "--- [3/3] Copying dict + ngram to web demo ---"

WEB_DICT_DIR="$WORKSPACE_ROOT/web/public/dict"

# Remove old versioned files (avoid stale versions accumulating)
rm -f "$WEB_DICT_DIR"/thaime-v*.dict
rm -f "$WEB_DICT_DIR"/thaime_ngram_v*.bin

DICT_FILE="thaime-${VTAG}.dict"
mkdir -p "$WEB_DICT_DIR"
cp "$DICT_DIR/$DICT_FILE" "$WEB_DICT_DIR/$DICT_FILE"

# Write .env for Vite to pick up the versioned filename
echo "VITE_DICT_FILE=$DICT_FILE" > "$WORKSPACE_ROOT/web/.env"

# Copy pinned ngram binary if available
INPUT_DIR="$WORKSPACE_ROOT/data/input"
if [ -n "$NGRAM_FILE" ] && [ -f "$INPUT_DIR/$NGRAM_FILE" ]; then
    cp "$INPUT_DIR/$NGRAM_FILE" "$WEB_DICT_DIR/$NGRAM_FILE"
    echo "VITE_NGRAM_FILE=$NGRAM_FILE" >> "$WORKSPACE_ROOT/web/.env"
    echo "  Ngram: $WEB_DICT_DIR/$NGRAM_FILE"
else
    echo "  (no configured ngram binary found in $INPUT_DIR/ — skipping)"
fi

echo ""
echo "=== Build complete ==="
echo "Dict:  $DICT_DIR/thaime-${VTAG}.dict"
echo "WASM:  $WORKSPACE_ROOT/crates/thaime_wasm/pkg/"
echo "Web:   $WEB_DICT_DIR/$DICT_FILE"
