#!/usr/bin/env bash
# Quality cache aquery proof (issue #84, slices 1-3): per-adapter and
# per-capability action-key isolation via `bazel aquery` over real
# quality pipelines.
#
# The cache-correctness table in docs/quality/quality-testing.md requires,
# for each check kind, that one direct source misses only owning pipelines
# while unrelated files cause no miss, and that changing one tool
# invalidates only affected capability actions plus that changing one
# selected native config invalidates only its consuming capability
# actions. This harness proves the aquery action-shape half for Python
# (ruff lint/format, pydoclint lint, Ty typecheck), Rust (clippy lint,
# rustfmt format + rustfmt.toml native-config isolation), and JavaScript
# (biome lint/format + biome.json native-config isolation): each
# pipeline's declared Inputs mention its own source and tool and none of
# the other's, lint vs format vs typecheck ActionKeys differ,
# per-capability Inputs contain only their owning tool (ruff change
# misses lint/format but not typecheck; ty misses typecheck only;
# pydoclint misses lint only), and native configs reach only consuming
# capabilities (ruff.toml misses hinted Python lint/format only;
# rustfmt.toml misses hinted Rust format only, never lint; biome.json
# misses hinted JS lint/format only, never unhinted pipelines).
#
# Still open per #84 (recorded as gap, not claimed): full per-adapter
# table (every adapter + transitive/tool-version rows),
# pipeline invalidation (stage-order/runner changes),
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
union_actions="$(bazel aquery '//quality/testdata:fixture_real_python + //quality/testdata:fixture_real_rust' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
if [[ -z "$union_actions" ]]; then
  bad "union: empty aquery output"
else
  ok
fi
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

# Aggregate membership only: existing check action keys unchanged when the
# query widens from one target to a union. Compare sorted python ActionKeys
# from the single-target query against the python subset of the union
# (ActionKey follows its action header, so -A captures the owning key).
single_keys="$(printf '%s' "$python_actions" | grep 'ActionKey:' | sort || true)"
union_python_keys="$(printf '%s' "$union_actions" | grep -A 8 "action 'Dx.*fixture_real_python'" | grep 'ActionKey:' | sort || true)"
if [[ -z "$single_keys" ]]; then bad "single: want ActionKeys"; else ok; fi
if [[ "$single_keys" == "$union_python_keys" ]]; then ok; else bad "aggregate membership must leave existing action keys unchanged (python single vs union)"; fi

# Shared-config isolation: hinted Python consumes ruff.toml in lint/format
# but typecheck does not, and unhinted lint/format do not. Changing shared
# Ruff config misses consuming lint/format only; Ty boundaries unaffected
# unless shared.
hinted_actions="$(bazel aquery '//quality/testdata:fixture_real_python_hinted' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
if [[ -z "$hinted_actions" ]]; then
  bad "hinted: empty aquery output"
else
  ok
fi
hinted_lint_inputs="$(printf '%s' "$hinted_actions" | grep -A 8 'Mnemonic: DxRealQualityLint' | grep 'Inputs:' | head -1 || true)"
hinted_format_inputs="$(printf '%s' "$hinted_actions" | grep -A 8 'Mnemonic: DxRealQualityFormat' | grep 'Inputs:' | head -1 || true)"
hinted_typecheck_inputs="$(printf '%s' "$hinted_actions" | grep -A 8 'Mnemonic: DxRealQualityTypecheck' | grep 'Inputs:' | head -1 || true)"
if [[ "$hinted_lint_inputs" == *"ruff.toml"* ]]; then ok; else bad "hinted lint: want [ruff.toml] (shared Ruff config is an input)"; fi
if [[ "$hinted_format_inputs" == *"ruff.toml"* ]]; then ok; else bad "hinted format: want [ruff.toml]"; fi
if [[ "$hinted_typecheck_inputs" == *"ruff.toml"* ]]; then bad "hinted typecheck: forbidden [ruff.toml] (Ruff config must not invalidate Ty)"; else ok; fi
if [[ "$python_lint_inputs" == *"ruff.toml"* ]]; then bad "unhinted lint: forbidden [ruff.toml] (unhinted uses default, shared change must not miss)"; else ok; fi
if [[ "$python_typecheck_inputs" == *"ruff.toml"* ]]; then bad "unhinted typecheck: forbidden [ruff.toml]"; else ok; fi

# Native-config isolation, Rust: hinted generated-shape format consumes
# rustfmt.toml while hinted lint and unhinted format do not. Changing the
# selected rustfmt config misses the consuming format pipeline only;
# clippy lint is unaffected per the pipeline-invalidation row.
rust_hinted_actions="$(bazel aquery '//quality/testdata:fixture_real_rust_generated_shape' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
if [[ -z "$rust_hinted_actions" ]]; then
  bad "rust hinted: empty aquery output"
else
  ok
fi
rust_hinted_lint_inputs="$(printf '%s' "$rust_hinted_actions" | grep -A 8 'Mnemonic: DxRealQualityLint' | grep 'Inputs:' | head -1 || true)"
rust_hinted_format_inputs="$(printf '%s' "$rust_hinted_actions" | grep -A 8 'Mnemonic: DxRealQualityFormat' | grep 'Inputs:' | head -1 || true)"
if [[ "$rust_hinted_format_inputs" == *"rustfmt.toml"* ]]; then ok; else bad "rust hinted format: want [rustfmt.toml] (selected rustfmt config is an input)"; fi
if [[ "$rust_hinted_lint_inputs" == *"rustfmt.toml"* ]]; then bad "rust hinted lint: forbidden [rustfmt.toml] (rustfmt config must not invalidate clippy lint)"; else ok; fi
if [[ "$rust_format_inputs" == *"rustfmt.toml"* ]]; then bad "rust unhinted format: forbidden [rustfmt.toml] (default config, selected change must not miss)"; else ok; fi
if [[ "$rust_lint_inputs" == *"rustfmt.toml"* ]]; then bad "rust unhinted lint: forbidden [rustfmt.toml]"; else ok; fi

# Native-config isolation, JavaScript: hinted lint/format consume
# biome.json while unhinted lint/format do not. Changing the selected
# Biome config misses consuming lint/format together (single Biome tool
# serves both capabilities) and nothing unhinted.
js_hinted_actions="$(bazel aquery '//quality/testdata:fixture_real_javascript_hinted' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
if [[ -z "$js_hinted_actions" ]]; then
  bad "js hinted: empty aquery output"
else
  ok
fi
js_actions="$(bazel aquery '//quality/testdata:fixture_real_javascript' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
if [[ -z "$js_actions" ]]; then
  bad "js unhinted: empty aquery output"
else
  ok
fi
js_hinted_lint_inputs="$(printf '%s' "$js_hinted_actions" | grep -A 8 'Mnemonic: DxRealQualityLint' | grep 'Inputs:' | head -1 || true)"
js_hinted_format_inputs="$(printf '%s' "$js_hinted_actions" | grep -A 8 'Mnemonic: DxRealQualityFormat' | grep 'Inputs:' | head -1 || true)"
js_lint_inputs="$(printf '%s' "$js_actions" | grep -A 8 'Mnemonic: DxRealQualityLint' | grep 'Inputs:' | head -1 || true)"
js_format_inputs="$(printf '%s' "$js_actions" | grep -A 8 'Mnemonic: DxRealQualityFormat' | grep 'Inputs:' | head -1 || true)"
if [[ "$js_hinted_lint_inputs" == *"biome.json"* ]]; then ok; else bad "js hinted lint: want [biome.json] (selected Biome config is an input)"; fi
if [[ "$js_hinted_format_inputs" == *"biome.json"* ]]; then ok; else bad "js hinted format: want [biome.json]"; fi
if [[ "$js_lint_inputs" == *"biome.json"* ]]; then bad "js unhinted lint: forbidden [biome.json] (default config, selected change must not miss)"; else ok; fi
if [[ "$js_format_inputs" == *"biome.json"* ]]; then bad "js unhinted format: forbidden [biome.json]"; else ok; fi

echo "quality cache aquery: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
