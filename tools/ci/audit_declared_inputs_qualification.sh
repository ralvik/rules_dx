#!/usr/bin/env bash
# Audit curator declared-Bazel-inputs qualification harness.
#
# Qualifies the `dx audit` future declared-inputs delivery with fixture
# evidence (issue #812):
# - declared inputs: the committed `licenses.toml` bytes ride
#   `//:audit_curator` as a declared Bazel input, so Bazel-owned analysis
#   declares the same bytes the CLI reads and cache identity reflects the
#   data actually analyzed rather than an untracked workspace read;
# - advisory snapshots ride identified snapshot plus identity rels
#   (`.dx/advisory/<set>.json` plus `.meta.json`) as declared analysis
#   inputs with 24h same-day freshness and refresh-failure mapping pinned
#   in `dx_audit::advisory` (missing/invalid/stale fails with
#   `advisory_refresh_failed`, never clean and never a stale fallback;
#   live CLI performs no network fetch);
# - per-package shape unchanged: `[[inventory]]` keeps package plus set
#   plus license plus versions plus text_present so NOTICE aggregation in
#   `deploy/release/notice.bzl` stays hermetic and cached (hermetic
#   `notice_gen`, byte-identical rebuilds, `missing-notice-text` fails);
# - semantics preserved: same policy evaluation, same advisory freshness
#   plus fail-closed mapping, same SARIF plus SPDX reporting, same exit
#   codes; fixtures plus `dx_audit` units pin the mapping.
# Seed only: no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:audit_declared_inputs_qualification`,
# following //tools/ci:cli_strict_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

root_build="BUILD.bazel"
curator="cli/audit/src/curator.rs"
audit_lib="cli/audit/src/lib.rs"
audit_build="cli/audit/BUILD.bazel"
pins="cli/audit/tests/fixtures/declared_inputs/pins.bzl"
expected="cli/audit/tests/fixtures/declared_inputs/declared_inputs.expected"
fixture_build="cli/audit/tests/fixtures/declared_inputs/BUILD.bazel"
notice_rule="deploy/release/notice.bzl"
audit_doc="docs/cli/commands/audit-update-bazel.md"
testing="docs/testing/cli.md"
build="tools/ci/ci_targets_c.bzl"
ci="tools/ci/dogfood_freshness.sh"

# Root filegroup exposes the committed curator bytes as a declared input.
if grep -q -F -e 'name = "audit_curator"' "$root_build" &&
  grep -q -F -e '"licenses.toml"' "$root_build" &&
  grep -q -F -e 'issue #812' "$root_build"; then
  ok
else
  bad "BUILD.bazel lost its //:audit_curator declared-input filegroup over licenses.toml under #812"
fi

# Curator module pins the label plus rel plus advisory sets.
if grep -q -F -e 'LICENSES_TOML_LABEL' "$curator" &&
  grep -q -F -e '//:audit_curator' "$curator" &&
  grep -q -F -e 'LICENSES_TOML_REL' "$curator" &&
  grep -q -F -e 'CURATOR_ADVISORY_SETS' "$curator" &&
  grep -q -F -e 'curator_input_rels' "$curator"; then
  ok
else
  bad "curator.rs lost its label plus rel plus advisory-sets pins under #812"
fi

# Curator module keeps per-package shape plus advisory rel mapping.
if grep -q -F -e 'advisory_inputs' "$curator" &&
  grep -q -F -e 'snapshot_rel' "$curator" &&
  grep -q -F -e 'identity_rel' "$curator" &&
  grep -q -F -e 'LicenseInventory' "$curator"; then
  ok
else
  bad "curator.rs lost its snapshot plus identity plus inventory-shape mapping under #812"
fi

# Crate root wires the curator module.
if grep -q -F -e 'pub mod curator' "$audit_lib"; then
  ok
else
  bad "cli/audit lib.rs lost its pub mod curator wiring under #812"
fi

# Crate BUILD lists the curator source.
if grep -q -F -e 'src/curator.rs' "$audit_build"; then
  ok
else
  bad "cli/audit BUILD.bazel lost its src/curator.rs listing under #812"
fi

# Fixture pins stay present with label plus rel plus sets plus shape.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'CURATOR_LABEL = "//:audit_curator"' "$pins" &&
  grep -q -F -e 'CURATOR_REL = "licenses.toml"' "$pins" &&
  grep -q -F -e 'CURATOR_ADVISORY_SETS' "$pins" &&
  grep -q -F -e 'INVENTORY_SHAPE' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #812' "$pins"; then
  ok
else
  bad "declared_inputs pins fixture lost its label plus sets plus shape wiring under #812"
fi

# Fixture expected pins the declared plus shape plus honesty lines.
if grep -q -F -e 'declared inputs qualified seed-only under issue #812' "$expected" &&
  grep -q -F -e '//:audit_curator declares licenses.toml' "$expected" &&
  grep -q -F -e 'per-package inventory shape unchanged' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "declared_inputs.expected lost its declared plus shape plus honesty lines under #812"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'declared_inputs.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "declared_inputs BUILD.bazel lost its pins plus expected exports with corpus under #812"
fi

# NOTICE aggregation stays hermetic over declared per-package inputs.
if grep -q -F -e 'def notice_bundle' "$notice_rule" &&
  grep -q -F -e 'hermetic' "$notice_rule" &&
  grep -q -F -e 'missing-notice-text' "$notice_rule"; then
  ok
else
  bad "deploy/release/notice.bzl lost its hermetic notice_bundle with missing-notice-text under #812"
fi

# Live proof: the curator filegroup resolves to the committed bytes.
if bazel query '//:audit_curator' --noshow_progress >/dev/null 2>&1 &&
  bazel cquery '//:audit_curator' --noshow_progress 2>/dev/null | grep -q -e 'audit_curator'; then
  ok
else
  bad "audit_curator filegroup failed to resolve (want //:audit_curator green, #812)"
fi

# Live proof: the curator bytes ride the filegroup as a declared input.
if bazel query '//:audit_curator' --output=build --noshow_progress 2>/dev/null | grep -q -e 'licenses.toml'; then
  ok
else
  bad "audit_curator query lost its licenses.toml declared input (want hermetic bytes, #812)"
fi

# Live proof: the NOTICE demo stays green over declared inputs.
if bazel build //deploy/release:notice_demo --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "notice_demo failed to build (want green over declared inputs, #812)"
fi

# Live proof: dx_audit units stay green with the curator mapping.
if bazel test //cli/audit:dx_audit_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "dx_audit_test failed (want green curator plus advisory units, #812)"
fi

# Audit docs keep the declared-inputs delivery record with same shape.
if grep -q -F -e 'declared-Bazel-inputs delivery keeps the same' "$audit_doc" &&
  grep -q -F -e 'per-package shape' "$audit_doc"; then
  ok
else
  bad "audit-update-bazel.md lost its declared-inputs same-shape record under #812"
fi

# Testing matrix pins the declared-inputs qualification under #812.
if grep -q -F -e 'bazel run //tools/ci:audit_declared_inputs_qualification' "$testing" &&
  grep -q -F -e 'issue #812' "$testing"; then
  ok
else
  bad "testing/cli.md lost its #812 declared-inputs pins"
fi

# BUILD owns the harness target plus dogfood wires it.
if grep -q -F -e 'name = "audit_declared_inputs_qualification"' "$build" &&
  grep -q -F -e 'audit_declared_inputs_qualification.sh' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:audit_declared_inputs_qualification' "$ci"; then
  ok
else
  bad "tools/ci wiring lost the audit_declared_inputs_qualification target plus dogfood-freshness under #812"
fi

dx_test_summary "audit declared inputs qualification harness"
