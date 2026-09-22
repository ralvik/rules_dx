#!/usr/bin/env bash
# Correlation plus committed-change manifest qualification harness.
#
# Qualifies the minor-1.1 output-protocol evolution (issue #811):
# - correlation: optional advisory `correlation` grouping identifier for
#   interleaved operations (`run:<target>` on run operations,
#   `update:<set>` on per-set reports plus file events), 1-128 chars of
#   [A-Za-z0-9/_:.-], line order authoritative, v1.0 consumers ignore it,
#   omitted preserves v1.0 wire shape;
# - manifest: backend committed-change manifest (`dx_update::manifest`)
#   validated against declared file locks, projecting to paired `change`
#   plus `applied` `mutation` events in path order before the per-set
#   report, empty or absent emits nothing preserving v1.0;
# - compat: 1.0 witnesses decode under 1.1 by major-only enforcement,
#   directory hubs never validate, Git scan/BUILD parse/rerun rejected;
# - fixtures: `cli/update/tests/fixtures/correlation_manifest/` (`pins.bzl`
#   plus `correlation_manifest.expected`) pins shapes plus rejected routes
#   plus honesty; unit fixtures live in `dx_output`, `dx_update::manifest`,
#   `cli/cli/src/exec/update.rs`, `cli/cli/src/exec/run.rs`;
# - scope: protocol-only; live Go no-op owns empty manifest, other sets
#   own none yet so live still emits per-set reports only.
#   Seed only: no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:correlation_manifest_qualification`,
# following //tools/ci:update_events_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

protocol="docs/cli/output-protocol.md"
command_doc="docs/cli/commands/audit-update-bazel.md"
pins="cli/update/tests/fixtures/correlation_manifest/pins.bzl"
expected="cli/update/tests/fixtures/correlation_manifest/correlation_manifest.expected"
fixture_build="cli/update/tests/fixtures/correlation_manifest/BUILD.bazel"
exec_update="cli/cli/src/exec/update.rs"
exec_run="cli/cli/src/exec/run.rs"
manifest="cli/update/src/manifest.rs"
lifecycle="cli/output/src/lifecycle.rs"
validation="cli/output/src/validation.rs"
schema="cli/schema/src/lib.rs"
finalize="cli/cli/src/finalize.rs"
targets="tools/ci/ci_targets_c.bzl"
dogfood="tools/ci/dogfood_freshness.sh"

# Protocol owns the 1.1 envelope with correlation plus current 1.1 version.
if grep -q -F -e '`correlation`' "$protocol" &&
  grep -q -F -e '{"major":1,"minor":1}' "$protocol" &&
  grep -q -F -e 'run:<target>' "$protocol" &&
  grep -q -F -e 'update:<set>' "$protocol" &&
  grep -q -F -e 'line order stays authoritative' "$protocol"; then
  ok
else
  bad "output-protocol.md lost its 1.1 envelope with correlation plus run/update grouping"
fi

# Protocol owns the manifest projection with spanning edits plus empty-preserves-v1.0.
if grep -q -F -e 'committed-change manifest' "$protocol" &&
  grep -q -F -e '`dx_update::manifest`' "$protocol" &&
  grep -q -F -e '0..old_len' "$protocol" &&
  grep -q -F -e '0..0' "$protocol" &&
  grep -q -F -e 'Empty or absent manifests project to no file events' "$protocol" &&
  grep -q -F -e 'third_party/dotnet/deps' "$protocol"; then
  ok
else
  bad "output-protocol.md lost its manifest projection with spanning edits plus empty-preserves-v1.0"
fi

# Protocol Compatibility Tests require the correlation_manifest fixtures.
if grep -q -F -e 'cli/update/tests/fixtures/correlation_manifest/' "$protocol" &&
  grep -q -F -e 'issue #811' "$protocol" &&
  grep -q -F -e 'v1.0 consumers ignore the field' "$protocol"; then
  ok
else
  bad "output-protocol.md Compatibility Tests lost the correlation_manifest fixture requirement under #811"
fi

# Command docs carry the 1.1 manifest cross-link with fixture paths.
if grep -q -F -e 'adds the backend committed-change manifest' "$command_doc" &&
  grep -q -F -e 'cli/update/tests/fixtures/correlation_manifest/' "$command_doc" &&
  grep -q -F -e 'cli/cli/src/exec/update.rs' "$command_doc" &&
  grep -q -F -e '[Output Protocol](../output-protocol.md#mutation)' "$command_doc"; then
  ok
else
  bad "audit-update-bazel.md lost its #811 manifest cross-link with fixtures"
fi

# Fixture pins stay present with shapes plus rejected plus honesty.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'SCHEMA_MINOR = 1' "$pins" &&
  grep -q -F -e 'CORRELATION_RUN = "run://app:bin"' "$pins" &&
  grep -q -F -e 'UPDATE_CORRELATION_PREFIX = "update:"' "$pins" &&
  grep -q -F -e 'MANIFEST_REJECTS_DIR_HUB' "$pins" &&
  grep -q -F -e 'COMPAT_V10_IGNORES_CORRELATION' "$pins" &&
  grep -q -F -e 'REJECTED_GIT_SCAN' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #811' "$pins"; then
  ok
else
  bad "correlation_manifest pins fixture lost its shapes plus compat plus rejected wiring under #811"
fi

# Expected fixture pins the shapes plus compat plus honesty lines.
if grep -q -F -e 'Correlation plus committed-change manifest (issue #811)' "$expected" &&
  grep -q -F -e 'run operations carry run:<target>' "$expected" &&
  grep -q -F -e 'directory hub third_party/dotnet/deps never validates' "$expected" &&
  grep -q -F -e 'empty or absent manifests emit nothing' "$expected" &&
  grep -q -F -e 'git scan rejected' "$expected" &&
  grep -q -F -e 'qualified seed-only under issue #811' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "correlation_manifest.expected lost its shapes plus compat plus honesty lines under #811"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'correlation_manifest.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "correlation_manifest BUILD.bazel lost its pins plus expected exports with corpus under #811"
fi

# Output helpers own correlation validation plus attachment.
if grep -q -F -e 'pub fn check_correlation' "$validation" &&
  grep -q -F -e 'pub fn with_correlation' "$lifecycle" &&
  grep -q -F -e 'SCHEMA_MINOR' "$lifecycle"; then
  ok
else
  bad "dx_output lost its check_correlation plus with_correlation wiring under #811"
fi

# Manifest owns validation plus projection plus dir-hub rejection.
if grep -q -F -e 'pub fn validate' "$manifest" &&
  grep -q -F -e 'pub fn project' "$manifest" &&
  grep -q -F -e 'Directory hubs' "$manifest" &&
  grep -q -F -e 'ProjectedFile' "$manifest"; then
  ok
else
  bad "dx_update::manifest lost its validate plus project plus dir-hub wiring under #811"
fi

# Run operations carry run:<target> correlation in execution order.
if grep -q -F -e 'with_correlation' "$exec_run" &&
  grep -q -F -e 'run:{target}' "$exec_run"; then
  ok
else
  bad "exec/run.rs lost its run:<target> correlation wiring under #811"
fi

# Update per-set reports carry update:<set> plus manifest-gated file pairs.
if grep -q -F -e 'update:{set_name}' "$exec_update" &&
  grep -q -F -e 'project_manifest_events' "$exec_update" &&
  grep -q -F -e 'live_success_manifest' "$exec_update"; then
  ok
else
  bad "exec/update.rs lost its update:<set> correlation plus manifest projection under #811"
fi

# Schema is minor 1.1 with forward-compat major-only witness enforcement.
if grep -q -F -e 'pub const SCHEMA_MINOR: u32 = 1' "$schema" &&
  grep -q -F -e 'payload.schema_major != generation_result::SCHEMA_MAJOR' "$finalize" &&
  ! grep -q -F -e 'payload.schema_minor != generation_result::SCHEMA_MINOR' "$finalize"; then
  ok
else
  bad "schema minor 1.1 plus major-only witness compat lost under #811"
fi

# Narrow suites prove the wire: output plus update plus cli plus schema.
if bazel test --noshow_progress //cli/output:dx_output_test //cli/update:dx_update_test //cli/schema:dx_schema_test 2>&1 | tail -n 5 | grep -q -F -e 'PASSED'; then
  ok
else
  bad "narrow output/update/schema suites failed under #811"
fi
if bazel test --noshow_progress //cli/cli:dx_cli_test 2>&1 | tail -n 5 | grep -q -F -e 'PASSED'; then
  ok
else
  bad "cli suite failed under #811"
fi

# Targets own the harness plus dogfood wires it in dogfood-freshness.
if grep -q -F -e 'name = "correlation_manifest_qualification"' "$targets" &&
  grep -q -F -e 'correlation_manifest_qualification.sh' "$targets" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:correlation_manifest_qualification' "$dogfood"; then
  ok
else
  bad "tools/ci targets or dogfood lost the correlation_manifest_qualification wiring (want target plus dogfood-freshness)"
fi

dx_test_summary "correlation manifest qualification harness"
