#!/usr/bin/env bash
# Devcontainer parity fixture (issue #183): the repository's own
# `.devcontainer/devcontainer.json` must stay byte-identical to the
# `dx init` scaffold output, so the definition we ship is the one we
# boot. A drifting hand copy is worse than none: this test fails the
# moment the scaffold and the checked-in file disagree.
#
# Shell sources have no corpus class. Process-spawning tests stay out of
# the coverage denominator per the repo coverage preset.
set -euo pipefail

# Portable realpath (issue #299): GNU `realpath` is absent on macOS;
# `readlink -f` covers some platforms, python3 covers the rest.
portable_realpath() {
  if command -v realpath >/dev/null 2>&1; then
    realpath "$1"
  elif command -v readlink >/dev/null 2>&1 && readlink -f "$1" >/dev/null 2>&1; then
    readlink -f "$1"
  else
    python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$1"
  fi
}

expected="$(portable_realpath "$1")"
dx_bin="$(portable_realpath "$2")"

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

# The `dx` binary resolves its workspace through a `MODULE.bazel`
# marker (or `--workspace` override): seed a stub workspace so `init`
# plans against an empty tree instead of this repository.
touch "$scratch/MODULE.bazel"
"${dx_bin}" --workspace "$scratch" init --quiet >/dev/null

diff -u "$expected" "$scratch/.devcontainer/devcontainer.json"
echo "devcontainer parity: scaffold output matches checked-in definition"
