#!/usr/bin/env bash
# Renovate parity fixture (issue #3): the repository's own
# `renovate.json` must stay byte-identical to the `dx init`
# scaffold output, so the config we ship is the one we run.
# A drifting hand copy is worse than none: this test fails the
# moment the scaffold and the checked-in file disagree.
#
# Shell sources have no corpus class. Process-spawning tests stay out of
# the coverage denominator per the repo coverage preset.
set -euo pipefail

expected="$(realpath "$1")"
dx_bin="$(realpath "$2")"

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

# The `dx` binary resolves its workspace through a `MODULE.bazel`
# marker (or `--workspace` override): seed a stub workspace so `init`
# plans against an empty tree instead of this repository.
touch "$scratch/MODULE.bazel"
"${dx_bin}" --workspace "$scratch" init --quiet >/dev/null

diff -u "$expected" "$scratch/renovate.json"
echo "renovate parity: scaffold output matches checked-in definition"
