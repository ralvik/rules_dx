#!/usr/bin/env bash
# FSharpLint console vs library-API binding qualification.
#
# Decides the open FSharpLint adapter-input risk with fixture evidence,
# recorded explicitly here and in the owning docs, never silently dropped
# (silent console parse rejected per the issue alternatives):
# - console-parse REJECTED for diagnostics, both shapes: the standard
#   multi-line human block carries only the start position with the rule
#   ID inside a URL plus snippet plus caret plus separator plus
#   shared-stream info lines, so block boundaries are unprovable; the
#   `-f msbuild` single line carries the full range plus rule ID but drops
#   ErrorText plus SuggestedFix plus TypeChecks plus RuleName with
#   shared-stream info lines, so fail-closed parsing is unprovable
#   (fixtures `console_standard.txt` plus `console_msbuild.txt` modeled on
#   the documented FL0036 plus FL0034 shapes).
# - wire REQUIRED: structured diagnostics via a custom .NET entrypoint
#   binding `FSharpLint.Application.Lint` (`lintFile`/`lintSource` for
#   files, `lintProject`/`lintSolution` for project context) with the
#   `ReceivedWarning` callback over the exact official tool-package
#   artifacts on the shared managed .NET runtime cohort (no
#   `dotnet tool install`).
# - target-coupled project context REQUIRED where rules need it
#   (`.fsproj`/`.sln` plus `fsharplint.json` from the authoritative
#   `fsharp_*` target context; single-file runs without required project
#   context fail closed, never a silent check-only fallback).
# - fix flow is sandbox-apply-and-diff with declared per-target outputs as
#   unified patches (never `IN_PLACE` mutation of immutable inputs; the
#   console emits diagnostics only, never patches).
# - native config is sole policy: checked-in `fsharplint.json` required to
#   change policy (see `example.fsharplint.json` with formatting-adjacent
#   rules off because Fantomas owns formatting).
# Adapter-only: no adapter claims `fsharp` yet (cohort stays owned by;
# decision recorded under with fixtures).
#
# Versioned here, run by CI via `bazel run //tools/ci:fsharplint_qualification`.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="fsharp/tests/fixtures/fsharplint/pins.bzl"
pins_build="fsharp/tests/fixtures/fsharplint/BUILD.bazel"
sample_src="fsharp/tests/fixtures/fsharplint/Sample.fs"
standard_sample="fsharp/tests/fixtures/fsharplint/console_standard.txt"
msbuild_sample="fsharp/tests/fixtures/fsharplint/console_msbuild.txt"
sample_conf="fsharp/tests/fixtures/fsharplint/example.fsharplint.json"
adapters="quality/adapters.bzl"
parity="quality/parity_tests.bzl"
curated="quality/curated_defaults.bzl"
native="quality/native_config.bzl"
matrix="quality/testdata/runner_matrix_cases.bzl"
support="docs/product/support-matrix.md"
baseline="docs/tools/tool-baseline.md"
acquisition="docs/tools/tool-acquisition.md"
integrations="docs/quality/tool-integrations.md"
native_doc="docs/quality/native-configuration.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

# Fixture set stays present.
if [[ -f "$pins" && -f "$pins_build" && -f "$sample_src" && -f "$standard_sample" && -f "$msbuild_sample" && -f "$sample_conf" ]]; then
  ok
else
  bad "fsharplint fixture missing (want $pins plus $pins_build plus Sample.fs plus console_standard plus console_msbuild plus example config)"
fi

# Pins record the reference version plus exact-package artifact plus the
# upstream wrapper-vs-library shapes (versions qualified seed-only under
# ; digests stay owned under).
if grep -q -F -e 'FSHARPLINT_VERSION = "0.27.0"' "$pins" &&
  grep -q -F -e 'exact official tool package over the managed .NET cohort' "$pins" &&
  grep -q -F -e 'no dotnet tool install' "$pins" &&
  grep -q -F -e 'FSharpLint.Application.Lint' "$pins" &&
  grep -q -F -e 'ReceivedWarning' "$pins"; then
  ok
else
  bad "pins.bzl lost its FSharpLint 0.27.0 version plus exact-package artifact plus library-API shape under issue #493"
fi

# Pins record the explicit parse-vs-wire decision plus rejections (silent
# console parse rejected per the issue alternatives).
if grep -q -F -e 'console-parse rejected' "$pins" &&
  grep -q -F -e 'wire via FSharpLint.Application.Lint' "$pins" &&
  grep -q -F -e 'FSHARPLINT_CONSOLE_PARSE_REJECTED' "$pins" &&
  grep -q -F -e 'silent console parse rejected' "$pins" &&
  grep -q -F -e 'FSHARPLINT_WIRE_REQUIRED' "$pins"; then
  ok
else
  bad "pins.bzl lost its explicit parse-vs-wire record with silent-parse rejection under issue #493"
fi

# Pins record target-coupled project wiring plus fix flow plus
# native-config sole policy, and the example config proves
# Fantomas-owns-formatting (formatting-adjacent rules off).
if grep -q -F -e 'target-coupled' "$pins" &&
  grep -q -F -e 'sandbox-apply-and-diff' "$pins" &&
  grep -q -F -e 'IN_PLACE' "$pins" &&
  grep -q -F -e 'checked-in fsharplint.json' "$pins" &&
  grep -q -F -e 'Fantomas owns formatting' "$pins" &&
  grep -q -F -e '"indentation": {"enabled": false}' "$sample_conf" &&
  grep -q -F -e '"interfaceNames": {"enabled": true' "$sample_conf"; then
  ok
else
  bad "pins.bzl lost its project wiring plus fix-flow plus config-policy record (or example config lost formatting-off) under issue #493"
fi

# Standard console fixture proves lossiness: rule IDs appear only inside
# the See-URL hint, the location line carries the start position only, the
# snippet plus caret plus 80-dash separator share stdout with info lines,
# and no line carries an end position or a structured rule field.
if grep -q -F -e 'FL0036.html' "$standard_sample" &&
  grep -q -F -e 'FL0034.html' "$standard_sample" &&
  grep -q -F -e 'Error in file Sample.fs on line 3 starting at column 6' "$standard_sample" &&
  grep -q -F -e 'type ExampleInterface =' "$standard_sample" &&
  grep -q -F -e '^' "$standard_sample" &&
  grep -q -F -e '--------------------------------------------------------------------------------' "$standard_sample" &&
  grep -q -F -e 'Running FSharpLint with' "$standard_sample" &&
  ! grep -q -F -e 'RuleIdentifier' "$standard_sample" &&
  ! grep -q -F -e 'SuggestedFix' "$standard_sample"; then
  ok
else
  bad "standard console fixture lost its lossiness proof (want URL-only rule IDs, start-only locations, snippet plus caret plus separator, shared-stream info, no structured fields)"
fi

# MSBuild console fixture proves the single-line shape is still lossy: each
# warning carries the full 4-tuple range plus rule ID plus message, info
# lines share the same stdout stream, and fix plus typecheck context
# (ErrorText, SuggestedFix, TypeChecks, RuleName) travels only via the
# library record, never the console.
if grep -q -F -e ':FSharpLint warning FL0036:' "$msbuild_sample" &&
  grep -q -F -e ':FSharpLint warning FL0034:' "$msbuild_sample" &&
  grep -q -F -e 'Sample.fs(3,6,3,23)' "$msbuild_sample" &&
  grep -q -F -e 'Sample.fs(6,23,6,36)' "$msbuild_sample" &&
  grep -q -F -e 'Running FSharpLint with' "$msbuild_sample" &&
  grep -q -F -e 'LintWarning{RuleIdentifier, RuleName, FilePath, ErrorText' "$pins" &&
  grep -q -F -e 'SuggestedFix' "$pins"; then
  ok
else
  bad "msbuild console fixture lost its shape-plus-lossiness proof (want 4-tuple ranges with rule IDs, shared-stream info, fix context owned by the library record)"
fi

# No false adapter claim: fsharplint stays out of REAL_ADAPTERS and the runner
# matrix carries no fsharp cells (claims land only with green adapter
# evidence; cohort classification stays, owned under).
if ! grep -q -F -e '"fsharplint":' "$adapters" &&
  ! grep -q -F -e 'matrix_fsharp_' "$matrix" &&
  grep -q -F -e '"fsharp": "fsharp"' "$adapters"; then
  ok
else
  bad "false fsharplint adapter claim (want no fsharplint in REAL_ADAPTERS, no matrix_fsharp_ cells, fsharp stays classified)"
fi

# Tool integrations record the explicit decision (wire required,
# console-parse rejected both shapes, target-coupled project context,
# sandbox fix flow), never silent, with no adapter claim.
if grep -q -F -e 'decided under issue #493' "$integrations" &&
  grep -q -F -e 'fsharp/tests/fixtures/fsharplint/' "$integrations" &&
  grep -q -F -e 'wire via `FSharpLint.Application.Lint`' "$integrations" &&
  grep -q -F -e 'console-parse is rejected' "$integrations" &&
  grep -q -F -e 'target-coupled' "$integrations" &&
  grep -q -F -e 'sandbox-apply-and-diff' "$integrations" &&
  grep -q -F -e 'ReceivedWarning' "$integrations" &&
  grep -q -F -e 'not silent' "$integrations" &&
  grep -q -F -e 'no adapter claims `scala`, `csharp`, or' "$integrations"; then
  ok
else
  bad "tool-integrations lost its explicit FSharpLint parse-vs-wire plus wiring decision under issue #493"
fi

# Tool acquisition keeps the FSharpLint research row plus the decided
# exact-package route with the decision and no false claim.
if grep -q -F -e '| FSharpLint |' "$acquisition" &&
  grep -q -F -e 'decided under issue #493' "$acquisition" &&
  grep -q -F -e 'Decided route: CSharpier and Fantomas take the' "$acquisition" &&
  grep -q -F -e 'no consumer runs `dotnet tool install`' "$acquisition" &&
  grep -q -F -e 'no adapter claims `csharp` or' "$acquisition" &&
  grep -q -F -e 'maintainer acquisition must establish and record byte identity' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its FSharpLint row plus decided route with #493 decision and no-claim honesty"
fi

# Support matrix records the decision in adapter-input notes plus wire
# formats plus fix modes plus open risks, with fixtures and no Supported claim.
if grep -q -F -e 'decided under issue #493' "$support" &&
  grep -q -F -e 'fsharp/tests/fixtures/fsharplint/' "$support" &&
  grep -q -F -e 'console-parse is rejected' "$support" &&
  grep -q -F -e 'FSharpLint.Application.Lint' "$support" &&
  grep -q -F -e 'no adapter claims' "$support"; then
  ok
else
  bad "support-matrix lost its FSharpLint #493 decision with fixtures in adapter notes plus wire formats plus open risks"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "fsharplint_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:fsharplint_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the fsharplint_qualification wiring (want target plus dogfood-freshness)"
fi

# Verification matrix owns the qualified record under.
if grep -q -F -e 'fsharplint_qualification' "$verify" &&
  grep -q -F -e 'issue #493' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:fsharplint_qualification' "$verify"; then
  ok
else
  bad "verification-matrix lost its #493 FSharpLint wiring qualified record"
fi

# Live proof: F# foundation fixture stays green on the seed host
# (decision only; no adapter behavior yet).
if bazel test //fsharp/tests/fixtures/hello:hello_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "fsharplint live proof failed (want fsharp hello green)"
fi

dx_test_summary "fsharplint qualification harness"
