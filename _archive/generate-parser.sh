#!/usr/bin/env bash
# Regenerate parser.rs from udon.machine
#
# Usage: ./generate-parser.sh

set -e
cd "$(dirname "$0")"

echo "Regenerating parser from generator/udon.machine..."
ruby generator/genmachine-rs generator/udon.machine > udon-core/src/parser.rs

echo "Done. Run 'cargo build' to verify."
