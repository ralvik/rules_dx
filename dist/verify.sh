#!/bin/sh
# Verify dist/ digests reproduce from exact published bytes (M28 delivery).
set -eu
cd "$(dirname "$0")"
sha256sum -c SHA256SUMS
echo "dist verify: digests match exact published bytes"
