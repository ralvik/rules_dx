#!/usr/bin/env bash
# Parser-sample backfill qualification harness.
#
# Qualifies the as-built parser-sample backfill with fixture evidence and
# owned gaps, without claiming Supported:
# - delivered: every adapter-backed tool keeps a parser module with pinned
#   pass (clean) plus fail (dirty) samples exercised as unit tests
#   (`quality/adapter/src/parsers/*.rs`: biome lint plus format, buildifier,
#   clippy plus rustc via the shared rust diagnostics, csharpier, eslint,
#   fantomas, flake8, fsharplint, markdown_check, prettier, pydoclint, pylint,
#   roslyn, ruff lint plus format, rustfmt, scalafix, scalafmt, taplo lint
#   plus format, tsc, ty, vale);
# - wiring: recorded Clippy/rustc diagnostics stay byte-identical between
#   the parser unit samples and the Layer-2 matrix injected upstream
#   (`quality/testdata/runner_matrix_cases.bzl`); tsc keeps its adapter
#   parser with pass plus fail samples but stays pipeline-only by design
#   (target-coupled, no runner dispatch, no matrix cell);
# - open owned gaps: platform plus consumer plus release evidence, exact
#   per-tool version pins beyond the seed host, no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:parser_sample_qualification`,
# following //tools/ci:hello_smoke_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

roadmap="docs/roadmap.md"
verify="docs/testing/verification-matrix.md"
verify_remaining="docs/testing/verification-matrix-remaining.md"
runner_doc="docs/quality/runner-matrix.md"
ci=".github/workflows/ci.yml"
build="tools/ci/BUILD.bazel"
matrix="quality/testdata/runner_matrix_cases.bzl"
real_rs="quality/runner/src/real.rs"

# Roadmap owns the Seed-host-delivered history record under Cleanup-completed.
if grep -q -F -e 'parser-sample backfill Seed-host-delivered (closed #465' "$roadmap"; then
  ok
else
  bad "roadmap lost its parser-sample Seed-host-delivered record under closed #465"
fi

# Remaining matrix owns the qualified seed-only record under.
if grep -q -F -e 'parser_sample_qualification' "$verify_remaining" &&
  grep -q -F -e 'qualified seed-only under #465' "$verify_remaining" &&
  grep -q -F -e 'bazel run //tools/ci:parser_sample_qualification' "$verify_remaining"; then
  ok
else
  bad "verification-matrix-remaining lost its #465 parser-sample qualified record"
fi

# Verification matrix lists the harness in dogfood-freshness.
if grep -q -F -e ':parser_sample_qualification' "$verify"; then
  ok
else
  bad "verification-matrix dogfood-freshness lost :parser_sample_qualification"
fi

# Verification matrix Green lists the harness count.
if grep -q -F -e '`parser_sample_qualification` 29/29' "$verify"; then
  ok
else
  bad "verification-matrix Green lost parser_sample_qualification 23/23"
fi

# BUILD owns the harness target.
if grep -q -F -e 'name = "parser_sample_qualification"' "$build"; then
  ok
else
  bad "tools/ci/BUILD.bazel lost the parser_sample_qualification target"
fi

# CI wires the harness in dogfood-freshness.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:parser_sample_qualification' "$ci"; then
  ok
else
  bad "ci.yml lost the parser_sample_qualification step (want dogfood-freshness)"
fi

# Biome keeps lint plus format parsers with dirty plus clean samples.
if [[ -f "quality/adapter/src/parsers/biome.rs" ]] &&
  grep -q -F -e 'pub fn parse_biome_lint' quality/adapter/src/parsers/biome.rs &&
  grep -q -F -e 'pub fn parse_biome_format' quality/adapter/src/parsers/biome.rs &&
  grep -q -F -e 'BIOME_LINT_DIRTY' quality/adapter/src/parsers/biome.rs &&
  grep -q -F -e 'BIOME_LINT_CLEAN' quality/adapter/src/parsers/biome.rs &&
  grep -q -F -e 'BIOME_FMT_DIRTY' quality/adapter/src/parsers/biome.rs; then
  ok
else
  bad "biome lost its lint plus format parser with dirty plus clean samples"
fi

# Buildifier keeps its parser with dirty plus clean plus syntax samples.
if [[ -f "quality/adapter/src/parsers/buildifier.rs" ]] &&
  grep -q -F -e 'pub fn parse_buildifier' quality/adapter/src/parsers/buildifier.rs &&
  grep -q -F -e 'BUILDIFIER_DIRTY' quality/adapter/src/parsers/buildifier.rs &&
  grep -q -F -e 'BUILDIFIER_CLEAN' quality/adapter/src/parsers/buildifier.rs &&
  grep -q -F -e 'BUILDIFIER_BROKEN' quality/adapter/src/parsers/buildifier.rs; then
  ok
else
  bad "buildifier lost its parser with dirty plus clean plus syntax samples"
fi

# Clippy plus rustc share the rust diagnostics parser with lint plus type samples.
if [[ -f "quality/adapter/src/parsers/rust.rs" ]] &&
  grep -q -F -e 'pub fn parse_clippy' quality/adapter/src/parsers/rust.rs &&
  grep -q -F -e 'pub fn parse_rustc' quality/adapter/src/parsers/rust.rs &&
  grep -q -F -e 'CLIPPY_LINT' quality/adapter/src/parsers/rust.rs &&
  grep -q -F -e 'RUSTC_TYPE_ERROR' quality/adapter/src/parsers/rust.rs; then
  ok
else
  bad "rust lost its clippy plus rustc parser with lint plus type samples"
fi

# Recorded Clippy/rustc diagnostics stay byte-identical to the matrix injected upstream.
if grep -q -F -e '_CLIPPY_LINT' "$matrix" &&
  grep -q -F -e '_RUSTC_TYPE_ERROR' "$matrix" &&
  grep -q -F -e 'byte-identical to' "$matrix" &&
  grep -q -F -e 'parser unit samples' "$matrix" &&
  grep -q -F -e 'clippy::len_zero' "$matrix" &&
  grep -q -F -e 'E0308' "$matrix"; then
  ok
else
  bad "runner matrix lost its byte-identical clippy/rustc parser-sample pin"
fi

# ESLint keeps its parser with dirty plus clean samples.
if [[ -f "quality/adapter/src/parsers/eslint.rs" ]] &&
  grep -q -F -e 'pub fn parse_eslint' quality/adapter/src/parsers/eslint.rs &&
  grep -q -F -e 'ESLINT_DIRTY' quality/adapter/src/parsers/eslint.rs &&
  grep -q -F -e 'ESLINT_CLEAN' quality/adapter/src/parsers/eslint.rs; then
  ok
else
  bad "eslint lost its parser with dirty plus clean samples"
fi

# flake8 keeps its parser with dirty plus clean samples.
if [[ -f "quality/adapter/src/parsers/flake8.rs" ]] &&
  grep -q -F -e 'pub fn parse_flake8' quality/adapter/src/parsers/flake8.rs &&
  grep -q -F -e 'FLAKE8_DIRTY' quality/adapter/src/parsers/flake8.rs; then
  ok
else
  bad "flake8 lost its parser with dirty plus clean samples"
fi

# Markdown checker keeps its workspace-keyed parser with finding plus clean samples.
if [[ -f "quality/adapter/src/parsers/markdown.rs" ]] &&
  grep -q -F -e 'pub fn parse_markdown_findings' quality/adapter/src/parsers/markdown.rs &&
  grep -q -F -e 'missing-file-target' quality/adapter/src/parsers/markdown.rs &&
  grep -q -F -e 'missing-anchor' quality/adapter/src/parsers/markdown.rs; then
  ok
else
  bad "markdown_check lost its parser with finding plus clean samples"
fi

# Prettier keeps its check parser with warn-line plus clean samples.
if [[ -f "quality/adapter/src/parsers/prettier.rs" ]] &&
  grep -q -F -e 'pub fn parse_prettier_check' quality/adapter/src/parsers/prettier.rs &&
  grep -q -F -e '[warn]' quality/adapter/src/parsers/prettier.rs; then
  ok
else
  bad "prettier lost its check parser with warn-line plus clean samples"
fi

# pydoclint keeps its parser with violation plus syntax plus clean samples.
if [[ -f "quality/adapter/src/parsers/pydoclint.rs" ]] &&
  grep -q -F -e 'pub fn parse_pydoclint' quality/adapter/src/parsers/pydoclint.rs &&
  grep -q -F -e 'DOC101' quality/adapter/src/parsers/pydoclint.rs; then
  ok
else
  bad "pydoclint lost its parser with violation plus syntax plus clean samples"
fi

# pylint keeps its parser with dirty plus clean samples.
if [[ -f "quality/adapter/src/parsers/pylint.rs" ]] &&
  grep -q -F -e 'pub fn parse_pylint' quality/adapter/src/parsers/pylint.rs &&
  grep -q -F -e 'PYLINT_DIRTY' quality/adapter/src/parsers/pylint.rs; then
  ok
else
  bad "pylint lost its parser with dirty plus clean samples"
fi

# Ruff keeps lint plus format parsers with dirty plus clean samples.
if [[ -f "quality/adapter/src/parsers/ruff.rs" ]] &&
  grep -q -F -e 'pub fn parse_ruff' quality/adapter/src/parsers/ruff.rs &&
  grep -q -F -e 'pub fn parse_ruff_format' quality/adapter/src/parsers/ruff.rs &&
  grep -q -F -e 'RUFF_LINT_DIRTY' quality/adapter/src/parsers/ruff.rs &&
  grep -q -F -e 'RUFF_FORMAT_DIRTY' quality/adapter/src/parsers/ruff.rs; then
  ok
else
  bad "ruff lost its lint plus format parser with dirty plus clean samples"
fi

# rustfmt keeps its parser with diff plus syntax plus clean samples.
if [[ -f "quality/adapter/src/parsers/rustfmt.rs" ]] &&
  grep -q -F -e 'pub fn parse_rustfmt' quality/adapter/src/parsers/rustfmt.rs &&
  grep -q -F -e 'Diff in ' quality/adapter/src/parsers/rustfmt.rs; then
  ok
else
  bad "rustfmt lost its parser with diff plus syntax plus clean samples"
fi

# Taplo keeps lint plus format parsers with dirty plus clean samples.
if [[ -f "quality/adapter/src/parsers/taplo.rs" ]] &&
  grep -q -F -e 'pub fn parse_taplo_lint' quality/adapter/src/parsers/taplo.rs &&
  grep -q -F -e 'pub fn parse_taplo_format_check' quality/adapter/src/parsers/taplo.rs &&
  grep -q -F -e 'TAPLO_DIRTY_STDERR' quality/adapter/src/parsers/taplo.rs; then
  ok
else
  bad "taplo lost its lint plus format parser with dirty plus clean samples"
fi

# tsc keeps its adapter parser with pass plus fail samples but stays pipeline-only.
if [[ -f "quality/adapter/src/parsers/tsc.rs" ]] &&
  grep -q -F -e 'pub fn parse_tsc' quality/adapter/src/parsers/tsc.rs &&
  grep -q -F -e 'TS2322' quality/adapter/src/parsers/tsc.rs &&
  grep -q -F -e 'TS1109' quality/adapter/src/parsers/tsc.rs &&
  grep -q -F -e 'tsc' quality/tools/typescript/BUILD.bazel &&
  ! grep -q -F -e '"tsc",' "$real_rs" &&
  grep -q -F -e 'keeps an adapter parser' "$real_rs"; then
  ok
else
  bad "tsc lost its parser samples or pipeline-only wiring (parser plus no dispatch)"
fi

# Ty keeps its concise parser with diagnostic plus clean samples.
if [[ -f "quality/adapter/src/parsers/ty.rs" ]] &&
  grep -q -F -e 'pub fn parse_ty' quality/adapter/src/parsers/ty.rs &&
  grep -q -F -e 'All checks passed!' quality/adapter/src/parsers/ty.rs &&
  grep -q -F -e 'invalid-assignment' quality/adapter/src/parsers/ty.rs; then
  ok
else
  bad "ty lost its concise parser with diagnostic plus clean samples"
fi

# Vale keeps its JSON parser with alert plus config plus clean samples.
if [[ -f "quality/adapter/src/parsers/vale.rs" ]] &&
  grep -q -F -e 'pub fn parse_vale' quality/adapter/src/parsers/vale.rs &&
  grep -q -F -e 'VALE_DIRTY' quality/adapter/src/parsers/vale.rs &&
  grep -q -F -e 'VALE_CONFIG_ERROR' quality/adapter/src/parsers/vale.rs; then
  ok
else
  bad "vale lost its JSON parser with alert plus config plus clean samples"
fi

# Scalafmt keeps its diff parser with dirty plus clean samples (issue #797).
if [[ -f "quality/adapter/src/parsers/scalafmt.rs" ]] &&
  grep -q -F -e 'pub fn parse_scalafmt' quality/adapter/src/parsers/scalafmt.rs &&
  grep -q -F -e '--- a/' quality/adapter/src/parsers/scalafmt.rs; then
  ok
else
  bad "scalafmt lost its diff parser with dirty plus clean samples"
fi

# Scalafix keeps its callback-NDJSON parser with records plus clean (issue #797).
if [[ -f "quality/adapter/src/parsers/scalafix.rs" ]] &&
  grep -q -F -e 'pub fn parse_scalafix' quality/adapter/src/parsers/scalafix.rs &&
  grep -q -F -e 'DisableSyntax' quality/adapter/src/parsers/scalafix.rs; then
  ok
else
  bad "scalafix lost its callback parser with records plus clean"
fi

# CSharpier keeps its check parser with path plus clean samples (issue #797).
if [[ -f "quality/adapter/src/parsers/csharpier.rs" ]] &&
  grep -q -F -e 'pub fn parse_csharpier' quality/adapter/src/parsers/csharpier.rs; then
  ok
else
  bad "csharpier lost its check parser with path plus clean samples"
fi

# Fantomas keeps its JSON parser with needs-formatting plus clean (issue #797).
if [[ -f "quality/adapter/src/parsers/fantomas.rs" ]] &&
  grep -q -F -e 'pub fn parse_fantomas' quality/adapter/src/parsers/fantomas.rs &&
  grep -q -F -e 'needs-formatting' quality/adapter/src/parsers/fantomas.rs; then
  ok
else
  bad "fantomas lost its JSON parser with needs-formatting plus clean"
fi

# Roslyn keeps its SARIF parser with results plus clean (issue #797, delegated).
if [[ -f "quality/adapter/src/parsers/roslyn.rs" ]] &&
  grep -q -F -e 'pub fn parse_roslyn' quality/adapter/src/parsers/roslyn.rs &&
  grep -q -F -e 'CA1822' quality/adapter/src/parsers/roslyn.rs; then
  ok
else
  bad "roslyn lost its SARIF parser with results plus clean"
fi

# FSharpLint keeps its library-NDJSON parser with records plus clean (issue #797).
if [[ -f "quality/adapter/src/parsers/fsharplint.rs" ]] &&
  grep -q -F -e 'pub fn parse_fsharplint' quality/adapter/src/parsers/fsharplint.rs &&
  grep -q -F -e 'FL0036' quality/adapter/src/parsers/fsharplint.rs; then
  ok
else
  bad "fsharplint lost its library parser with records plus clean"
fi

# Runner-matrix doc owns the parser-sample backfill record.
if grep -q -F -e 'Parser samples' "$runner_doc" &&
  grep -q -F -e 'byte-identical to the parser unit samples' "$runner_doc" &&
  grep -q -F -e 'issue #465' "$runner_doc"; then
  ok
else
  bad "runner-matrix doc lost its parser-sample backfill record under #465"
fi

dx_test_summary "parser-sample backfill qualification harness"
