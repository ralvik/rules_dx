#!/usr/bin/env bash
# M12 WP3: upstream Rust IDE tool acquisition proof.
#
# Proves the pinned patched rules_rust supplies the exact IDE binaries the
# environment contract reuses: rust-analyzer discovery (`gen_rust_project`)
# and the Bazel flycheck pipeline (`flycheck`). Each tool must answer
# `--help` with status zero, which proves acquisition from the pinned
# toolchain without invoking project-owned discovery, codegen, or
# environment mutation. Focused exact-target projection is proven by the
# `gen_rust_project //rust/tests/fixtures/hello:hello_lib` run recorded in the M12
# completion report (one crate, `hello`).
set -euo pipefail

# Shared workspace + runfiles helpers (issues #319, #323).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../../tools/sh/lib.sh"

# Portable helpers via tools/sh/lib.sh dx_realpath/dx_mkscratch (issues #299, #323).

discover="$(dx_realpath "$1")"
flycheck="$(dx_realpath "$2")"

discover_help="$("${discover}" --help)"
case "${discover_help}" in
  *"TARGETS"*) ;;
  *)
    echo "gen_rust_project --help missing TARGETS usage" >&2
    echo "${discover_help}" >&2
    exit 1
    ;;
esac

flycheck_help="$("${flycheck}" --help)"
case "${flycheck_help}" in
  *"rust-analyzer flycheck wrapper backed by \`bazel build\`"*) ;;
  *)
    echo "flycheck --help missing wrapper usage" >&2
    echo "${flycheck_help}" >&2
    exit 1
    ;;
esac

echo "ide acquisition: gen_rust_project + flycheck answer --help"
