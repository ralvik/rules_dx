#!/usr/bin/env bash
# Quality cache aquery proof (issue #84, slices 1-2): per-adapter and
# per-capability action-key isolation via `bazel aquery` over real
# quality pipelines.
#
# The cache-correctness table in docs/quality/quality-testing.md requires,
# for each check kind, that one direct source misses only owning pipelines
# while unrelated files cause no miss, and that changing one tool
# invalidates only affected capability actions. This harness proves the
# aquery action-shape half for Python (ruff lint/format, pydoclint lint,
# Ty typecheck) and Rust (clippy lint, rustfmt format): each pipeline's
# declared Inputs mention its own source and tool and none of the other's,
# lint vs format vs typecheck ActionKeys differ, and per-capability Inputs
# contain only their owning tool (ruff change misses lint/format but not
# typecheck; ty misses typecheck only; pydoclint misses lint only).
#
# Still open per #84 (recorded as gap, not claimed): full per-adapter
# table (every adapter + transitive/config/tool-version rows),
# pipeline invalidation (native config/stage-order/runner changes),
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

# Per-capability tool isolation: changing one tool invalidates only its
# owning capability action. Markers use ecosystem-specific repo strings
# (dx_ty for Ty, ruff, pydoclint, clippy-driver, rustfmt) to avoid
# substring collisions with common words like quality.
python_lint_inputs="$(printf '%s' "$python_actions" | grep -A 8 'Mnemonic: DxRealQualityLint' | grep 'Inputs:' | head -1 || true)"
python_format_inputs="$(printf '%s' "$python_actions" | grep -A 8 'Mnemonic: DxRealQualityFormat' | grep 'Inputs:' | head -1 || true)"
python_typecheck_inputs="$(printf '%s' "$python_actions" | grep -A 8 'Mnemonic: DxRealQualityTypecheck' | grep 'Inputs:' | head -1 || true)"
rust_lint_inputs="$(printf '%s' "$rust_actions" | grep -A 8 'Mnemonic: DxRealQualityLint' | grep 'Inputs:' | head -1 || true)"
rust_format_inputs="$(printf '%s' "$rust_actions" | grep -A 8 'Mnemonic: DxRealQualityFormat' | grep 'Inputs:' | head -1 || true)"
if [[ "$python_lint_inputs" == *"ruff"* ]]; then ok; else bad "python lint: want [ruff]"; fi
if [[ "$python_lint_inputs" == *"pydoclint"* ]]; then ok; else bad "python lint: want [pydoclint]"; fi
if [[ "$python_lint_inputs" == *"dx_ty"* ]]; then bad "python lint: forbidden [dx_ty] (ty change must not invalidate lint)"; else ok; fi
if [[ "$python_format_inputs" == *"ruff"* ]]; then ok; else bad "python format: want [ruff]"; fi
if [[ "$python_format_inputs" == *"dx_ty"* ]]; then bad "python format: forbidden [dx_ty]"; else ok; fi
if [[ "$python_format_inputs" == *"pydoclint"* ]]; then bad "python format: forbidden [pydoclint] (pydoclint change must not invalidate format)"; else ok; fi
if [[ "$python_typecheck_inputs" == *"dx_ty"* ]]; then ok; else bad "python typecheck: want [dx_ty]"; fi
if [[ "$python_typecheck_inputs" == *"ruff"* ]]; then bad "python typecheck: forbidden [ruff] (ruff change must not invalidate typecheck)"; else ok; fi
if [[ "$rust_lint_inputs" == *"clippy-driver"* ]]; then ok; else bad "rust lint: want [clippy-driver]"; fi
if [[ "$rust_lint_inputs" == *"rustfmt"* ]]; then bad "rust lint: forbidden [rustfmt]"; else ok; fi
if [[ "$rust_format_inputs" == *"rustfmt"* ]]; then ok; else bad "rust format: want [rustfmt]"; fi
if [[ "$rust_format_inputs" == *"clippy-driver"* ]]; then bad "rust format: forbidden [clippy-driver]"; else ok; fi

echo "quality cache aquery: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
