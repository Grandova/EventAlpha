#!/usr/bin/env bash
# ==============================================================================
# PolyQuant 5M - Root Entrypoint for One-Click Linux & Baota Installation
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec bash "$SCRIPT_DIR/deploy/install.sh" "$@"
