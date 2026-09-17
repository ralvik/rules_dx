#!/usr/bin/env bash
# Bazel-owned benchmark harness (issue #215): times the built dx binary and
# emits JSON-lines results. No `dx perf` command (ADR 0006 minimal surface).
#
# Usage:
#   bazel build //dx/cli:dx //perf:bench_all
#   bazel run //perf:bench_micro          # microbenchmarks, per-PR (fast, no Bazel query)
#   bazel run //perf:bench_scenario_warm  # warm scenario benchmarks (server warm)
#   bazel run //perf:bench_cold            # cold benchmark: times the full
#                                        #   `bazel run //dx/cli:dx` stack
#                                        #   (server startup included); the
#                                        #   workflow runs `bazel shutdown`
#                                        #   first so the cold path is real
#   bazel run //perf:bench_all            # micro + warm scenario, JSON-lines to stdout
#
# Each output line: {"benchmark": ..., "duration_ms": ..., "iteration": ...,
# "host": ..., "bazel_version": ..., "commit": ...}.
# Comparison against perf/baseline.json happens in perf/compare.py (median-of-N,
# tolerance band, warn-only except gated absolutes).
#
# User-facing slice: run these targets on your own workspace via
# `--workspace DIR` passthrough, e.g.
#   bazel run //perf:bench_micro -- --workspace /path/to/workspace
# Scopes/commands stay fixed to the reference fixture for comparability; the
# workspace flag only changes which checkout the dx binary measures.
set -euo pipefail

# Under `bazel run` the process starts in the target's runfiles directory
# inside bazel-out (where `git rev-parse --show-toplevel` resolves to the
# execroot and nested bazel refuses to run), so the real checkout must come
# from BUILD_WORKSPACE_DIRECTORY. Direct execution falls back to git.
if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]]; then
  workspace="$BUILD_WORKSPACE_DIRECTORY"
else
  workspace="$(git rev-parse --show-toplevel)"
fi
user_workspace=""
dx_override=""
positional=()
expect_workspace=""
for arg in "$@"; do
  if [[ -n "$expect_workspace" ]]; then
    user_workspace="$arg"
    expect_workspace=""
    continue
  fi
  case "$arg" in
    --workspace=*) user_workspace="${arg#--workspace=}" ;;
    --workspace) expect_workspace="1" ;;
    --dx=*) dx_override="${arg#--dx=}" ;;
    *) positional+=("$arg") ;;
  esac
done
if [[ -n "$user_workspace" ]]; then
  workspace="$user_workspace"
fi
cd "$workspace"

if [[ -n "$dx_override" ]]; then
  dx_bin="$dx_override"
else
  dx_bin="$workspace/bazel-bin/dx/cli/dx"
fi
if [[ ! -x "$dx_bin" ]]; then
  echo "perf bench: building //dx/cli:dx first" >&2
  bazel build --noshow_progress //dx/cli:dx >&2
fi

bazel_version="$(cat .bazelversion 2>/dev/null || echo unknown)"
commit="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
host="linux_x86_64"
# Named-benchmark filter: `bench.sh <name>` runs one benchmark; no arg runs all warm.
only="${positional[0]:-all}"

run_case() {
  local name="$1"; shift
  local iterations="$1"; shift
  local i
  for ((i = 1; i <= iterations; i++)); do
    # Timing uses the bash-builtin EPOCHREALTIME (no fork): stamping via a
    # child process (e.g. `python3 -c ...perf_counter()...`) inflates the
    # end stamp by the child's own startup (~8ms here) and corrupts
    # single-digit-millisecond benchmarks.
    local start end ms
    start="$EPOCHREALTIME"
    "$dx_bin" "$@" >/dev/null 2>&1
    local rc=$?
    end="$EPOCHREALTIME"
    ms="$(awk "BEGIN {print ($end - $start) * 1000.0}")"
    python3 -c 'import json,sys; print(json.dumps({"benchmark": sys.argv[1], "duration_ms": float(sys.argv[2]), "iteration": int(sys.argv[3]), "host": sys.argv[4], "bazel_version": sys.argv[5], "commit": sys.argv[6], "rc": int(sys.argv[7])}))' \
      "$name" "$ms" "$i" "$host" "$bazel_version" "$commit" "$rc"
    if [[ "$rc" != "0" && "$rc" != "1" ]]; then
      echo "perf bench: $name iteration $i exited $rc" >&2
      return "$rc"
    fi
  done
}

run_bazel_case() {
  # Cold-path helper: times a full `bazel run //dx/cli:dx -- ...` stack
  # (including server startup) instead of the direct binary.
  local name="$1"; shift
  local iterations="$1"; shift
  local i
  for ((i = 1; i <= iterations; i++)); do
    local start end ms rc
    start="$EPOCHREALTIME"
    rc=0
    bazel run --noshow_progress //dx/cli:dx -- "$@" >/dev/null 2>&1 || rc=$?
    end="$EPOCHREALTIME"
    ms="$(awk "BEGIN {print ($end - $start) * 1000.0}")"
    python3 -c 'import json,sys; print(json.dumps({"benchmark": sys.argv[1], "duration_ms": float(sys.argv[2]), "iteration": int(sys.argv[3]), "host": sys.argv[4], "bazel_version": sys.argv[5], "commit": sys.argv[6], "rc": int(sys.argv[7]), "via": "bazel-run"}))' \
      "$name" "$ms" "$i" "$host" "$bazel_version" "$commit" "$rc"
  done
}

micro() {
  run_case dx_startup 7 --help
  run_case dx_status 7 status
  run_case dx_status_json 7 status --output json
}

scenario_warm() {
  run_case scope_owners 5 owners python/hello/hello.py
  run_case scope_deps 5 deps //python/hello:hello
  run_case generate_check_warm 5 generate --check //examples/adopt-rust/...
}

case "$only" in
  micro) micro ;;
  scenario_warm|warm) scenario_warm ;;
  cold) run_bazel_case generate_check_cold 1 generate --check //examples/adopt-rust/... ;;
  all) micro; scenario_warm ;;
  dx_startup) run_case dx_startup 7 --help ;;
  dx_status) run_case dx_status 7 status ;;
  dx_status_json) run_case dx_status_json 7 status --output json ;;
  scope_owners) run_case scope_owners 5 owners python/hello/hello.py ;;
  scope_deps) run_case scope_deps 5 deps //python/hello:hello ;;
  generate_check_warm) run_case generate_check_warm 5 generate --check //examples/adopt-rust/... ;;
  generate_check_cold) run_bazel_case generate_check_cold 1 generate --check //examples/adopt-rust/... ;;
  *) echo "perf bench: unknown benchmark '$only'; want micro|warm|cold|all|<name>" >&2; exit 2 ;;
esac
