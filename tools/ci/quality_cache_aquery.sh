#!/usr/bin/env bash
# Quality cache aquery proof (issue #84, slices 1-14): per-adapter and
# per-capability action-key isolation via `bazel aquery` over real
# quality pipelines. All queries request lint+format+typecheck aspects
# explicitly so typecheck presence (python/rust/mixed) and absence
# (JS-family/corpus) prove real pipeline shape, never query shape; a
# fresh `bazel shutdown` still passes (no warm-cache dependence).
#
# The cache-correctness table in docs/quality/quality-testing.md requires,
# for each check kind, that one direct source misses only owning pipelines
# while unrelated files cause no miss, and that changing one tool
# invalidates only affected capability actions plus that changing one
# selected native config invalidates only its consuming capability
# actions and that changing an unselected adapter leaves keys unchanged.
# This harness proves the aquery action-shape half for Python
# (ruff lint/format, pydoclint lint, Ty typecheck + runner), Rust (clippy lint,
# rustfmt format + rustfmt.toml native-config isolation, rustc typecheck +
# runner), JavaScript
# (biome lint/format per-capability + biome.json native-config isolation,
# prettier never an input except JSON format, eslint/flake8/pylint opt-ins
# never inputs, target-coupled tsc never an input in default pipelines),
# Starlark
# (buildifier lint/format), TOML (taplo lint/format), Markdown
# (markdown_check + vale dual-tool lint with sorted stage order + sibling
# link-resolution inputs), TypeScript/JSX/TSX (biome lint/format each owning only
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
# JSON lint vs format split across biome vs prettier). Class-membership
# and stage-subset isolation holds via mixed multi-class unions
# (rust+starlark+toml single actions own all three sources+tools with
# sorted tool-ID stage order), capability-tag removal (no-lint drops
# lint, no-format drops format, no-typecheck drops typecheck), and provider-less plain targets
# emitting zero actions (unsupported classes leave keys unchanged);
# the apply step never appears as a Bazel action and the runner
# executable is an action input to every pipeline including typecheck
# (runner change invalidates the complete affected target/capability
# action, including markdown sibling lint); python lint stages run in
# sorted tool-ID order [pydoclint ruff] and markdown lint stages in
# [markdown_check vale] (stage-order policy change invalidates); markdown
# siblings resolve link targets only (sibling file is an input, never a
# stage source, and the sibling pipeline owns only its own sources);
# typecheck keys differ from lint/format and the manifest holds
# (python/rust 1 typecheck; JS/TS/JSX/TSX/JSON/Starlark/TOML lint+format
# only with exact 1+1 counts; markdown lint-only with exact 1 lint).
#
# Still open per #84 (recorded as gap, not claimed): full per-adapter
# table remainder (Go/Java/etc. adapters have no implementation yet;
# equivalent rows land with each adapter under #470-#475/#476-#484, enforced by the
# parity manifest + release_policy gate), transitive rows (quality actions
# take only direct sources by construction per QualitySourcesInfo
# validation, so transitive deps never enter inputs unless direct —
# proven by the unrelated-file no-miss checks above), tool-version rows
# (tools are direct file inputs above, so a version change is an input
# change by construction), pipeline invalidation remainder
# (stage-order/runner policy changes beyond canonical order), plus
# controlled remote-cache / separate-machine proof (requires remote
# infrastructure unavailable per docs/testing/README.md Remote Tests;
# tracked under #507, never claimed here). The execution-log half below
# distinguishes executed actions from cache hits locally; this harness is
# action-graph plus local execution-log, no remote execution.
#
# Run by CI via `bazel run //tools/ci:quality_cache_aquery`,
# after //tools/ci:examples_laziness_aquery.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

query_target() { # target -> aquery output
  local target="$1"
  bazel aquery "$target" \
    --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
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
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
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
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
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
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
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
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
if [[ -z "$js_hinted_actions" ]]; then
  bad "js hinted: empty aquery output"
else
  ok
fi
js_actions="$(bazel aquery '//quality/testdata:fixture_real_javascript' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
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
# JS per-capability owns-biome: both lint and format consume the Biome
# tool (biome change misses both), while prettier is never an input
# (prettier change misses nothing in JS, only JSON format).
if [[ "$js_lint_inputs" == *"biome"* ]]; then ok; else bad "js lint: want [biome] (biome change misses lint)"; fi
if [[ "$js_format_inputs" == *"biome"* ]]; then ok; else bad "js format: want [biome] (biome change misses format)"; fi
if [[ "$js_lint_inputs" == *"prettier"* ]]; then bad "js lint: forbidden [prettier] (prettier change must not invalidate JS lint)"; else ok; fi
if [[ "$js_format_inputs" == *"prettier"* ]]; then bad "js format: forbidden [prettier] (prettier change must not invalidate JS format)"; else ok; fi
# ESLint opt-in laziness (JS part): ESLint is an explicit opt-in lint
# adapter, so the default JS pipeline must never mention it (ESLint
# change leaves default keys unchanged per the unselected-adapter row).
if [[ "$js_actions" == *"eslint"* ]]; then bad "js: forbidden [eslint] (opt-in ESLint must not invalidate default JS)"; else ok; fi

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
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
toml_actions="$(bazel aquery '//quality/testdata:fixture_real_toml' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
markdown_actions="$(bazel aquery '//quality/testdata:fixture_real_markdown' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
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
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
jsx_actions="$(bazel aquery '//quality/testdata:fixture_real_jsx' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
tsx_actions="$(bazel aquery '//quality/testdata:fixture_real_tsx' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
json_actions="$(bazel aquery '//quality/testdata:fixture_real_json' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
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
# ESLint opt-in laziness remainder: default TS/JSX/TSX/JSON/Rust/corpus
# pipelines must never mention the opt-in ESLint adapter (ESLint change
# leaves default keys unchanged; python + JS parts already covered
# above). All variables defined here, after every aquery.
if [[ "$typescript_actions" == *"eslint"* ]]; then bad "typescript: forbidden [eslint] (opt-in ESLint must not invalidate default TS)"; else ok; fi
if [[ "$jsx_actions" == *"eslint"* ]]; then bad "jsx: forbidden [eslint] (opt-in ESLint must not invalidate default JSX)"; else ok; fi
if [[ "$tsx_actions" == *"eslint"* ]]; then bad "tsx: forbidden [eslint] (opt-in ESLint must not invalidate default TSX)"; else ok; fi
if [[ "$json_actions" == *"eslint"* ]]; then bad "json: forbidden [eslint] (opt-in ESLint must not invalidate default JSON)"; else ok; fi
if [[ "$rust_actions" == *"eslint"* ]]; then bad "rust: forbidden [eslint] (opt-in ESLint must not invalidate Rust)"; else ok; fi
if [[ "$starlark_actions" == *"eslint"* ]]; then bad "starlark: forbidden [eslint] (opt-in ESLint must not invalidate Starlark)"; else ok; fi
if [[ "$toml_actions" == *"eslint"* ]]; then bad "toml: forbidden [eslint] (opt-in ESLint must not invalidate TOML)"; else ok; fi
if [[ "$markdown_actions" == *"eslint"* ]]; then bad "markdown: forbidden [eslint] (opt-in ESLint must not invalidate Markdown)"; else ok; fi

# Class-membership + stage-source-subset + runner/stage-order/apply
# isolation: mixed multi-class targets union exact source subsets into
# one action per capability with sorted tool-ID stage order; capability
# tags drop only their owning pipeline; provider-less targets emit zero
# actions (unsupported classes leave keys unchanged); apply never
# appears as a Bazel action while the runner executable is always an
# input. Covers the class-membership and pipeline-invalidation rows
# toward the full table (supported-class manifests + transitive/
# tool-version still open).
mixed_actions="$(bazel aquery '//quality/testdata:fixture_real_mixed' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
no_lint_actions="$(bazel aquery '//quality/testdata:fixture_real_no_lint' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
no_format_actions="$(bazel aquery '//quality/testdata:fixture_real_no_format' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
no_typecheck_actions="$(bazel aquery '//quality/testdata:fixture_real_python_no_typecheck' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
plain_actions="$(bazel aquery '//quality/testdata:plain' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
mixed_lint_count="$(printf '%s' "$mixed_actions" | grep -c 'Mnemonic: DxRealQualityLint' || true)"
mixed_format_count="$(printf '%s' "$mixed_actions" | grep -c 'Mnemonic: DxRealQualityFormat' || true)"
no_lint_lint_count="$(printf '%s' "$no_lint_actions" | grep -c 'Mnemonic: DxRealQualityLint' || true)"
no_lint_format_count="$(printf '%s' "$no_lint_actions" | grep -c 'Mnemonic: DxRealQualityFormat' || true)"
no_format_lint_count="$(printf '%s' "$no_format_actions" | grep -c 'Mnemonic: DxRealQualityLint' || true)"
no_format_format_count="$(printf '%s' "$no_format_actions" | grep -c 'Mnemonic: DxRealQualityFormat' || true)"
no_typecheck_typecheck_count="$(printf '%s' "$no_typecheck_actions" | grep -c 'Mnemonic: DxRealQualityTypecheck' || true)"
python_typecheck_count="$(printf '%s' "$python_actions" | grep -c 'Mnemonic: DxRealQualityTypecheck' || true)"
plain_dx_count="$(printf '%s' "$plain_actions" | grep -c 'Mnemonic: DxRealQuality' || true)"
if [[ "$mixed_lint_count" == "1" ]]; then ok; else bad "mixed: want exactly 1 lint action (got $mixed_lint_count)"; fi
if [[ "$mixed_format_count" == "1" ]]; then ok; else bad "mixed: want exactly 1 format action (got $mixed_format_count)"; fi
if [[ "$no_lint_lint_count" == "0" ]]; then ok; else bad "no-lint: want 0 lint actions (tag drops owning pipeline, got $no_lint_lint_count)"; fi
if [[ "$no_lint_format_count" == "1" ]]; then ok; else bad "no-lint: want exactly 1 format action (got $no_lint_format_count)"; fi
if [[ "$no_format_lint_count" == "1" ]]; then ok; else bad "no-format: want exactly 1 lint action (got $no_format_lint_count)"; fi
if [[ "$no_format_format_count" == "0" ]]; then ok; else bad "no-format: want 0 format actions (tag drops owning pipeline, got $no_format_format_count)"; fi
if [[ "$no_typecheck_typecheck_count" == "0" ]]; then ok; else bad "no-typecheck: want 0 typecheck actions (got $no_typecheck_typecheck_count)"; fi
if [[ "$python_typecheck_count" == "1" ]]; then ok; else bad "python baseline: want 1 typecheck action (tag removal invalidates, got $python_typecheck_count)"; fi
if [[ "$plain_dx_count" == "0" ]]; then ok; else bad "plain: want 0 quality actions (unsupported with no provider leaves keys unchanged, got $plain_dx_count)"; fi
mixed_format_inputs="$(printf '%s' "$mixed_actions" | grep -A 8 'Mnemonic: DxRealQualityFormat' | grep 'Inputs:' | head -1 || true)"
no_lint_format_inputs="$(printf '%s' "$no_lint_actions" | grep -A 8 'Mnemonic: DxRealQualityFormat' | grep 'Inputs:' | head -1 || true)"
no_format_lint_inputs="$(printf '%s' "$no_format_actions" | grep -A 8 'Mnemonic: DxRealQualityLint' | grep 'Inputs:' | head -1 || true)"
if [[ "$mixed_format_inputs" == *"real_clean.toml"* && "$mixed_format_inputs" == *"taplo"* ]]; then ok; else bad "mixed format: want [real_clean.toml+taplo] (exact stage source subset unions all three classes)"; fi
if [[ "$no_lint_format_inputs" == *"real_clean.toml"* ]]; then bad "no-lint format: forbidden [real_clean.toml] (subset without TOML must not mention it)"; else ok; fi
if [[ "$no_lint_format_inputs" == *"taplo"* ]]; then bad "no-lint format: forbidden [taplo] (TOML tool must not invalidate subset without TOML)"; else ok; fi
if [[ "$no_format_lint_inputs" == *"real_clean.bzl"* && "$no_format_lint_inputs" == *"buildifier"* ]]; then ok; else bad "no-format lint: want [real_clean.bzl+buildifier] (exact stage source subset unions both classes)"; fi
if [[ "$no_format_lint_inputs" == *"real_clean.toml"* ]]; then bad "no-format lint: forbidden [real_clean.toml] (subset without TOML must not mention it)"; else ok; fi
if [[ "$no_format_lint_inputs" == *"taplo"* ]]; then bad "no-format lint: forbidden [taplo] (TOML tool must not invalidate subset without TOML)"; else ok; fi
if [[ "$no_format_lint_inputs" == *"rustfmt"* ]]; then bad "no-format lint: forbidden [rustfmt] (format tool must not invalidate lint)"; else ok; fi
if [[ "$no_format_lint_inputs" == *"clippy-driver"* ]]; then ok; else bad "no-format lint: want [clippy-driver] (lint tool change misses lint)"; fi
mixed_format_key="$(printf '%s' "$mixed_actions" | grep -A 10 "Dx real quality format //quality/testdata:fixture_real_mixed" | grep 'ActionKey:' | head -1 || true)"
mixed_lint_key="$(printf '%s' "$mixed_actions" | grep -A 10 "Dx real quality lint //quality/testdata:fixture_real_mixed" | grep 'ActionKey:' | head -1 || true)"
no_lint_format_key="$(printf '%s' "$no_lint_actions" | grep -A 10 "Dx real quality format //quality/testdata:fixture_real_no_lint" | grep 'ActionKey:' | head -1 || true)"
no_format_lint_key="$(printf '%s' "$no_format_actions" | grep -A 10 "Dx real quality lint //quality/testdata:fixture_real_no_format" | grep 'ActionKey:' | head -1 || true)"
if [[ -n "$mixed_format_key" && -n "$mixed_lint_key" && -n "$no_lint_format_key" && -n "$no_format_lint_key" ]]; then ok; else bad "want ActionKey lines in mixed/no-lint/no-format outputs"; fi
if [[ "$mixed_format_key" != "$no_lint_format_key" ]]; then ok; else bad "mixed vs no-lint format ActionKeys must differ (adding TOML class invalidates)"; fi
if [[ "$mixed_format_key" != "$mixed_lint_key" ]]; then ok; else bad "mixed lint vs format ActionKeys must differ (capability isolation holds multi-class)"; fi
if [[ "$mixed_lint_key" != "$no_format_lint_key" ]]; then ok; else bad "mixed vs no-format lint ActionKeys must differ (adding TOML class invalidates)"; fi
if [[ "$no_format_lint_key" != "$no_lint_format_key" ]]; then ok; else bad "no-format lint vs no-lint format ActionKeys must differ (capability isolation holds multi-class)"; fi
if [[ "$mixed_actions" == *"DxApply"* || "$no_lint_actions" == *"DxApply"* || "$no_format_actions" == *"DxApply"* || "$no_typecheck_actions" == *"DxApply"* || "$python_actions" == *"DxApply"* ]]; then bad "apply step must never appear as a Bazel action (apply never changes action keys)"; else ok; fi
if [[ "$mixed_format_inputs" == *"quality_runner"* && "$no_lint_format_inputs" == *"quality_runner"* && "$no_format_lint_inputs" == *"quality_runner"* ]]; then ok; else bad "want [quality_runner] executable in mixed/no-lint/no-format inputs (runner change invalidates)"; fi
# Runner-everywhere isolation: the runner executable is an input to every
# pipeline's actions, so changing the runner invalidates the complete
# affected target/capability action per the pipeline-invalidation row.
# Extends the mixed/no-lint runner presence above to the full per-adapter
# table (python/rust/js-family/corpus).
if [[ "$python_actions" == *"quality_runner"* ]]; then ok; else bad "python: want [quality_runner] in inputs (runner change invalidates)"; fi
if [[ "$rust_actions" == *"quality_runner"* ]]; then ok; else bad "rust: want [quality_runner] in inputs (runner change invalidates)"; fi
if [[ "$js_actions" == *"quality_runner"* ]]; then ok; else bad "js: want [quality_runner] in inputs (runner change invalidates)"; fi
if [[ "$typescript_actions" == *"quality_runner"* ]]; then ok; else bad "typescript: want [quality_runner] in inputs (runner change invalidates)"; fi
if [[ "$jsx_actions" == *"quality_runner"* ]]; then ok; else bad "jsx: want [quality_runner] in inputs (runner change invalidates)"; fi
if [[ "$tsx_actions" == *"quality_runner"* ]]; then ok; else bad "tsx: want [quality_runner] in inputs (runner change invalidates)"; fi
if [[ "$json_actions" == *"quality_runner"* ]]; then ok; else bad "json: want [quality_runner] in inputs (runner change invalidates)"; fi
if [[ "$starlark_actions" == *"quality_runner"* ]]; then ok; else bad "starlark: want [quality_runner] in inputs (runner change invalidates)"; fi
if [[ "$toml_actions" == *"quality_runner"* ]]; then ok; else bad "toml: want [quality_runner] in inputs (runner change invalidates)"; fi
if [[ "$markdown_actions" == *"quality_runner"* ]]; then ok; else bad "markdown: want [quality_runner] in inputs (runner change invalidates)"; fi
mixed_format_stages="$(printf '%s' "$mixed_actions" | grep -A 30 "Dx real quality format //quality/testdata:fixture_real_mixed" | grep -o "'[a-z_]*;[a-z_]*;" | tr '\n' ' ' || true)"
mixed_lint_stages="$(printf '%s' "$mixed_actions" | grep -A 30 "Dx real quality lint //quality/testdata:fixture_real_mixed" | grep -o "'[a-z_]*;[a-z_]*;" | tr '\n' ' ' || true)"
no_format_lint_stages="$(printf '%s' "$no_format_actions" | grep -A 30 "Dx real quality lint //quality/testdata:fixture_real_no_format" | grep -o "'[a-z_]*;[a-z_]*;" | tr '\n' ' ' || true)"
if [[ "$mixed_format_stages" == *"'buildifier;starlark;"*"'rustfmt;rust;"*"'taplo;toml;"* ]]; then ok; else bad "mixed format stages must be sorted tool-ID order [buildifier rustfmt taplo] (got $mixed_format_stages)"; fi
if [[ "$mixed_lint_stages" == *"'buildifier;starlark;"*"'clippy;rust;"*"'taplo;toml;"* ]]; then ok; else bad "mixed lint stages must be sorted tool-ID order [buildifier clippy taplo] (got $mixed_lint_stages)"; fi
if [[ "$no_format_lint_stages" == *"'buildifier;starlark;"*"'clippy;rust;"* ]]; then ok; else bad "no-format lint stages must be sorted tool-ID order [buildifier clippy] (got $no_format_lint_stages)"; fi
# Flake8/pylint opt-in laziness: both are explicit opt-in Python lint
# adapters, so default pipelines must never mention them (flake8/pylint
# change leaves default keys unchanged per the unselected-adapter +
# opt-in rows; eslint part already covered above). All variables defined
# here, after every aquery.
if [[ "$python_actions" == *"flake8"* ]]; then bad "python: forbidden [flake8] (opt-in flake8 must not invalidate default Python)"; else ok; fi
if [[ "$python_actions" == *"pylint"* ]]; then bad "python: forbidden [pylint] (opt-in pylint must not invalidate default Python)"; else ok; fi
if [[ "$js_actions" == *"flake8"* ]]; then bad "js: forbidden [flake8] (opt-in flake8 must not invalidate JS)"; else ok; fi
if [[ "$js_actions" == *"pylint"* ]]; then bad "js: forbidden [pylint] (opt-in pylint must not invalidate JS)"; else ok; fi
if [[ "$typescript_actions" == *"flake8"* ]]; then bad "typescript: forbidden [flake8] (opt-in flake8 must not invalidate TS)"; else ok; fi
if [[ "$typescript_actions" == *"pylint"* ]]; then bad "typescript: forbidden [pylint] (opt-in pylint must not invalidate TS)"; else ok; fi
if [[ "$jsx_actions" == *"flake8"* ]]; then bad "jsx: forbidden [flake8] (opt-in flake8 must not invalidate JSX)"; else ok; fi
if [[ "$jsx_actions" == *"pylint"* ]]; then bad "jsx: forbidden [pylint] (opt-in pylint must not invalidate JSX)"; else ok; fi
if [[ "$tsx_actions" == *"flake8"* ]]; then bad "tsx: forbidden [flake8] (opt-in flake8 must not invalidate TSX)"; else ok; fi
if [[ "$tsx_actions" == *"pylint"* ]]; then bad "tsx: forbidden [pylint] (opt-in pylint must not invalidate TSX)"; else ok; fi
if [[ "$json_actions" == *"flake8"* ]]; then bad "json: forbidden [flake8] (opt-in flake8 must not invalidate JSON)"; else ok; fi
if [[ "$json_actions" == *"pylint"* ]]; then bad "json: forbidden [pylint] (opt-in pylint must not invalidate JSON)"; else ok; fi
if [[ "$rust_actions" == *"flake8"* ]]; then bad "rust: forbidden [flake8] (opt-in flake8 must not invalidate Rust)"; else ok; fi
if [[ "$rust_actions" == *"pylint"* ]]; then bad "rust: forbidden [pylint] (opt-in pylint must not invalidate Rust)"; else ok; fi
if [[ "$starlark_actions" == *"flake8"* ]]; then bad "starlark: forbidden [flake8] (opt-in flake8 must not invalidate Starlark)"; else ok; fi
if [[ "$starlark_actions" == *"pylint"* ]]; then bad "starlark: forbidden [pylint] (opt-in pylint must not invalidate Starlark)"; else ok; fi
if [[ "$toml_actions" == *"flake8"* ]]; then bad "toml: forbidden [flake8] (opt-in flake8 must not invalidate TOML)"; else ok; fi
if [[ "$toml_actions" == *"pylint"* ]]; then bad "toml: forbidden [pylint] (opt-in pylint must not invalidate TOML)"; else ok; fi
if [[ "$markdown_actions" == *"flake8"* ]]; then bad "markdown: forbidden [flake8] (opt-in flake8 must not invalidate Markdown)"; else ok; fi
if [[ "$markdown_actions" == *"pylint"* ]]; then bad "markdown: forbidden [pylint] (opt-in pylint must not invalidate Markdown)"; else ok; fi
if [[ "$no_format_actions" == *"flake8"* ]]; then bad "no-format: forbidden [flake8] (opt-in flake8 must not invalidate no-format)"; else ok; fi
if [[ "$no_format_actions" == *"pylint"* ]]; then bad "no-format: forbidden [pylint] (opt-in pylint must not invalidate no-format)"; else ok; fi

# Class-membership opt-in + runner isolation: mixed/no-lint/no-format/
# no-typecheck pipelines must never mention opt-in adapters (eslint/
# flake8/pylint change leaves class-membership keys unchanged per the
# unselected-adapter + opt-in rows), and the no-typecheck lint/format
# actions still consume the runner executable (runner change misses
# them). All class-membership aqueries defined above.
if [[ "$mixed_actions" == *"eslint"* ]]; then bad "mixed: forbidden [eslint] (opt-in ESLint must not invalidate mixed)"; else ok; fi
if [[ "$no_lint_actions" == *"eslint"* ]]; then bad "no-lint: forbidden [eslint] (opt-in ESLint must not invalidate no-lint)"; else ok; fi
if [[ "$no_format_actions" == *"eslint"* ]]; then bad "no-format: forbidden [eslint] (opt-in ESLint must not invalidate no-format)"; else ok; fi
if [[ "$no_typecheck_actions" == *"eslint"* ]]; then bad "no-typecheck: forbidden [eslint] (opt-in ESLint must not invalidate no-typecheck)"; else ok; fi
if [[ "$mixed_actions" == *"flake8"* ]]; then bad "mixed: forbidden [flake8] (opt-in flake8 must not invalidate mixed)"; else ok; fi
if [[ "$mixed_actions" == *"pylint"* ]]; then bad "mixed: forbidden [pylint] (opt-in pylint must not invalidate mixed)"; else ok; fi
if [[ "$no_lint_actions" == *"flake8"* ]]; then bad "no-lint: forbidden [flake8] (opt-in flake8 must not invalidate no-lint)"; else ok; fi
if [[ "$no_lint_actions" == *"pylint"* ]]; then bad "no-lint: forbidden [pylint] (opt-in pylint must not invalidate no-lint)"; else ok; fi
if [[ "$no_typecheck_actions" == *"flake8"* ]]; then bad "no-typecheck: forbidden [flake8] (opt-in flake8 must not invalidate no-typecheck)"; else ok; fi
if [[ "$no_typecheck_actions" == *"pylint"* ]]; then bad "no-typecheck: forbidden [pylint] (opt-in pylint must not invalidate no-typecheck)"; else ok; fi
no_typecheck_lint_inputs="$(printf '%s' "$no_typecheck_actions" | grep -A 8 'Mnemonic: DxRealQualityLint' | grep 'Inputs:' | head -1 || true)"
no_typecheck_format_inputs="$(printf '%s' "$no_typecheck_actions" | grep -A 8 'Mnemonic: DxRealQualityFormat' | grep 'Inputs:' | head -1 || true)"
if [[ "$no_typecheck_lint_inputs" == *"quality_runner"* && "$no_typecheck_format_inputs" == *"quality_runner"* ]]; then ok; else bad "want [quality_runner] in no-typecheck lint/format inputs (runner change invalidates)"; fi

# Typecheck runner + per-capability isolation + supported-class manifest:
# rust typecheck owns rustc only (clippy/rustfmt changes miss it),
# both python/ty and rust/rustc typechecks consume the runner (runner
# change misses typecheck too), typecheck ActionKeys differ from
# lint/format (one source misses only its owning capability), and the
# manifest holds (python/rust 1 typecheck, JS-family/starlark/toml 0,
# markdown 0 format + 0 typecheck). All aqueries defined above; typecheck
# appears in the lint+format query because aquery includes all
# dx_results actions for the target (see slice 11 notes).
rust_typecheck_inputs="$(printf '%s' "$rust_actions" | grep -A 8 'Mnemonic: DxRealQualityTypecheck' | grep 'Inputs:' | head -1 || true)"
if [[ "$rust_typecheck_inputs" == *"rustc"* ]]; then ok; else bad "rust typecheck: want [rustc] (rustc change misses typecheck)"; fi
if [[ "$rust_typecheck_inputs" == *"clippy-driver"* ]]; then bad "rust typecheck: forbidden [clippy-driver] (clippy change must not invalidate typecheck)"; else ok; fi
if [[ "$rust_typecheck_inputs" == *"rustfmt"* ]]; then bad "rust typecheck: forbidden [rustfmt] (rustfmt change must not invalidate typecheck)"; else ok; fi
if [[ "$rust_typecheck_inputs" == *"quality_runner"* ]]; then ok; else bad "rust typecheck: want [quality_runner] in inputs (runner change invalidates)"; fi
if [[ "$python_typecheck_inputs" == *"quality_runner"* ]]; then ok; else bad "python typecheck: want [quality_runner] in inputs (runner change invalidates)"; fi
python_typecheck_key="$(printf '%s' "$python_actions" | grep -A 6 'Mnemonic: DxRealQualityTypecheck' | grep 'ActionKey:' | head -1 || true)"
python_lint_key="$(printf '%s' "$python_actions" | grep -A 6 'Mnemonic: DxRealQualityLint' | grep 'ActionKey:' | head -1 || true)"
python_format_key="$(printf '%s' "$python_actions" | grep -A 6 'Mnemonic: DxRealQualityFormat' | grep 'ActionKey:' | head -1 || true)"
rust_typecheck_key="$(printf '%s' "$rust_actions" | grep -A 6 'Mnemonic: DxRealQualityTypecheck' | grep 'ActionKey:' | head -1 || true)"
rust_lint_key="$(printf '%s' "$rust_actions" | grep -A 6 'Mnemonic: DxRealQualityLint' | grep 'ActionKey:' | head -1 || true)"
rust_format_key="$(printf '%s' "$rust_actions" | grep -A 6 'Mnemonic: DxRealQualityFormat' | grep 'ActionKey:' | head -1 || true)"
if [[ -n "$python_typecheck_key" && -n "$python_lint_key" && -n "$python_format_key" ]]; then ok; else bad "want ActionKey lines in python lint/format/typecheck"; fi
if [[ "$python_typecheck_key" != "$python_lint_key" && "$python_typecheck_key" != "$python_format_key" ]]; then ok; else bad "python typecheck ActionKey must differ from lint/format (capability isolation)"; fi
if [[ -n "$rust_typecheck_key" && -n "$rust_lint_key" && -n "$rust_format_key" ]]; then ok; else bad "want ActionKey lines in rust lint/format/typecheck"; fi
if [[ "$rust_typecheck_key" != "$rust_lint_key" && "$rust_typecheck_key" != "$rust_format_key" ]]; then ok; else bad "rust typecheck ActionKey must differ from lint/format (capability isolation)"; fi
markdown_format_count="$(printf '%s' "$markdown_actions" | grep -c 'Mnemonic: DxRealQualityFormat' || true)"
markdown_typecheck_count="$(printf '%s' "$markdown_actions" | grep -c 'Mnemonic: DxRealQualityTypecheck' || true)"
markdown_lint_count="$(printf '%s' "$markdown_actions" | grep -c 'Mnemonic: DxRealQualityLint' || true)"
js_typecheck_count="$(printf '%s' "$js_actions" | grep -c 'Mnemonic: DxRealQualityTypecheck' || true)"
js_lint_count="$(printf '%s' "$js_actions" | grep -c 'Mnemonic: DxRealQualityLint' || true)"
js_format_count="$(printf '%s' "$js_actions" | grep -c 'Mnemonic: DxRealQualityFormat' || true)"
typescript_lint_count="$(printf '%s' "$typescript_actions" | grep -c 'Mnemonic: DxRealQualityLint' || true)"
typescript_format_count="$(printf '%s' "$typescript_actions" | grep -c 'Mnemonic: DxRealQualityFormat' || true)"
typescript_typecheck_count="$(printf '%s' "$typescript_actions" | grep -c 'Mnemonic: DxRealQualityTypecheck' || true)"
jsx_lint_count="$(printf '%s' "$jsx_actions" | grep -c 'Mnemonic: DxRealQualityLint' || true)"
jsx_format_count="$(printf '%s' "$jsx_actions" | grep -c 'Mnemonic: DxRealQualityFormat' || true)"
jsx_typecheck_count="$(printf '%s' "$jsx_actions" | grep -c 'Mnemonic: DxRealQualityTypecheck' || true)"
tsx_lint_count="$(printf '%s' "$tsx_actions" | grep -c 'Mnemonic: DxRealQualityLint' || true)"
tsx_format_count="$(printf '%s' "$tsx_actions" | grep -c 'Mnemonic: DxRealQualityFormat' || true)"
tsx_typecheck_count="$(printf '%s' "$tsx_actions" | grep -c 'Mnemonic: DxRealQualityTypecheck' || true)"
json_lint_count="$(printf '%s' "$json_actions" | grep -c 'Mnemonic: DxRealQualityLint' || true)"
json_format_count="$(printf '%s' "$json_actions" | grep -c 'Mnemonic: DxRealQualityFormat' || true)"
json_typecheck_count="$(printf '%s' "$json_actions" | grep -c 'Mnemonic: DxRealQualityTypecheck' || true)"
starlark_lint_count="$(printf '%s' "$starlark_actions" | grep -c 'Mnemonic: DxRealQualityLint' || true)"
starlark_format_count="$(printf '%s' "$starlark_actions" | grep -c 'Mnemonic: DxRealQualityFormat' || true)"
starlark_typecheck_count="$(printf '%s' "$starlark_actions" | grep -c 'Mnemonic: DxRealQualityTypecheck' || true)"
toml_lint_count="$(printf '%s' "$toml_actions" | grep -c 'Mnemonic: DxRealQualityLint' || true)"
toml_format_count="$(printf '%s' "$toml_actions" | grep -c 'Mnemonic: DxRealQualityFormat' || true)"
toml_typecheck_count="$(printf '%s' "$toml_actions" | grep -c 'Mnemonic: DxRealQualityTypecheck' || true)"
if [[ "$markdown_format_count" == "0" ]]; then ok; else bad "markdown: want 0 format actions (manifest: lint-only, got $markdown_format_count)"; fi
if [[ "$markdown_typecheck_count" == "0" ]]; then ok; else bad "markdown: want 0 typecheck actions (manifest: lint-only, got $markdown_typecheck_count)"; fi
if [[ "$markdown_lint_count" == "1" ]]; then ok; else bad "markdown: want exactly 1 lint action (manifest: lint-only, got $markdown_lint_count)"; fi
if [[ "$js_typecheck_count" == "0" ]]; then ok; else bad "js: want 0 typecheck actions (manifest: lint+format only, got $js_typecheck_count)"; fi
if [[ "$js_lint_count" == "1" ]]; then ok; else bad "js: want exactly 1 lint action (manifest: lint+format only, got $js_lint_count)"; fi
if [[ "$js_format_count" == "1" ]]; then ok; else bad "js: want exactly 1 format action (manifest: lint+format only, got $js_format_count)"; fi
if [[ "$typescript_lint_count" == "1" ]]; then ok; else bad "typescript: want exactly 1 lint action (manifest: lint+format only, got $typescript_lint_count)"; fi
if [[ "$typescript_format_count" == "1" ]]; then ok; else bad "typescript: want exactly 1 format action (manifest: lint+format only, got $typescript_format_count)"; fi
if [[ "$typescript_typecheck_count" == "0" ]]; then ok; else bad "typescript: want 0 typecheck actions (manifest: lint+format only, got $typescript_typecheck_count)"; fi
if [[ "$jsx_lint_count" == "1" ]]; then ok; else bad "jsx: want exactly 1 lint action (manifest: lint+format only, got $jsx_lint_count)"; fi
if [[ "$jsx_format_count" == "1" ]]; then ok; else bad "jsx: want exactly 1 format action (manifest: lint+format only, got $jsx_format_count)"; fi
if [[ "$jsx_typecheck_count" == "0" ]]; then ok; else bad "jsx: want 0 typecheck actions (manifest: lint+format only, got $jsx_typecheck_count)"; fi
if [[ "$tsx_lint_count" == "1" ]]; then ok; else bad "tsx: want exactly 1 lint action (manifest: lint+format only, got $tsx_lint_count)"; fi
if [[ "$tsx_format_count" == "1" ]]; then ok; else bad "tsx: want exactly 1 format action (manifest: lint+format only, got $tsx_format_count)"; fi
if [[ "$tsx_typecheck_count" == "0" ]]; then ok; else bad "tsx: want 0 typecheck actions (manifest: lint+format only, got $tsx_typecheck_count)"; fi
if [[ "$json_lint_count" == "1" ]]; then ok; else bad "json: want exactly 1 lint action (manifest: lint+format split, got $json_lint_count)"; fi
if [[ "$json_format_count" == "1" ]]; then ok; else bad "json: want exactly 1 format action (manifest: lint+format split, got $json_format_count)"; fi
if [[ "$json_typecheck_count" == "0" ]]; then ok; else bad "json: want 0 typecheck actions (manifest: lint+format split, got $json_typecheck_count)"; fi
if [[ "$starlark_lint_count" == "1" ]]; then ok; else bad "starlark: want exactly 1 lint action (manifest: lint+format only, got $starlark_lint_count)"; fi
if [[ "$starlark_format_count" == "1" ]]; then ok; else bad "starlark: want exactly 1 format action (manifest: lint+format only, got $starlark_format_count)"; fi
if [[ "$starlark_typecheck_count" == "0" ]]; then ok; else bad "starlark: want 0 typecheck actions (manifest: lint+format only, got $starlark_typecheck_count)"; fi
if [[ "$toml_lint_count" == "1" ]]; then ok; else bad "toml: want exactly 1 lint action (manifest: lint+format only, got $toml_lint_count)"; fi
if [[ "$toml_format_count" == "1" ]]; then ok; else bad "toml: want exactly 1 format action (manifest: lint+format only, got $toml_format_count)"; fi
if [[ "$toml_typecheck_count" == "0" ]]; then ok; else bad "toml: want 0 typecheck actions (manifest: lint+format only, got $toml_typecheck_count)"; fi

# Markdown dual-tool + stage-order + sibling isolation: markdown lint owns
# both markdown_check (repo-owned link/structure) and vale (prose style)
# in one action with sorted tool-ID stage order [markdown_check vale], so
# changing either tool misses lint together; python lint stages run in
# sorted order [pydoclint ruff]; markdown siblings are link-resolution
# inputs only (never stage sources), and the sibling pipeline owns only
# its own sources with distinct ActionKeys (one direct source misses only
# its owning pipeline). Markdown_check change must not invalidate
# python/rust/js (unselected-adapter row for the repo-owned tool).
# Class-membership typecheck preservation: mixed/no-lint/no-format each
# carry exactly one rust typecheck (capability-tag removal drops only its
# owning lint/format pipeline, never typecheck); typecheck keys differ
# from lint/format (capability isolation holds multi-class for typecheck)
# and consume runner+rustc only (runner/rustc change misses typecheck;
# opt-in eslint/flake8/pylint never appear).
markdown_lint_inputs="$(printf '%s' "$markdown_actions" | grep -A 8 'Mnemonic: DxRealQualityLint' | grep 'Inputs:' | head -1 || true)"
if [[ "$markdown_lint_inputs" == *"quality_markdown"* ]]; then ok; else bad "markdown lint: want [quality_markdown] (markdown_check change misses lint)"; fi
if [[ "$markdown_lint_inputs" == *"vale"* ]]; then ok; else bad "markdown lint: want [vale] (vale change misses lint)"; fi
if [[ "$markdown_lint_inputs" == *"quality_runner"* ]]; then ok; else bad "markdown lint: want [quality_runner] in inputs (runner change invalidates)"; fi
markdown_lint_stages="$(printf '%s' "$markdown_actions" | grep -A 30 "Dx real quality lint //quality/testdata:fixture_real_markdown" | grep -o "'[a-z_]*;[a-z_]*;" | tr '\n' ' ' || true)"
if [[ "$markdown_lint_stages" == *"'markdown_check;markdown;"*"'vale;markdown;"* ]]; then ok; else bad "markdown lint stages must be sorted tool-ID order [markdown_check vale] (got $markdown_lint_stages)"; fi
python_lint_stages="$(printf '%s' "$python_actions" | grep -A 30 "Dx real quality lint //quality/testdata:fixture_real_python" | grep -o "'[a-z_]*;[a-z_]*;" | tr '\n' ' ' || true)"
if [[ "$python_lint_stages" == *"'pydoclint;python;"*"'ruff;python;"* ]]; then ok; else bad "python lint stages must be sorted tool-ID order [pydoclint ruff] (got $python_lint_stages)"; fi
if [[ "$python_actions" == *"markdown_check"* ]]; then bad "python: forbidden [markdown_check] (unselected repo-owned adapter must not invalidate Python)"; else ok; fi
if [[ "$rust_actions" == *"markdown_check"* ]]; then bad "rust: forbidden [markdown_check] (unselected repo-owned adapter must not invalidate Rust)"; else ok; fi
if [[ "$js_actions" == *"markdown_check"* ]]; then bad "js: forbidden [markdown_check] (unselected repo-owned adapter must not invalidate JS)"; else ok; fi
sibling_actions="$(bazel aquery '//quality/testdata:fixture_real_markdown_sibling' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_typecheck_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect,//quality:real_aspects.bzl%real_rust_typecheck_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
if [[ -z "$sibling_actions" ]]; then bad "sibling: empty aquery output"; else ok; fi
sibling_lint_inputs="$(printf '%s' "$sibling_actions" | grep -A 8 'Mnemonic: DxRealQualityLint' | grep 'Inputs:' | head -1 || true)"
if [[ "$sibling_lint_inputs" == *"sibling_clean.md"* ]]; then ok; else bad "sibling lint: want [sibling_clean.md] in inputs"; fi
if [[ "$sibling_lint_inputs" == *"sibling_license.txt"* ]]; then ok; else bad "sibling lint: want [sibling_license.txt] in inputs (sibling link target resolves)"; fi
if [[ "$sibling_lint_inputs" == *"quality_markdown"* && "$sibling_lint_inputs" == *"vale"* ]]; then ok; else bad "sibling lint: want [quality_markdown+vale] (both tools miss sibling lint)"; fi
if [[ "$sibling_lint_inputs" == *"quality_runner"* ]]; then ok; else bad "sibling lint: want [quality_runner] in inputs (runner change invalidates)"; fi
sibling_stages="$(printf '%s' "$sibling_actions" | grep -o "'[a-z_]*;markdown;[^']*'" | tr '\n' ' ' || true)"
if [[ "$sibling_stages" == *"sibling_clean.md"* ]]; then ok; else bad "sibling stages: want [sibling_clean.md] as stage source"; fi
if [[ "$sibling_stages" == *"sibling_license.txt"* ]]; then bad "sibling stages: forbidden [sibling_license.txt] as stage source (sibling never linted, input only)"; else ok; fi
if [[ "$markdown_actions" == *"sibling_clean.md"* ]]; then bad "markdown base: forbidden [sibling_clean.md] (sibling source must not invalidate base)"; else ok; fi
if [[ "$markdown_actions" == *"sibling_license.txt"* ]]; then bad "markdown base: forbidden [sibling_license.txt] (sibling input must not invalidate base)"; else ok; fi
sibling_key="$(printf '%s' "$sibling_actions" | grep 'ActionKey:' | head -1 || true)"
if [[ -n "$sibling_key" ]]; then ok; else bad "want ActionKey line in sibling output"; fi
if [[ -n "$sibling_key" && -n "$markdown_key" && "$sibling_key" != "$markdown_key" ]]; then ok; else bad "sibling vs base markdown ActionKeys must differ (one source misses only owning pipeline)"; fi

# Class-membership typecheck preservation: capability-tag removal drops
# only its owning lint/format pipeline, never typecheck. Mixed (rust+
# starlark+toml) carries 1 typecheck; no-lint (rust+starlark, no lint)
# and no-format (rust+starlark, no format) each preserve their single rust
# typecheck. Typecheck keys differ from lint/format (capability isolation
# holds multi-class for typecheck) and consume runner+rustc only.
mixed_typecheck_count="$(printf '%s' "$mixed_actions" | grep -c 'Mnemonic: DxRealQualityTypecheck' || true)"
no_lint_typecheck_count="$(printf '%s' "$no_lint_actions" | grep -c 'Mnemonic: DxRealQualityTypecheck' || true)"
no_format_typecheck_count="$(printf '%s' "$no_format_actions" | grep -c 'Mnemonic: DxRealQualityTypecheck' || true)"
if [[ "$mixed_typecheck_count" == "1" ]]; then ok; else bad "mixed: want exactly 1 typecheck action (capability-tag removal never drops typecheck, got $mixed_typecheck_count)"; fi
if [[ "$no_lint_typecheck_count" == "1" ]]; then ok; else bad "no-lint: want exactly 1 typecheck action (dropping lint preserves typecheck, got $no_lint_typecheck_count)"; fi
if [[ "$no_format_typecheck_count" == "1" ]]; then ok; else bad "no-format: want exactly 1 typecheck action (dropping format preserves typecheck, got $no_format_typecheck_count)"; fi
mixed_typecheck_key="$(printf '%s' "$mixed_actions" | grep -A 10 "Dx real quality typecheck //quality/testdata:fixture_real_mixed" | grep 'ActionKey:' | head -1 || true)"
no_lint_typecheck_key="$(printf '%s' "$no_lint_actions" | grep -A 10 "Dx real quality typecheck //quality/testdata:fixture_real_no_lint" | grep 'ActionKey:' | head -1 || true)"
no_format_typecheck_key="$(printf '%s' "$no_format_actions" | grep -A 10 "Dx real quality typecheck //quality/testdata:fixture_real_no_format" | grep 'ActionKey:' | head -1 || true)"
if [[ -n "$mixed_typecheck_key" && -n "$no_lint_typecheck_key" && -n "$no_format_typecheck_key" ]]; then ok; else bad "want ActionKey lines in mixed/no-lint/no-format typecheck outputs"; fi
if [[ "$mixed_typecheck_key" != "$mixed_lint_key" ]]; then ok; else bad "mixed typecheck vs lint ActionKeys must differ (capability isolation holds multi-class for typecheck)"; fi
if [[ "$mixed_typecheck_key" != "$mixed_format_key" ]]; then ok; else bad "mixed typecheck vs format ActionKeys must differ (capability isolation holds multi-class for typecheck)"; fi
if [[ "$no_lint_typecheck_key" != "$no_lint_format_key" ]]; then ok; else bad "no-lint typecheck vs format ActionKeys must differ (capability isolation holds when lint dropped)"; fi
if [[ "$no_format_typecheck_key" != "$no_format_lint_key" ]]; then ok; else bad "no-format typecheck vs lint ActionKeys must differ (capability isolation holds when format dropped)"; fi
mixed_typecheck_inputs="$(printf '%s' "$mixed_actions" | grep -A 8 'Mnemonic: DxRealQualityTypecheck' | grep 'Inputs:' | head -1 || true)"
no_lint_typecheck_inputs="$(printf '%s' "$no_lint_actions" | grep -A 8 'Mnemonic: DxRealQualityTypecheck' | grep 'Inputs:' | head -1 || true)"
no_format_typecheck_inputs="$(printf '%s' "$no_format_actions" | grep -A 8 'Mnemonic: DxRealQualityTypecheck' | grep 'Inputs:' | head -1 || true)"
if [[ "$mixed_typecheck_inputs" == *"quality_runner"* ]]; then ok; else bad "mixed typecheck: want [quality_runner] in inputs (runner change invalidates typecheck)"; fi
if [[ "$no_lint_typecheck_inputs" == *"quality_runner"* ]]; then ok; else bad "no-lint typecheck: want [quality_runner] in inputs (runner change invalidates typecheck)"; fi
if [[ "$no_format_typecheck_inputs" == *"quality_runner"* ]]; then ok; else bad "no-format typecheck: want [quality_runner] in inputs (runner change invalidates typecheck)"; fi
if [[ "$mixed_typecheck_inputs" == *"rustc"* ]]; then ok; else bad "mixed typecheck: want [rustc] (rustc change misses typecheck)"; fi
if [[ "$no_lint_typecheck_inputs" == *"rustc"* ]]; then ok; else bad "no-lint typecheck: want [rustc] (rustc change misses typecheck)"; fi
if [[ "$no_format_typecheck_inputs" == *"rustc"* ]]; then ok; else bad "no-format typecheck: want [rustc] (rustc change misses typecheck)"; fi
if [[ "$mixed_typecheck_inputs" == *"eslint"* || "$mixed_typecheck_inputs" == *"flake8"* || "$mixed_typecheck_inputs" == *"pylint"* ]]; then bad "mixed typecheck: forbidden [eslint/flake8/pylint] (opt-in change must not invalidate typecheck)"; else ok; fi
if [[ "$no_lint_typecheck_inputs" == *"eslint"* || "$no_lint_typecheck_inputs" == *"flake8"* || "$no_lint_typecheck_inputs" == *"pylint"* ]]; then bad "no-lint typecheck: forbidden [eslint/flake8/pylint] (opt-in change must not invalidate typecheck)"; else ok; fi
if [[ "$no_format_typecheck_inputs" == *"eslint"* || "$no_format_typecheck_inputs" == *"flake8"* || "$no_format_typecheck_inputs" == *"pylint"* ]]; then bad "no-format typecheck: forbidden [eslint/flake8/pylint] (opt-in change must not invalidate typecheck)"; else ok; fi

# Target-coupled tsc laziness (#408): tsc never applies from the class
# alone and never spawns a bare invocation (which would lose
# tsconfig/declaration context per quality/tools/typescript/BUILD.bazel).
# The aspect drops tsc stages: TS type safety is delegated to the upstream
# build (`transpiler = "tsc"` fails `bazel build //...` on type errors) plus
# `<name>_upstream_typecheck_test` under `bazel test //...`. Default
# QualitySourcesInfo-only pipelines must never mention tsc (tsc change
# leaves default keys unchanged per the unselected-adapter +
# target-coupled rows; `tsc` never collides with the `typescript` class
# name as a substring); fixtures prove the unfetched half here.
if [[ "$python_actions" == *"tsc"* ]]; then bad "python: forbidden [tsc] (target-coupled tsc must not invalidate default Python)"; else ok; fi
if [[ "$rust_actions" == *"tsc"* ]]; then bad "rust: forbidden [tsc] (target-coupled tsc must not invalidate Rust)"; else ok; fi
if [[ "$js_actions" == *"tsc"* ]]; then bad "js: forbidden [tsc] (target-coupled tsc must not invalidate JS)"; else ok; fi
if [[ "$typescript_actions" == *"tsc"* ]]; then bad "typescript: forbidden [tsc] (target-coupled tsc must not invalidate default TS fixtures)"; else ok; fi
if [[ "$jsx_actions" == *"tsc"* ]]; then bad "jsx: forbidden [tsc] (target-coupled tsc must not invalidate default JSX)"; else ok; fi
if [[ "$tsx_actions" == *"tsc"* ]]; then bad "tsx: forbidden [tsc] (target-coupled tsc must not invalidate default TSX fixtures)"; else ok; fi
if [[ "$json_actions" == *"tsc"* ]]; then bad "json: forbidden [tsc] (target-coupled tsc must not invalidate JSON)"; else ok; fi
if [[ "$starlark_actions" == *"tsc"* ]]; then bad "starlark: forbidden [tsc] (target-coupled tsc must not invalidate Starlark)"; else ok; fi
if [[ "$toml_actions" == *"tsc"* ]]; then bad "toml: forbidden [tsc] (target-coupled tsc must not invalidate TOML)"; else ok; fi
if [[ "$markdown_actions" == *"tsc"* ]]; then bad "markdown: forbidden [tsc] (target-coupled tsc must not invalidate Markdown)"; else ok; fi
if [[ "$sibling_actions" == *"tsc"* ]]; then bad "sibling: forbidden [tsc] (target-coupled tsc must not invalidate sibling)"; else ok; fi
if [[ "$mixed_actions" == *"tsc"* ]]; then bad "mixed: forbidden [tsc] (target-coupled tsc must not invalidate mixed)"; else ok; fi
if [[ "$no_lint_actions" == *"tsc"* ]]; then bad "no-lint: forbidden [tsc] (target-coupled tsc must not invalidate no-lint)"; else ok; fi
if [[ "$no_format_actions" == *"tsc"* ]]; then bad "no-format: forbidden [tsc] (target-coupled tsc must not invalidate no-format)"; else ok; fi
if [[ "$no_typecheck_actions" == *"tsc"* ]]; then bad "no-typecheck: forbidden [tsc] (target-coupled tsc must not invalidate no-typecheck)"; else ok; fi

# Execution-log proof (issue #84 close-out): aquery proves action shape;
# execution logs distinguish executed actions from cache hits locally. A
# warm local no-op alone is not a cache test per the contract, so force
# one re-execution by removing a single build output (gitignored
# bazel-bin, never the checkout), rebuild the Python lint aspect with an
# execution log (must execute with source+tool+runner in the log), then
# rebuild unchanged (log must be empty: cache hit, nothing executed).
# Controlled remote-cache / separate-machine proof stays tracked under
# #507 per docs/testing/README.md (infrastructure unavailable here).
exec_first="$(mktemp /tmp/quality_cache_exec_first.XXXXXX.json)"
exec_second="$(mktemp /tmp/quality_cache_exec_second.XXXXXX.json)"
lint_pb="bazel-bin/quality/testdata/fixture_real_python-real-lint.pb"
rm -f "$lint_pb"
if bazel build '//quality/testdata:fixture_real_python' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect \
  --output_groups=dx_results \
  --execution_log_json_file="$exec_first" \
  --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "exec-log: first Python lint aspect build must succeed"
fi
if [[ -s "$exec_first" ]]; then ok; else bad "exec-log: first build must execute (non-empty log), proving execution is recorded"; fi
if grep -q "real_clean.py" "$exec_first" 2>/dev/null; then ok; else bad "exec-log: first log must mention [real_clean.py] (direct source executed)"; fi
if grep -q "ruff" "$exec_first" 2>/dev/null; then ok; else bad "exec-log: first log must mention [ruff] (tool executed)"; fi
if grep -q "quality_runner" "$exec_first" 2>/dev/null; then ok; else bad "exec-log: first log must mention [quality_runner] (runner executed)"; fi
if bazel build '//quality/testdata:fixture_real_python' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect \
  --output_groups=dx_results \
  --execution_log_json_file="$exec_second" \
  --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "exec-log: second Python lint aspect build must succeed"
fi
if [[ ! -s "$exec_second" ]]; then ok; else bad "exec-log: second unchanged build must be a cache hit (empty log, nothing executed)"; fi
rm -f "$exec_first" "$exec_second"

dx_test_summary "quality cache aquery"
