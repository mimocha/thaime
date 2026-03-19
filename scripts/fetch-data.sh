#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0

# SPDX-License-Identifier: MPL-2.0
#
# Fetch NLP data artifacts from thaime-nlp GitHub Releases,
# verify checksums, decompress, and compile trie binaries.
#
# Prerequisites:
#   - gh (GitHub CLI): https://cli.github.com/
#   - gunzip, sha256sum
#   - Rust toolchain (for trie compilation)
#
# Usage:
#   ./scripts/fetch-data.sh
#
# Environment variables:
#   THAIME_DATA_DIR  — when set, skip download and use this local directory
#                      (must contain the uncompressed artifact files)

set -euo pipefail

# ---------------------------------------------------------------------------
# Shared helpers
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib-common.sh"

REPO=$(parse_toml_value "repo")
VERSION=$(parse_toml_value "version")

TRIE_FILE=$(parse_artifact_field "trie_dataset" "file")
TRIE_SHA256=$(parse_artifact_field "trie_dataset" "sha256")

NGRAM_FILE=$(parse_artifact_field "ngram" "file")
NGRAM_SHA256=$(parse_artifact_field "ngram" "sha256")

# Version tag for output filenames: "v1.0.0" → "v1_0_0"
VTAG="${VERSION//./_}"

echo "=== THAIME Data Fetch ==="
echo "Repo:    $REPO"
echo "Version: $VERSION ($VTAG)"
echo "Trie:    $TRIE_FILE"
echo "N-gram:  $NGRAM_FILE"
echo

# ---------------------------------------------------------------------------
# Directories
# ---------------------------------------------------------------------------
INPUT_DIR="$WORKSPACE_ROOT/data/input"
DICT_DIR="$WORKSPACE_ROOT/data/dict"
mkdir -p "$INPUT_DIR" "$DICT_DIR"

# ---------------------------------------------------------------------------
# Acquire artifacts (download or copy from local dir)
# ---------------------------------------------------------------------------
if [ -n "${THAIME_DATA_DIR:-}" ]; then
    echo "Using local data directory: $THAIME_DATA_DIR"

    # Strip .gz suffix — local dir should have uncompressed files
    TRIE_BASENAME="${TRIE_FILE%.gz}"
    NGRAM_BASENAME="${NGRAM_FILE%.gz}"

    cp "$THAIME_DATA_DIR/$TRIE_BASENAME" "$INPUT_DIR/$TRIE_BASENAME"
    cp "$THAIME_DATA_DIR/$NGRAM_BASENAME" "$INPUT_DIR/$NGRAM_BASENAME"

    echo "Copied uncompressed artifacts from local directory."
    echo "WARNING: Local files are not verified against pinned checksums."
    echo "         (Pinned checksums are for compressed .gz artifacts; local files are uncompressed.)"
else
    echo "Downloading from GitHub Release ${VERSION}..."

    # Check for gh CLI
    if ! command -v gh &>/dev/null; then
        echo "Error: 'gh' (GitHub CLI) is required. Install from https://cli.github.com/"
        exit 1
    fi

    # Download .gz files into input dir
    gh release download "$VERSION" \
        --repo "$REPO" \
        --pattern "$TRIE_FILE" \
        --pattern "$NGRAM_FILE" \
        --dir "$INPUT_DIR" \
        --clobber

    # Verify checksums
    echo "Verifying checksums..."
    echo "$TRIE_SHA256  $INPUT_DIR/$TRIE_FILE" | sha256sum -c -
    echo "$NGRAM_SHA256  $INPUT_DIR/$NGRAM_FILE" | sha256sum -c -

    # Decompress
    echo "Decompressing..."
    gunzip -f "$INPUT_DIR/$TRIE_FILE"
    gunzip -f "$INPUT_DIR/$NGRAM_FILE"

    echo "Download and decompression complete."
fi

# ---------------------------------------------------------------------------
# Compile trie binaries (via thaime_dictgen)
# ---------------------------------------------------------------------------
TRIE_BASENAME="${TRIE_FILE%.gz}"
NGRAM_BASENAME="${NGRAM_FILE%.gz}"

echo
echo "Compiling trie dictionary..."
cargo run -r -p thaime_dictgen -- \
    "$INPUT_DIR/$TRIE_BASENAME" \
    "$DICT_DIR" \
    --version-tag "$VTAG"

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo
echo "=== Done ==="
echo "Input files:"
echo "  $INPUT_DIR/$TRIE_BASENAME"
echo "  $INPUT_DIR/$NGRAM_BASENAME"
echo "Dict files:"
ls -lh "$DICT_DIR"/*-"${VTAG}"* 2>/dev/null || echo "  (versioned files not found — check dictgen output)"
