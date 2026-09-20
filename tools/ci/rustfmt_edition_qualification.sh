#!/usr/bin/env bash
# Rustfmt crate-edition qualification harness.
#
# Qualifies the as-built edition-aware formatting gate with fixture evidence
# and owned gaps, without claiming Supported:
# - delivered: the crate's real edition flows from the authoritative
#   `CrateInfo` (test targets use the inner crate) through the quality
#   aspect as `--tool-edition` into `rustfmt --edition`, with
#   `RUST_EDITION` as the provider-less fallback; the flag wins over any
#   config `edition` key, and a missing edition fails the action instead
#   of guessing;
# - fixtures: `edition_2015_lib` (2015-only `async` identifier, clean only
#   under `--edition 2015`) with `edition_fmt_test` under
#   `bazel test //...`, plus the Layer-2 `matrix_rust_format_edition_2015`
#   pass cell and the `matrix_rust_format_edition_mismatch` wrong-edition
#   syntax-error cell (single-edition rustfmt rejected);
# - open owned gaps: platform plus consumer plus release evidence, exact
#   per-edition pins beyond the seed host, no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:rustfmt_edition_qualification`,
# following //tools/ci:parser_sample_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

roadmap="docs/roadmap.md"
verify="docs/testing/verification-matrix.md"
runner_doc="docs/quality/runner-matrix.md"
integrations="docs/quality/tool-integrations.md"
ci=".github/workflows/ci.yml"
build="tools/ci/BUILD.bazel"
matrix="quality/testdata/runner_matrix_cases.bzl"
fixture_build="rust/tests/fixtures/hello/BUILD.bazel"
fixture_src="rust/tests/fixtures/hello/edition_2015.rs"

# Roadmap owns the delivered record under.
if grep -q -F -e 'rustfmt with crate edition delivered (issue #468' "$roadmap"; then
  ok
else
  bad "roadmap lost its rustfmt delivered record under #468"
fi

# Verification matrix owns the qualified seed-only record under.
if grep -q -F -e 'rustfmt_edition_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #468' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:rustfmt_edition_qualification' "$verify"; then
  ok
else
  bad "verification-matrix lost its #468 rustfmt-edition qualified record"
fi

# Verification matrix lists the harness in dogfood-freshness.
if grep -q -F -e ':rustfmt_edition_qualification' "$verify"; then
  ok
else
  bad "verification-matrix dogfood-freshness lost :rustfmt_edition_qualification"
fi

# Verification matrix Green lists the harness count.
if grep -q -F -e '`rustfmt_edition_qualification` 20/20' "$verify"; then
  ok
else
  bad "verification-matrix Green lost rustfmt_edition_qualification 20/20"
fi

# BUILD owns the harness target.
if grep -q -F -e 'name = "rustfmt_edition_qualification"' "$build"; then
  ok
else
  bad "tools/ci/BUILD.bazel lost the rustfmt_edition_qualification target"
fi

# CI wires the harness in dogfood-freshness.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:rustfmt_edition_qualification' "$ci"; then
  ok
else
  bad "ci.yml lost the rustfmt_edition_qualification step (want dogfood-freshness)"
fi

# The adapter passes the caller edition to `--edition` verbatim, never a default.
if grep -q -F -e 'OsString::from("--edition")' quality/adapter/src/commands.rs &&
  grep -q -F -e 'edition: &str' quality/adapter/src/commands.rs; then
  ok
else
  bad "commands.rs lost the verbatim caller-edition --edition wiring"
fi

# The runner fails the action when the edition is missing, on check and fix.
if grep -q -F -e 'missing tool edition' quality/runner/src/real.rs &&
  [[ "$(grep -c -F -e 'Self::rustfmt_edition(tool)?' quality/runner/src/real.rs)" -ge 2 ]]; then
  ok
else
  bad "real.rs lost the missing-edition failure on check plus fix"
fi

# The aspect reads the authoritative CrateInfo edition, the test inner crate, else RUST_EDITION.
if grep -q -F -e 'target[_rust_common.crate_info].edition' quality/real_aspects.bzl &&
  grep -q -F -e 'target[_rust_common.test_crate_info].crate.edition' quality/real_aspects.bzl &&
  grep -q -F -e 'rustfmt_edition = RUST_EDITION' quality/real_aspects.bzl; then
  ok
else
  bad "real_aspects.bzl lost the CrateInfo plus test-crate plus RUST_EDITION edition rule"
fi

# The single source of truth stays consistent: RUST_EDITION matches rustfmt.toml.
if grep -q -F -e 'RUST_EDITION = "2021"' rust/rules/edition.bzl &&
  grep -q -F -e 'edition = "2021"' rustfmt.toml; then
  ok
else
  bad "RUST_EDITION drifted from rustfmt.toml (want both 2021)"
fi

# The matrix proves a non-default edition over the real toolchain rustfmt.
if grep -q -F -e 'matrix_rust_format_edition_2015' "$matrix" &&
  grep -q -F -e '"2015"' "$matrix" &&
  grep -q -F -e 'rustfmt_from_toolchain' "$matrix"; then
  ok
else
  bad "runner matrix lost the 2015 edition cell over toolchain rustfmt"
fi

# The matrix proves the wrong edition surfaces syntax errors (single-edition rejected).
if grep -q -F -e 'matrix_rust_format_edition_mismatch' "$matrix" &&
  grep -q -F -e 'expected identifier, found keyword' "$matrix"; then
  ok
else
  bad "runner matrix lost the wrong-edition mismatch cell"
fi

# The crate fixture carries a 2015 lib with rustfmt coverage under bazel test.
if grep -q -F -e 'name = "edition_2015_lib"' "$fixture_build" &&
  grep -q -F -e 'edition = "2015"' "$fixture_build" &&
  grep -q -F -e 'name = "edition_fmt_test"' "$fixture_build"; then
  ok
else
  bad "hello fixture lost edition_2015_lib plus edition_fmt_test"
fi

# The fixture uses edition-sensitive syntax (plain identifier only in 2015).
if [[ -f "$fixture_src" ]] && grep -q -F -e 'let async = 1;' "$fixture_src"; then
  ok
else
  bad "edition_2015.rs lost its 2015-only async-identifier bytes"
fi

# Live proof: the aspect action for the 2015 crate carries rustfmt=2015.
edition_actions="$(bazel aquery '//rust/tests/fixtures/hello:edition_2015_lib' \
  --aspects=//quality:real_aspects.bzl%real_rust_format_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
if [[ "$edition_actions" == *"'rustfmt=2015'"* ]]; then
  ok
else
  bad "aspect action for edition_2015_lib lost --tool-edition rustfmt=2015"
fi

# Live proof: the default crate still carries the single source of truth.
default_actions="$(bazel aquery '//rust/tests/fixtures/hello:hello_lib' \
  --aspects=//quality:real_aspects.bzl%real_rust_format_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
if [[ "$default_actions" == *"'rustfmt=2021'"* ]]; then
  ok
else
  bad "aspect action for hello_lib lost --tool-edition rustfmt=2021"
fi

# Runner-matrix doc owns the edition record.
if grep -q -F -e 'matrix_rust_format_edition_2015' "$runner_doc" &&
  grep -q -F -e 'matrix_rust_format_edition_mismatch' "$runner_doc" &&
  grep -q -F -e 'issue #468' "$runner_doc"; then
  ok
else
  bad "runner-matrix doc lost its edition record under #468"
fi

# Tool-integrations owns the delivered crate-contextual record.
if grep -q -F -e 'Rust formatting is crate-contextual (delivered under issue #468)' "$integrations"; then
  ok
else
  bad "tool-integrations lost its delivered crate-contextual record under #468"
fi

# rustfmt_test stays the single-sourced lint entry point for the new lib.
if grep -q -F -e 'targets = [":edition_2015_lib"]' "$fixture_build" &&
  grep -q -F -e 'rustfmt_test' "$fixture_build"; then
  ok
else
  bad "edition fixture lost its single-sourced rustfmt_test coverage"
fi

# The hinted 2015 lib proves the CLI flag wins over the 2021 config key.
if grep -q -F -e 'aspect_hints = ["//:rustfmt_config"]' "$fixture_build" &&
  grep -q -F -e 'silently wins over any config' quality/adapter/src/commands.rs; then
  ok
else
  bad "edition fixture lost the flag-wins-over-config proof (hint plus adapter note)"
fi

dx_test_summary "rustfmt edition qualification harness"
