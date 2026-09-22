#!/usr/bin/env bash
# Scalafix console + semanticdb-classpath wiring qualification.
#
# Decides the open Scalafix adapter-input risk with fixture evidence,
# recorded explicitly here and in the owning docs, never silently dropped
# (silent console parse rejected per the issue alternatives):
# - console-parse REJECTED for diagnostics: lint-only console lines carry
#   rule IDs but rewritable rules emit patch-only unified diff with no rule
#   attribution plus interleaved patch/diagnostic sections, so fail-closed
#   parsing is unprovable (fixtures `console_lint.txt` plus
#   `console_rewrite.txt` modeled on DisableSyntax.var plus ProcedureSyntax;
# open upstream scalacenter/scalafix, report-API scalacenter/scalafix).
# - wire REQUIRED: structured diagnostics plus patches via a custom Java
#   entrypoint binding `scalafix.interfaces.ScalafixMainCallback` over the
#   semantic-rule artifacts on the shared managed JDK plus the Scala
#   Maven-lock story (`maven_install.json` plus `fail_if_repin_required`).
# - target-coupled semanticdb plus classpath wiring REQUIRED for semantic
#   rules (`--classpath` plus `--sourceroot` plus `--semanticdb-targetroots`
#   from the authoritative `scala_*` target context; syntactic-only runs need
#   no target context; missing semantic context fails closed, never a silent
#   check-only fallback).
# - fix flow is sandbox-apply-and-diff with declared per-target outputs as
#   unified patches (never `IN_PLACE` mutation of immutable inputs).
# - native config is sole policy: checked-in `.scalafix.conf` required for
#   OrganizeImports plus RemoveUnused (see `example.scalafix.conf`).
# Adapter-only: no adapter claims `scala` yet (cohort stays owned by;
# decision recorded under with fixtures).
#
# Versioned here, run by CI via `bazel run //tools/ci:scalafix_qualification`.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

pins="scala/tests/fixtures/scalafix/pins.bzl"
pins_build="scala/tests/fixtures/scalafix/BUILD.bazel"
lint_sample="scala/tests/fixtures/scalafix/console_lint.txt"
rewrite_sample="scala/tests/fixtures/scalafix/console_rewrite.txt"
sample_src="scala/tests/fixtures/scalafix/Sample.scala"
sample_conf="scala/tests/fixtures/scalafix/example.scalafix.conf"
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

# Fixture set stays present.
if [[ -f "$pins" && -f "$pins_build" && -f "$lint_sample" && -f "$rewrite_sample" && -f "$sample_src" && -f "$sample_conf" ]]; then
  ok
else
  bad "scalafix fixture missing (want $pins plus $pins_build plus console samples plus Sample.scala plus example config)"
fi

# Pins record the reference version plus managed-JVM artifact (versions
# qualified seed-only under; digests stay owned under).
if grep -q -F -e 'SCALAFIX_VERSION = "0.14.7"' "$pins" &&
  grep -q -F -e 'semantic-rule artifacts over the shared managed JDK' "$pins"; then
  ok
else
  bad "pins.bzl lost its Scalafix 0.14.7 version plus managed-JVM artifact under issue #490"
fi

# Pins record the explicit parse-vs-wire decision plus rejections (silent
# console parse rejected per the issue alternatives).
if grep -q -F -e 'console-parse rejected' "$pins" &&
  grep -q -F -e 'wire via ScalafixMainCallback' "$pins" &&
  grep -q -F -e 'ScalafixMainCallback' "$pins" &&
  grep -q -F -e 'silent console parse rejected' "$pins"; then
  ok
else
  bad "pins.bzl lost its explicit parse-vs-wire record with silent-parse rejection under issue #490"
fi

# Pins record target-coupled semanticdb plus classpath wiring plus fix flow
# plus native-config sole policy.
if grep -q -F -e 'target-coupled' "$pins" &&
  grep -q -F -e '--classpath' "$pins" &&
  grep -q -F -e 'sandbox-apply-and-diff' "$pins" &&
  grep -q -F -e 'IN_PLACE' "$pins" &&
  grep -q -F -e 'checked-in .scalafix.conf' "$pins"; then
  ok
else
  bad "pins.bzl lost its semanticdb plus classpath wiring plus fix-flow plus config-policy record under issue #490"
fi

# Console fixtures prove lossiness: lint lines carry rule IDs, the rewrite
# diff carries no rule attribution (so a console parser would miss or
# misattribute rewritable findings).
if grep -q -F -e '[DisableSyntax.var]' "$lint_sample" &&
  grep -q -F -e '+++' "$rewrite_sample" &&
  ! grep -q -F -e 'DisableSyntax' "$rewrite_sample" &&
  ! grep -q -F -e 'ProcedureSyntax' "$rewrite_sample"; then
  ok
else
  bad "console fixtures lost their lossiness proof (want rule IDs in lint sample, diff-only with no rule ID in rewrite sample)"
fi

# Adapter delivered under #797: scalafix claims scala in REAL_ADAPTERS and
# the runner matrix carries scala cells (green adapter evidence; cohort
# classification stays).
if grep -q -F -e '"scalafix":' "$adapters" &&
  grep -q -F -e 'matrix_scala_' quality/testdata/runner_matrix_scala_dotnet.bzl &&
  grep -q -F -e '"scala": "scala"' "$adapters"; then
  ok
else
  bad "scalafix adapter delivery missing (want scalafix in REAL_ADAPTERS plus matrix_scala_ cells plus scala classified under #797)"
fi

# Tool integrations record the explicit decision (wire required,
# console-parse rejected, target-coupled wiring, sandbox fix flow), never
# silent, with the adapter delivered under #797.
if grep -q -F -e 'decided under issue #490' "$integrations" &&
  grep -q -F -e 'wire via `scalafix.interfaces.ScalafixMainCallback`' "$integrations" &&
  grep -q -F -e 'console-parse is rejected' "$integrations" &&
  grep -q -F -e 'target-coupled `--classpath`' "$integrations" &&
  grep -q -F -e 'sandbox-apply-and-diff' "$integrations" &&
  grep -q -F -e 'no machine-readable CLI output' "$integrations" &&
  grep -q -F -e 'not silent' "$integrations" &&
  grep -q -F -e 'adapters qualified seed-only under #797' "$integrations"; then
  ok
else
  bad "tool-integrations lost its explicit Scalafix parse-vs-wire plus wiring decision under issue #490"
fi

# Tool acquisition keeps the Scalafix research row plus the decided
# managed-JVM route with the decision and no false claim.
if grep -q -F -e '| Scalafix |' "$acquisition" &&
  grep -q -F -e 'decided under issue #490' "$acquisition" &&
  grep -q -F -e 'Decided route: Scalafmt and Scalafix take the' "$acquisition" &&
  grep -q -F -e 'same shared managed JDK and Maven-lock story' "$acquisition" &&
  grep -q -F -e 'adapter claims `scala` yet' "$acquisition" &&
  grep -q -F -e 'maintainer acquisition must establish and record byte identity' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its Scalafix row plus decided route with #490 decision and no-claim honesty"
fi

# Support matrix records the decision in adapter-input notes plus fix
# modes plus open risks, with fixtures and no Supported claim.
# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "scalafix_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:scalafix_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the scalafix_qualification wiring (want target plus dogfood-freshness)"
fi

# Live proof: Scala foundation fixture stays green on the seed host
# (decision only; no adapter behavior yet).
if bazel test //scala/tests/fixtures/hello:hello_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "scalafix live proof failed (want scala hello green)"
fi

dx_test_summary "scalafix qualification harness"
