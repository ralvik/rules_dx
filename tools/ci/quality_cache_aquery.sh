#!/usr/bin/env bash
# Quality cache aquery proof (issue #84, slices 1-6): per-adapter and
# per-capability action-key isolation via `bazel aquery` over real
# quality pipelines.
#
# The cache-correctness table in docs/quality/quality-testing.md requires,
# for each check kind, that one direct source misses only owning pipelines
# while unrelated files cause no miss, and that changing one tool
# invalidates only affected capability actions plus that changing one
# selected native config invalidates only its consuming capability
# actions and that changing an unselected adapter leaves keys unchanged.
# This harness proves the aquery action-shape half for Python
# (ruff lint/format, pydoclint lint, Ty typecheck), Rust (clippy lint,
# rustfmt format + rustfmt.toml native-config isolation), JavaScript
# (biome lint/format + biome.json native-config isolation), Starlark
# (buildifier lint/format), TOML (taplo lint/format), Markdown
# (vale lint), TypeScript/JSX/TSX (biome lint/format each owning only
# its source), and JSON (biome lint + prettier format split): each pipeline's declared Inputs mention its own source
# and tool and none of the other's, lint vs format vs typecheck
# ActionKeys differ, per-capability Inputs contain only their owning
# tool (ruff change misses lint/format but not typecheck; ty misses
# typecheck only; pydoclint misses lint only), native configs reach
# only consuming capabilities (ruff.toml misses hinted Python
# lint/format only; rustfmt.toml misses hinted Rust format only, never
# lint; biome.json misses hinted JS lint/format only, never unhinted
# pipelines), and unselected adapters never appear in Inputs
# (biome/eslint/prettier must not invalidate Python; ruff/ty must not
# invalidate JS; ruff/ty/biome must not invalidate Rust; buildifier/
# taplo/vale must not invalidate Python/Rust/JS and vice versa;
# TypeScript/JSX/TSX/JSON each own only their source+tool with
# JSON lint vs format split across biome vs prettier).
#
# Still open per #84 (recorded as gap, not claimed): full per-adapter
# table remainder (Go/Java/etc. + transitive/tool-version rows),
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

# Unselected-adapter isolation: changing an adapter not selected for a
# pipeline must leave its action key unchanged. The aquery half is that
# unselected tool binaries never appear in Inputs, so their change
# cannot invalidate the action. Python must not mention JS/Rust
# tooling; JS must not mention Python/Rust tooling; Rust must not
# mention Python/JS tooling.
if [[ "$python_actions" == *"biome"* ]]; then bad "python: forbidden [biome] (unselected JS adapter must not invalidate Python)"; else ok; fi
if [[ "$python_actions" == *"eslint"* ]]; then bad "python: forbidden [eslint] (unselected JS adapter must not invalidate Python)"; else ok; fi
if [[ "$python_actions" == *"prettier"* ]]; then bad "python: forbidden [prettier] (unselected JS adapter must not invalidate Python)"; else ok; fi
if [[ "$python_actions" == *"rustfmt"* ]]; then bad "python: forbidden [rustfmt] (unselected Rust adapter must not invalidate Python)"; else ok; fi
if [[ "$js_actions" == *"ruff"* ]]; then bad "js: forbidden [ruff] (unselected Python adapter must not invalidate JS)"; else ok; fi
if [[ "$js_actions" == *"dx_ty"* ]]; then bad "js: forbidden [dx_ty] (unselected Python typecheck must not invalidate JS)"; else ok; fi
if [[ "$js_actions" == *"pydoclint"* ]]; then bad "js: forbidden [pydoclint] (unselected Python adapter must not invalidate JS)"; else ok; fi
if [[ "$js_actions" == *"clippy-driver"* ]]; then bad "js: forbidden [clippy-driver] (unselected Rust adapter must not invalidate JS)"; else ok; fi
if [[ "$rust_actions" == *"ruff"* ]]; then bad "rust: forbidden [ruff] (unselected Python adapter must not invalidate Rust)"; else ok; fi
if [[ "$rust_actions" == *"dx_ty"* ]]; then bad "rust: forbidden [dx_ty] (unselected Python typecheck must not invalidate Rust)"; else ok; fi
if [[ "$rust_actions" == *"biome"* ]]; then bad "rust: forbidden [biome] (unselected JS adapter must not invalidate Rust)"; else ok; fi

# Starlark/TOML/Markdown isolation: each target-less corpus adapter owns
# only its source and tool. Changing buildifier/taplo/vale must miss
# only its owning pipeline; Python/Rust/JS changes must not miss them
# and vice versa. Extends the unselected-adapter row toward the full
# per-adapter table (Go/Java/etc. + transitive/tool-version still open).
starlark_actions="$(bazel aquery '//quality/testdata:fixture_real_starlark' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
toml_actions="$(bazel aquery '//quality/testdata:fixture_real_toml' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
markdown_actions="$(bazel aquery '//quality/testdata:fixture_real_markdown' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
if [[ -z "$starlark_actions" ]]; then bad "starlark: empty aquery output"; else ok; fi
if [[ -z "$toml_actions" ]]; then bad "toml: empty aquery output"; else ok; fi
if [[ -z "$markdown_actions" ]]; then bad "markdown: empty aquery output"; else ok; fi
if [[ "$starlark_actions" == *"real_clean.bzl"* ]]; then ok; else bad "starlark: want [real_clean.bzl] in inputs"; fi
if [[ "$starlark_actions" == *"buildifier"* ]]; then ok; else bad "starlark: want [buildifier] in inputs"; fi
if [[ "$toml_actions" == *"real_clean.toml"* ]]; then ok; else bad "toml: want [real_clean.toml] in inputs"; fi
if [[ "$toml_actions" == *"taplo"* ]]; then ok; else bad "toml: want [taplo] in inputs"; fi
if [[ "$markdown_actions" == *"real_clean.md"* ]]; then ok; else bad "markdown: want [real_clean.md] in inputs"; fi
if [[ "$markdown_actions" == *"vale"* ]]; then ok; else bad "markdown: want [vale] in inputs"; fi
# Own-tool exclusivity: starlark must not mention python/rust/js tools.
if [[ "$starlark_actions" == *"ruff"* ]]; then bad "starlark: forbidden [ruff]"; else ok; fi
if [[ "$starlark_actions" == *"dx_ty"* ]]; then bad "starlark: forbidden [dx_ty]"; else ok; fi
if [[ "$starlark_actions" == *"biome"* ]]; then bad "starlark: forbidden [biome]"; else ok; fi
if [[ "$starlark_actions" == *"clippy-driver"* ]]; then bad "starlark: forbidden [clippy-driver]"; else ok; fi
if [[ "$toml_actions" == *"ruff"* ]]; then bad "toml: forbidden [ruff]"; else ok; fi
if [[ "$toml_actions" == *"biome"* ]]; then bad "toml: forbidden [biome]"; else ok; fi
if [[ "$toml_actions" == *"buildifier"* ]]; then bad "toml: forbidden [buildifier] (starlark tool must not invalidate TOML)"; else ok; fi
if [[ "$markdown_actions" == *"ruff"* ]]; then bad "markdown: forbidden [ruff]"; else ok; fi
if [[ "$markdown_actions" == *"biome"* ]]; then bad "markdown: forbidden [biome]"; else ok; fi
if [[ "$markdown_actions" == *"buildifier"* ]]; then bad "markdown: forbidden [buildifier]"; else ok; fi
if [[ "$markdown_actions" == *"taplo"* ]]; then bad "markdown: forbidden [taplo]"; else ok; fi
# Reverse: python/rust/js must not mention corpus-adapter tools.
if [[ "$python_actions" == *"buildifier"* ]]; then bad "python: forbidden [buildifier]"; else ok; fi
if [[ "$python_actions" == *"taplo"* ]]; then bad "python: forbidden [taplo]"; else ok; fi
if [[ "$python_actions" == *"vale"* ]]; then bad "python: forbidden [vale]"; else ok; fi
if [[ "$rust_actions" == *"buildifier"* ]]; then bad "rust: forbidden [buildifier]"; else ok; fi
if [[ "$rust_actions" == *"taplo"* ]]; then bad "rust: forbidden [taplo]"; else ok; fi
if [[ "$rust_actions" == *"vale"* ]]; then bad "rust: forbidden [vale]"; else ok; fi
if [[ "$js_actions" == *"buildifier"* ]]; then bad "js: forbidden [buildifier]"; else ok; fi
if [[ "$js_actions" == *"taplo"* ]]; then bad "js: forbidden [taplo]"; else ok; fi
if [[ "$js_actions" == *"vale"* ]]; then bad "js: forbidden [vale]"; else ok; fi
# ActionKeys differ across corpus adapters, so one direct source misses
# only its owning pipeline while unrelated files cause no miss.
starlark_key="$(printf '%s' "$starlark_actions" | grep 'ActionKey:' | head -1 || true)"
toml_key="$(printf '%s' "$toml_actions" | grep 'ActionKey:' | head -1 || true)"
markdown_key="$(printf '%s' "$markdown_actions" | grep 'ActionKey:' | head -1 || true)"
if [[ -n "$starlark_key" && -n "$toml_key" && -n "$markdown_key" ]]; then ok; else bad "want ActionKey lines in starlark/toml/markdown outputs"; fi
if [[ "$starlark_key" != "$toml_key" ]]; then ok; else bad "starlark vs toml ActionKeys must differ"; fi
if [[ "$starlark_key" != "$markdown_key" ]]; then ok; else bad "starlark vs markdown ActionKeys must differ"; fi
if [[ "$toml_key" != "$markdown_key" ]]; then ok; else bad "toml vs markdown ActionKeys must differ"; fi
if [[ "$starlark_key" != "$python_key" ]]; then ok; else bad "starlark vs python ActionKeys must differ"; fi

# TypeScript/JSX/TSX/JSON isolation: JS-family expansion shares the Biome
# tool across JS/TS/JSX/TSX/JSON-lint but each pipeline owns only its
# source; JSON format owns prettier instead (biome lint vs prettier
# format per-capability split). Changing one JS-family source misses
# only its owning pipeline; changing prettier misses JSON format only
# while biome misses lint only; Python/Rust/corpus changes must not
# miss JS-family pipelines and vice versa. Extends the per-adapter
# table toward Go/Java/etc. (transitive/tool-version still open).
typescript_actions="$(bazel aquery '//quality/testdata:fixture_real_typescript' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
jsx_actions="$(bazel aquery '//quality/testdata:fixture_real_jsx' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
tsx_actions="$(bazel aquery '//quality/testdata:fixture_real_tsx' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
json_actions="$(bazel aquery '//quality/testdata:fixture_real_json' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
if [[ -z "$typescript_actions" ]]; then bad "typescript: empty aquery output"; else ok; fi
if [[ -z "$jsx_actions" ]]; then bad "jsx: empty aquery output"; else ok; fi
if [[ -z "$tsx_actions" ]]; then bad "tsx: empty aquery output"; else ok; fi
if [[ -z "$json_actions" ]]; then bad "json: empty aquery output"; else ok; fi
if [[ "$typescript_actions" == *"real_clean.ts"* ]]; then ok; else bad "typescript: want [real_clean.ts] in inputs"; fi
if [[ "$typescript_actions" == *"biome"* ]]; then ok; else bad "typescript: want [biome] in inputs"; fi
if [[ "$jsx_actions" == *"real_clean.jsx"* ]]; then ok; else bad "jsx: want [real_clean.jsx] in inputs"; fi
if [[ "$jsx_actions" == *"biome"* ]]; then ok; else bad "jsx: want [biome] in inputs"; fi
if [[ "$tsx_actions" == *"real_clean.tsx"* ]]; then ok; else bad "tsx: want [real_clean.tsx] in inputs"; fi
if [[ "$tsx_actions" == *"biome"* ]]; then ok; else bad "tsx: want [biome] in inputs"; fi
if [[ "$json_actions" == *"real_clean.json"* ]]; then ok; else bad "json: want [real_clean.json] in inputs"; fi
if [[ "$json_actions" == *"biome"* ]]; then ok; else bad "json: want [biome] in inputs (lint owns biome)"; fi
if [[ "$json_actions" == *"prettier"* ]]; then ok; else bad "json: want [prettier] in inputs (format owns prettier)"; fi
# Own-tool exclusivity: TS/JSX/TSX must not mention python/rust/corpus
# tools nor the JSON-format prettier split; JSON must not mention
# python/rust/corpus tools.
if [[ "$typescript_actions" == *"ruff"* ]]; then bad "typescript: forbidden [ruff]"; else ok; fi
if [[ "$typescript_actions" == *"clippy-driver"* ]]; then bad "typescript: forbidden [clippy-driver]"; else ok; fi
if [[ "$typescript_actions" == *"prettier"* ]]; then bad "typescript: forbidden [prettier] (prettier change must not invalidate TS)"; else ok; fi
if [[ "$typescript_actions" == *"buildifier"* ]]; then bad "typescript: forbidden [buildifier]"; else ok; fi
if [[ "$typescript_actions" == *"taplo"* ]]; then bad "typescript: forbidden [taplo]"; else ok; fi
if [[ "$typescript_actions" == *"vale"* ]]; then bad "typescript: forbidden [vale]"; else ok; fi
if [[ "$jsx_actions" == *"ruff"* ]]; then bad "jsx: forbidden [ruff]"; else ok; fi
if [[ "$jsx_actions" == *"clippy-driver"* ]]; then bad "jsx: forbidden [clippy-driver]"; else ok; fi
if [[ "$jsx_actions" == *"prettier"* ]]; then bad "jsx: forbidden [prettier]"; else ok; fi
if [[ "$jsx_actions" == *"buildifier"* ]]; then bad "jsx: forbidden [buildifier]"; else ok; fi
if [[ "$tsx_actions" == *"ruff"* ]]; then bad "tsx: forbidden [ruff]"; else ok; fi
if [[ "$tsx_actions" == *"clippy-driver"* ]]; then bad "tsx: forbidden [clippy-driver]"; else ok; fi
if [[ "$tsx_actions" == *"prettier"* ]]; then bad "tsx: forbidden [prettier]"; else ok; fi
if [[ "$tsx_actions" == *"taplo"* ]]; then bad "tsx: forbidden [taplo]"; else ok; fi
if [[ "$json_actions" == *"ruff"* ]]; then bad "json: forbidden [ruff]"; else ok; fi
if [[ "$json_actions" == *"clippy-driver"* ]]; then bad "json: forbidden [clippy-driver]"; else ok; fi
if [[ "$json_actions" == *"buildifier"* ]]; then bad "json: forbidden [buildifier]"; else ok; fi
if [[ "$json_actions" == *"taplo"* ]]; then bad "json: forbidden [taplo]"; else ok; fi
if [[ "$json_actions" == *"vale"* ]]; then bad "json: forbidden [vale]"; else ok; fi
# Per-capability JSON split: lint owns biome only, format owns prettier
# only, so changing one misses only its owning capability.
json_lint_inputs="$(printf '%s' "$json_actions" | grep -A 8 'Mnemonic: DxRealQualityLint' | grep 'Inputs:' | head -1 || true)"
json_format_inputs="$(printf '%s' "$json_actions" | grep -A 8 'Mnemonic: DxRealQualityFormat' | grep 'Inputs:' | head -1 || true)"
if [[ "$json_lint_inputs" == *"biome"* ]]; then ok; else bad "json lint: want [biome]"; fi
if [[ "$json_lint_inputs" == *"prettier"* ]]; then bad "json lint: forbidden [prettier] (prettier change must not invalidate lint)"; else ok; fi
if [[ "$json_format_inputs" == *"prettier"* ]]; then ok; else bad "json format: want [prettier]"; fi
if [[ "$json_format_inputs" == *"biome"* ]]; then bad "json format: forbidden [biome] (biome change must not invalidate format)"; else ok; fi
# Reverse: existing pipelines must not mention JS-family sources.
if [[ "$python_actions" == *"real_clean.ts"* ]]; then bad "python: forbidden [real_clean.ts]"; else ok; fi
if [[ "$python_actions" == *"real_clean.json"* ]]; then bad "python: forbidden [real_clean.json]"; else ok; fi
if [[ "$rust_actions" == *"real_clean.ts"* ]]; then bad "rust: forbidden [real_clean.ts]"; else ok; fi
if [[ "$rust_actions" == *"real_clean.json"* ]]; then bad "rust: forbidden [real_clean.json]"; else ok; fi
if [[ "$js_actions" == *"real_clean.ts"* ]]; then bad "js: forbidden [real_clean.ts] (TS source must not invalidate JS)"; else ok; fi
if [[ "$js_actions" == *"real_clean.json"* ]]; then bad "js: forbidden [real_clean.json] (JSON source must not invalidate JS)"; else ok; fi
# ActionKeys differ across JS-family pipelines, so one direct source
# misses only its owning pipeline while unrelated files cause no miss.
typescript_key="$(printf '%s' "$typescript_actions" | grep 'ActionKey:' | head -1 || true)"
jsx_key="$(printf '%s' "$jsx_actions" | grep 'ActionKey:' | head -1 || true)"
tsx_key="$(printf '%s' "$tsx_actions" | grep 'ActionKey:' | head -1 || true)"
json_key="$(printf '%s' "$json_actions" | grep 'ActionKey:' | head -1 || true)"
if [[ -n "$typescript_key" && -n "$jsx_key" && -n "$tsx_key" && -n "$json_key" ]]; then ok; else bad "want ActionKey lines in typescript/jsx/tsx/json outputs"; fi
if [[ "$typescript_key" != "$jsx_key" ]]; then ok; else bad "typescript vs jsx ActionKeys must differ"; fi
if [[ "$typescript_key" != "$tsx_key" ]]; then ok; else bad "typescript vs tsx ActionKeys must differ"; fi
if [[ "$typescript_key" != "$json_key" ]]; then ok; else bad "typescript vs json ActionKeys must differ"; fi
if [[ "$jsx_key" != "$tsx_key" ]]; then ok; else bad "jsx vs tsx ActionKeys must differ"; fi
if [[ "$json_key" != "$python_key" ]]; then ok; else bad "json vs python ActionKeys must differ"; fi

echo "quality cache aquery: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
