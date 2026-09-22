#!/usr/bin/env bash
# Parser-sample backfill qualification harness.
#
# Qualifies the as-built parser-sample backfill with fixture evidence and
# owned gaps, without claiming Supported:
# - delivered: every adapter-backed tool keeps a parser module with pinned
#   pass (clean) plus fail (dirty) samples exercised as unit tests
#   (`quality/adapter/src/parsers/*.rs`: biome lint plus format, buf lint plus format,
#   buildifier, clang_format, clang_tidy, clippy plus rustc via the shared rust diagnostics,
#   cppcheck, csharpier, cue, djlint lint plus format, errcheck, eslint,
#   fantomas, flake8, fsharplint, gofumpt, govet, jsonnetfmt, keep_sorted,
#   markdown_check, modfmt, pkl, prettier, psscriptanalyzer, pydoclint, pylint,
#   qmlformat, qmllint, roslyn, rubocop, ruff lint plus format, rustfmt, scalafix, scalafmt,
#   shellcheck, shfmt, standardrb, staticcheck, stylelint, taplo lint
#   plus format, terraform, tsc, ty, vale, yamlfmt, yamllint);
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

runner_doc="docs/quality/runner-matrix.md"
targets="tools/ci/ci_targets_c.bzl"
dogfood="tools/ci/dogfood_freshness.sh"
matrix="quality/testdata/runner_matrix_cases.bzl"
matrix_rust="quality/testdata/runner_matrix_rust.bzl"
ci="tools/ci/dogfood_freshness.sh"
build="tools/ci/ci_targets_c.bzl"
matrix_cases="quality/testdata/runner_matrix_cases.bzl"
real_rs="quality/runner/src/real.rs"
real_tests_b="quality/runner/src/real_tests_b.rs"

# Planned work lives in GitHub issues only (docs/roadmap.md removed under #981).
if [[ ! -f "docs/roadmap.md" ]]; then
  ok
else
  bad "docs/roadmap.md still exists (planned work lives in GitHub issues only, #981)"
fi

# Targets own the harness plus dogfood wires it.
if grep -q -F -e 'name = "parser_sample_qualification"' "$targets" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:parser_sample_qualification' "$dogfood"; then
  ok
else
  bad "ci_targets or dogfood lost the parser_sample_qualification wiring (want target plus dogfood-freshness)"
fi

# CI wires the harness in dogfood-freshness.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:parser_sample_qualification' "$ci"; then
  ok
else
  bad "dogfood_freshness.sh lost the parser_sample_qualification step (want dogfood-freshness)"
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
if grep -q -F -e 'CLIPPY_LINT' "$matrix_rust" &&
  grep -q -F -e 'RUSTC_TYPE_ERROR' "$matrix_rust" &&
  grep -q -F -e 'byte-identical to' "$matrix_rust" &&
  grep -q -F -e 'parser unit samples' "$matrix_rust" &&
  grep -q -F -e 'clippy::len_zero' "$matrix_rust" &&
  grep -q -F -e 'E0308' "$matrix_rust"; then
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
  grep -q -F -e 'keeps an adapter parser' "$real_tests_b" &&
  grep -q -F -e 'pipeline-only' "$real_rs"; then
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

# JVM cohort keeps SARIF plus dry-run parsers with pass plus fail samples (delivered under #796).
if [[ -f "quality/adapter/src/parsers/sarif.rs" ]] &&
  grep -q -F -e 'pub fn parse_sarif' quality/adapter/src/parsers/sarif.rs &&
  grep -q -F -e 'sarif_reports_point_and_range' quality/adapter/src/parsers/sarif.rs; then
  ok
else
  bad "sarif lost its shared JVM parser with point plus range samples"
fi

# Checkstyle keeps its SARIF parser with point plus clean samples.
if [[ -f "quality/adapter/src/parsers/checkstyle.rs" ]] &&
  grep -q -F -e 'pub fn parse_checkstyle' quality/adapter/src/parsers/checkstyle.rs &&
  grep -q -F -e 'checkstyle_reports_sarif_points' quality/adapter/src/parsers/checkstyle.rs; then
  ok
else
  bad "checkstyle lost its SARIF parser with point plus clean samples"
fi

# google-java-format keeps its dry-run parser with path plus clean samples.
if [[ -f "quality/adapter/src/parsers/google_java_format.rs" ]] &&
  grep -q -F -e 'pub fn parse_google_java_format' quality/adapter/src/parsers/google_java_format.rs &&
  grep -q -F -e 'google_java_format_reports_paths' quality/adapter/src/parsers/google_java_format.rs; then
  ok
else
  bad "google-java-format lost its dry-run parser with path plus clean samples"
fi

# ktfmt keeps its dry-run parser with path plus clean samples.
if [[ -f "quality/adapter/src/parsers/ktfmt.rs" ]] &&
  grep -q -F -e 'pub fn parse_ktfmt' quality/adapter/src/parsers/ktfmt.rs &&
  grep -q -F -e 'ktfmt_reports_paths' quality/adapter/src/parsers/ktfmt.rs; then
  ok
else
  bad "ktfmt lost its dry-run parser with path plus clean samples"
fi

# ktlint keeps its SARIF parser with range plus clean samples.
if [[ -f "quality/adapter/src/parsers/ktlint.rs" ]] &&
  grep -q -F -e 'pub fn parse_ktlint' quality/adapter/src/parsers/ktlint.rs &&
  grep -q -F -e 'ktlint_reports_sarif_ranges' quality/adapter/src/parsers/ktlint.rs; then
  ok
else
  bad "ktlint lost its SARIF parser with range plus clean samples"
fi

# PMD keeps its SARIF parser with range plus clean samples.
if [[ -f "quality/adapter/src/parsers/pmd.rs" ]] &&
  grep -q -F -e 'pub fn parse_pmd' quality/adapter/src/parsers/pmd.rs &&
  grep -q -F -e 'pmd_reports_sarif_ranges' quality/adapter/src/parsers/pmd.rs; then
  ok
else
  bad "pmd lost its SARIF parser with range plus clean samples"
fi

# SpotBugs keeps its SARIF parser with range plus clean samples (target-coupled, no matrix cell).
if [[ -f "quality/adapter/src/parsers/spotbugs.rs" ]] &&
  grep -q -F -e 'pub fn parse_spotbugs' quality/adapter/src/parsers/spotbugs.rs &&
  grep -q -F -e 'spotbugs_reports_sarif_ranges' quality/adapter/src/parsers/spotbugs.rs; then
  ok
else
  bad "spotbugs lost its SARIF parser with range plus clean samples"
fi

# clang-format keeps its diff parser with dirty plus clean samples (issue #798).
if [[ -f "quality/adapter/src/parsers/clang_format.rs" ]] &&
  grep -q -F -e 'pub fn parse_clang_format' quality/adapter/src/parsers/clang_format.rs &&
  grep -q -F -e '--- a/' quality/adapter/src/parsers/clang_format.rs; then
  ok
else
  bad "clang_format lost its diff parser with dirty plus clean samples"
fi

# gofumpt keeps its diff parser with dirty plus clean samples (issue #798).
if [[ -f "quality/adapter/src/parsers/gofumpt.rs" ]] &&
  grep -q -F -e 'pub fn parse_gofumpt' quality/adapter/src/parsers/gofumpt.rs &&
  grep -q -F -e '--- a/' quality/adapter/src/parsers/gofumpt.rs; then
  ok
else
  bad "gofumpt lost its diff parser with dirty plus clean samples"
fi

# clang-tidy keeps its text parser with diagnostics plus clean (issue #798).
if [[ -f "quality/adapter/src/parsers/clang_tidy.rs" ]] &&
  grep -q -F -e 'pub fn parse_clang_tidy' quality/adapter/src/parsers/clang_tidy.rs &&
  grep -q -F -e 'readability-else-after-return' quality/adapter/src/parsers/clang_tidy.rs; then
  ok
else
  bad "clang_tidy lost its text parser with diagnostics plus clean"
fi

# cppcheck keeps its XML parser with errors plus clean (issue #798).
if [[ -f "quality/adapter/src/parsers/cppcheck.rs" ]] &&
  grep -q -F -e 'pub fn parse_cppcheck' quality/adapter/src/parsers/cppcheck.rs &&
  grep -q -F -e 'nullPointer' quality/adapter/src/parsers/cppcheck.rs; then
  ok
else
  bad "cppcheck lost its XML parser with errors plus clean"
fi

# staticcheck keeps its JSON parser with findings plus clean (issue #798).
if [[ -f "quality/adapter/src/parsers/staticcheck.rs" ]] &&
  grep -q -F -e 'pub fn parse_staticcheck' quality/adapter/src/parsers/staticcheck.rs &&
  grep -q -F -e 'SA4006' quality/adapter/src/parsers/staticcheck.rs; then
  ok
else
  bad "staticcheck lost its JSON parser with findings plus clean"
fi

# govet keeps its text parser with diagnostics plus clean (issue #798).
if [[ -f "quality/adapter/src/parsers/govet.rs" ]] &&
  grep -q -F -e 'pub fn parse_govet' quality/adapter/src/parsers/govet.rs &&
  grep -q -F -e 'non-constant format string' quality/adapter/src/parsers/govet.rs; then
  ok
else
  bad "govet lost its text parser with diagnostics plus clean"
fi

# errcheck keeps its text parser with diagnostics plus clean (issue #798).
if [[ -f "quality/adapter/src/parsers/errcheck.rs" ]] &&
  grep -q -F -e 'pub fn parse_errcheck' quality/adapter/src/parsers/errcheck.rs &&
  grep -q -F -e 'unchecked error' quality/adapter/src/parsers/errcheck.rs; then
  ok
else
  bad "errcheck lost its text parser with diagnostics plus clean"
fi

# Buf keeps its JSONL plus diff parsers with records plus clean (issue #799).
if [[ -f "quality/adapter/src/parsers/buf.rs" ]] &&
  grep -q -F -e 'pub fn parse_buf_lint' quality/adapter/src/parsers/buf.rs &&
  grep -q -F -e 'pub fn parse_buf_format' quality/adapter/src/parsers/buf.rs &&
  grep -q -F -e 'PACKAGE_DIRECTORY_MATCH' quality/adapter/src/parsers/buf.rs; then
  ok
else
  bad "buf lost its JSONL plus diff parsers with records plus clean"
fi

# qmlformat keeps its check parser with path plus clean samples (issue #799).
if [[ -f "quality/adapter/src/parsers/qmlformat.rs" ]] &&
  grep -q -F -e 'pub fn parse_qmlformat' quality/adapter/src/parsers/qmlformat.rs; then
  ok
else
  bad "qmlformat lost its check parser with path plus clean samples"
fi

# qmllint keeps its JSON parser with diagnostics plus clean (issue #799).
if [[ -f "quality/adapter/src/parsers/qmllint.rs" ]] &&
  grep -q -F -e 'pub fn parse_qmllint' quality/adapter/src/parsers/qmllint.rs &&
  grep -q -F -e 'unqualified' quality/adapter/src/parsers/qmllint.rs; then
  ok
else
  bad "qmllint lost its JSON parser with diagnostics plus clean"
fi


# cue keeps its diff parser with dirty plus clean samples (issue #800).
if [[ -f "quality/adapter/src/parsers/cue.rs" ]] &&
  grep -q -F -e 'pub fn parse_cue' quality/adapter/src/parsers/cue.rs &&
  grep -q -F -e '--- ' quality/adapter/src/parsers/cue.rs; then
  ok
else
  bad "cue lost its diff parser with dirty plus clean samples"
fi

# jsonnetfmt keeps its diff parser with dirty plus clean samples (issue #800).
if [[ -f "quality/adapter/src/parsers/jsonnetfmt.rs" ]] &&
  grep -q -F -e 'pub fn parse_jsonnetfmt' quality/adapter/src/parsers/jsonnetfmt.rs &&
  grep -q -F -e '--- ' quality/adapter/src/parsers/jsonnetfmt.rs; then
  ok
else
  bad "jsonnetfmt lost its diff parser with dirty plus clean samples"
fi

# pkl keeps its diff parser with dirty plus clean samples (issue #800).
if [[ -f "quality/adapter/src/parsers/pkl.rs" ]] &&
  grep -q -F -e 'pub fn parse_pkl' quality/adapter/src/parsers/pkl.rs &&
  grep -q -F -e '--- ' quality/adapter/src/parsers/pkl.rs; then
  ok
else
  bad "pkl lost its diff parser with dirty plus clean samples"
fi

# modfmt keeps its diff parser with dirty plus clean samples (issue #800).
if [[ -f "quality/adapter/src/parsers/modfmt.rs" ]] &&
  grep -q -F -e 'pub fn parse_modfmt' quality/adapter/src/parsers/modfmt.rs &&
  grep -q -F -e '--- ' quality/adapter/src/parsers/modfmt.rs; then
  ok
else
  bad "modfmt lost its diff parser with dirty plus clean samples"
fi

# terraform keeps its diff parser with dirty plus clean samples (issue #800).
if [[ -f "quality/adapter/src/parsers/terraform.rs" ]] &&
  grep -q -F -e 'pub fn parse_terraform' quality/adapter/src/parsers/terraform.rs &&
  grep -q -F -e '--- ' quality/adapter/src/parsers/terraform.rs; then
  ok
else
  bad "terraform lost its diff parser with dirty plus clean samples"
fi

# yamlfmt keeps its diff parser with dirty plus clean samples (issue #800).
if [[ -f "quality/adapter/src/parsers/yamlfmt.rs" ]] &&
  grep -q -F -e 'pub fn parse_yamlfmt' quality/adapter/src/parsers/yamlfmt.rs &&
  grep -q -F -e '--- ' quality/adapter/src/parsers/yamlfmt.rs; then
  ok
else
  bad "yamlfmt lost its diff parser with dirty plus clean samples"
fi

# shfmt keeps its diff parser with dirty plus clean samples (issue #800).
if [[ -f "quality/adapter/src/parsers/shfmt.rs" ]] &&
  grep -q -F -e 'pub fn parse_shfmt' quality/adapter/src/parsers/shfmt.rs &&
  grep -q -F -e '--- ' quality/adapter/src/parsers/shfmt.rs; then
  ok
else
  bad "shfmt lost its diff parser with dirty plus clean samples"
fi

# standardrb keeps its diff parser with dirty plus clean samples (issue #800).
if [[ -f "quality/adapter/src/parsers/standardrb.rs" ]] &&
  grep -q -F -e 'pub fn parse_standardrb' quality/adapter/src/parsers/standardrb.rs &&
  grep -q -F -e '--- ' quality/adapter/src/parsers/standardrb.rs; then
  ok
else
  bad "standardrb lost its diff parser with dirty plus clean samples"
fi

# djlint keeps its lint plus format parsers with dirty plus clean samples (issue #800).
if [[ -f "quality/adapter/src/parsers/djlint.rs" ]] &&
  grep -q -F -e 'pub fn parse_djlint' quality/adapter/src/parsers/djlint.rs &&
  grep -q -F -e 'pub fn parse_djlint_format' quality/adapter/src/parsers/djlint.rs; then
  ok
else
  bad "djlint lost its lint plus format parsers with dirty plus clean samples"
fi

# stylelint keeps its JSON parser with warnings plus clean (issue #800).
if [[ -f "quality/adapter/src/parsers/stylelint.rs" ]] &&
  grep -q -F -e 'pub fn parse_stylelint' quality/adapter/src/parsers/stylelint.rs &&
  grep -q -F -e 'color-no-invalid-hex' quality/adapter/src/parsers/stylelint.rs; then
  ok
else
  bad "stylelint lost its JSON parser with warnings plus clean"
fi

# rubocop keeps its JSON parser with offenses plus clean (issue #800).
if [[ -f "quality/adapter/src/parsers/rubocop.rs" ]] &&
  grep -q -F -e 'pub fn parse_rubocop' quality/adapter/src/parsers/rubocop.rs &&
  grep -q -F -e 'Style/StringLiterals' quality/adapter/src/parsers/rubocop.rs; then
  ok
else
  bad "rubocop lost its JSON parser with offenses plus clean"
fi

# psscriptanalyzer keeps its text parser with diagnostics plus clean (issue #800).
if [[ -f "quality/adapter/src/parsers/psscriptanalyzer.rs" ]] &&
  grep -q -F -e 'pub fn parse_psscriptanalyzer' quality/adapter/src/parsers/psscriptanalyzer.rs; then
  ok
else
  bad "psscriptanalyzer lost its text parser with diagnostics plus clean"
fi

# yamllint keeps its text parser with diagnostics plus clean (issue #800).
if [[ -f "quality/adapter/src/parsers/yamllint.rs" ]] &&
  grep -q -F -e 'pub fn parse_yamllint' quality/adapter/src/parsers/yamllint.rs; then
  ok
else
  bad "yamllint lost its text parser with diagnostics plus clean"
fi

# shellcheck keeps its gcc parser with diagnostics plus clean (issue #800).
if [[ -f "quality/adapter/src/parsers/shellcheck.rs" ]] &&
  grep -q -F -e 'pub fn parse_shellcheck' quality/adapter/src/parsers/shellcheck.rs; then
  ok
else
  bad "shellcheck lost its gcc parser with diagnostics plus clean"
fi

# keep_sorted keeps its text parser with diagnostics plus clean (issue #800).
if [[ -f "quality/adapter/src/parsers/keep_sorted.rs" ]] &&
  grep -q -F -e 'pub fn parse_keep_sorted' quality/adapter/src/parsers/keep_sorted.rs; then
  ok
else
  bad "keep_sorted lost its text parser with diagnostics plus clean"
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
