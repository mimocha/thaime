#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
#
# Full build pipeline: regenerate versioned dictionaries and rebuild all
# downstream artifacts (native engine, WASM, web demo).
#
# Usage: ./build.sh [input.json]
#   input.json  Path to trie_dataset.json (default: ./data/input/trie_dataset.json)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# --- Configuration ---
DEFAULT_INPUT="./data/input/trie_dataset.json"
INPUT="${1:-$DEFAULT_INPUT}"
DICT_DIR="data/dict"

# --- Extract version from workspace Cargo.toml ---
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
VTAG="v$(echo "$VERSION" | tr '.' '_')"

echo "=== THAIME build pipeline ==="
echo "Version: $VERSION ($VTAG)"
echo "Input:   $INPUT"
echo ""

if [ ! -f "$INPUT" ]; then
    echo "ERROR: Input file not found: $INPUT"
    exit 1
fi

# --- Step 1: Build dictgen ---
echo "--- [1/6] Building thaime_dictgen (release) ---"
cargo build -r -p thaime_dictgen

# --- Step 2: Generate dictionary ---
echo "--- [2/6] Generating dictionary ---"
target/release/thaime_dictgen "$INPUT" "$DICT_DIR"

# --- Step 3: Rename to versioned filenames ---
echo "--- [3/6] Versioning dictionary files ($VTAG) ---"

# Remove old versioned files (avoid stale versions accumulating)
rm -f "$DICT_DIR"/trie-v*.bin "$DICT_DIR"/metadata-v*.bin "$DICT_DIR"/thaime-v*.dict

mv "$DICT_DIR/trie.bin"     "$DICT_DIR/trie-${VTAG}.bin"
mv "$DICT_DIR/metadata.bin" "$DICT_DIR/metadata-${VTAG}.bin"
mv "$DICT_DIR/thaime.dict"  "$DICT_DIR/thaime-${VTAG}.dict"

echo "  trie-${VTAG}.bin"
echo "  metadata-${VTAG}.bin"
echo "  thaime-${VTAG}.dict"

# --- Step 4: Rebuild workspace (embeds fresh dict) ---
echo "--- [4/6] Building workspace (release) ---"
cargo build -r --workspace

# --- Step 5: Build WASM ---
echo "--- [5/6] Building WASM package ---"
wasm-pack build crates/thaime_wasm --target web

# --- Step 6: Copy dict to web demo ---
echo "--- [6/6] Copying dict to web demo ---"
DICT_FILE="thaime-${VTAG}.dict"
mkdir -p web/public/dict
cp "$DICT_DIR/$DICT_FILE" "web/public/dict/$DICT_FILE"

# Write .env for Vite to pick up the versioned filename
echo "VITE_DICT_FILE=$DICT_FILE" > web/.env

echo ""
echo "=== Build complete ==="
echo "Dict:  $DICT_DIR/thaime-${VTAG}.dict"
echo "WASM:  crates/thaime_wasm/pkg/"
echo "Web:   web/public/dict/$DICT_FILE"
