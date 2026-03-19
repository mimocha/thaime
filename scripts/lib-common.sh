#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0

# SPDX-License-Identifier: MPL-2.0
#
# Shared helpers for thaime build scripts.
# Source this file — do not execute directly.
#
# Provides:
#   WORKSPACE_ROOT  — absolute path to the repository root
#   CONFIG          — absolute path to thaime-data.toml
#   parse_toml_value <key>               — extract a top-level string value
#   parse_artifact_field <artifact> <field> — extract a field from an inline table

# Resolve workspace root (one level up from scripts/)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CONFIG="$WORKSPACE_ROOT/thaime-data.toml"

# ---------------------------------------------------------------------------
# Lightweight TOML parsing (no external dependency)
# ---------------------------------------------------------------------------
parse_toml_value() {
    local key="$1"
    grep -E "^${key}\s*=" "$CONFIG" | head -1 | sed 's/.*=\s*"\(.*\)"/\1/'
}

parse_artifact_field() {
    local artifact="$1" field="$2"
    grep -E "^${artifact}\s*=" "$CONFIG" \
        | sed "s/.*${field}\s*=\s*\"\([^\"]*\)\".*/\1/"
}
