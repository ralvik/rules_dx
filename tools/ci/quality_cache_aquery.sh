#!/usr/bin/env bash
# Quality cache aquery proof (issue #84, seed slice): per-adapter action-key
# isolation via `bazel aquery` over real quality pipelines.
#
# The cache-correctness table in docs/quality/quality-testing.md requires,
# for each check kind, that one direct source misses only owning pipelines
# while unrelated files cause no miss. This harness proves the aquery
# action-shape half for two adapters (Ruff/Python lint via ruff tool,
# Rust lint via clippy-driver): each pipeline's declared Inputs mention
# its own source and tool and none of the other's, and lint vs format
# ActionKeys differ so capability changes invalidate.
#
# Still open per #84 (recorded as gap, not claimed): full per-adapter
# table (every adapter + Ty/transitive/config/tool-version rows),
# pipeline invalidation (tool/config/stage-order/runner changes),
# formatter-set and class-membership rules, plus exec-log/remote-cache
# proof distinguishing executed actions from cache hits (requires
# controlled remote cache or separate machines per testing contract;
# a warm local no-op alone is not a cache test). This harness is
# action-graph only, no execution.
#
# Run by CI via `bazel run //tools/ci:quality_cache_aquery`,
# after //tools/ci:examples_laziness_aquery.
set -euo pipefail

if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]]; then
  workspace="$BUILD_WORKSPACE_DIRECTORY"
else
  workspace="$(git rev-parse --show-toplevel)"
fi
cd "$workspace"

pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

query_target() { # target -> aquery output
  local target="$1"
  bazel aquery "$target" \
    --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect \
    --output_groups=dx_results --output=text --noshow_progress 2>/dev/null
}

python_actions="$(query_target '//quality/testdata:fixture_real_python')"
rust_actions="$(query_target '//quality/testdata:fixture_real_rust')"
if [[ -z "$python_actions" ]]; then
  bad "python: empty aquery output"
else
  ok
fi
if [[ -z "$rust_actions" ]]; then
  bad "rust: empty aquery output"
else
  ok
fi

# Python lint owns its source and tool, not Rust's.
if [[ "$python_actions" == *"real_clean.py"* ]]; then ok; else bad "python: want [real_clean.py] in lint/format inputs"; fi
if [[ "$python_actions" == *"ruff"* ]]; then ok; else bad "python: want [ruff] in lint/format inputs"; fi
if [[ "$python_actions" == *"clean.rs"* ]]; then bad "python: forbidden [clean.rs] in inputs (unrelated rust source leaks)"; else ok; fi
if [[ "$python_actions" == *"clippy-driver"* ]]; then bad "python: forbidden [clippy-driver] in inputs (unrelated rust tool leaks)"; else ok; fi

# Rust lint owns its source and tool, not Python's.
if [[ "$rust_actions" == *"clean.rs"* ]]; then ok; else bad "rust: want [clean.rs] in lint/format inputs"; fi
if [[ "$rust_actions" == *"clippy-driver"* ]]; then ok; else bad "rust: want [clippy-driver] in lint/format inputs"; fi
if [[ "$rust_actions" == *"real_clean.py"* ]]; then bad "rust: forbidden [real_clean.py] in inputs (unrelated python source leaks)"; else ok; fi
if [[ "$rust_actions" == *"ruff"* ]]; then bad "rust: forbidden [ruff] in inputs (unrelated python tool leaks)"; else ok; fi

# ActionKeys differ across pipelines and capabilities, so changing one
# source invalidates only its owning pipeline.
python_key="$(printf '%s' "$python_actions" | grep 'ActionKey:' | head -1 || true)"
rust_key="$(printf '%s' "$rust_actions" | grep 'ActionKey:' | head -1 || true)"
if [[ -n "$python_key" && -n "$rust_key" ]]; then ok; else bad "want ActionKey lines in both aquery outputs"; fi
if [[ "$python_key" != "$rust_key" ]]; then ok; else bad "python vs rust ActionKeys must differ (pipelines isolated)"; fi
python_keys="$(printf '%s' "$python_actions" | grep 'ActionKey:' || true)"
python_key_count="$(printf '%s' "$python_keys" | wc -l | tr -d ' ')"
if [[ "$python_key_count" -ge 2 ]]; then ok; else bad "python: want >=2 ActionKeys (lint vs format capabilities)"; fi
first_key="$(printf '%s' "$python_keys" | head -1)"
second_key="$(printf '%s' "$python_keys" | sed -n '2p')"
if [[ "$first_key" != "$second_key" ]]; then ok; else bad "python lint vs format ActionKeys must differ"; fi

echo "quality cache aquery: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
