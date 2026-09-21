#!/usr/bin/env bash
# Scala plus .NET adapters qualification harness (issue #797).
#
# Qualifies the as-built Scala/.NET adapter delivery with fixture evidence
# and owned gaps, without claiming Supported:
# - delivered: six adapters (`scalafmt` format scala, `scalafix` lint scala
#   via callback NDJSON, `csharpier` format csharp, `roslyn` lint csharp via
#   delegated per-pivot SARIF union, `fantomas` format fsharp,
#   `fsharplint` lint fsharp via library NDJSON) over the decided routes
#   (managed JVM over the shared managed JDK plus Scala Maven-lock story
#   with target-coupled semanticdb plus classpath wiring for semantic rules;
#   exact-package plus shared-.NET-runtime as declared DLLs over one managed
#   .NET cohort, never `dotnet tool install`; Roslyn SDK-coupled with no
#   separate artifact); per-tool fixtures with pins plus samples;
#   Layer-2 matrix pass plus fail cells per tool (format via fake shell
#   doubles seed-only, lint via delegated recorded diagnostics like
#   Clippy/rustc); parsers with pass plus fail samples; native-config
#   bindings for the four configurable tools; runner dispatch plus fix flows
#   (formatters whole-file rewrite, lint check-only via
#   sandbox-apply-and-diff); Layer-2 verification cells flipped from Open
#   (adapter-less) to Delivered;
# - open owned gaps: exact artifact digests plus shared-JDK/.NET cohort
#   bounds, platform plus consumer plus release evidence, no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:scala_dotnet_adapters_qualification`,
# following //tools/ci:scala_dotnet_defaults_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

adapters="quality/adapters.bzl"
parity="quality/parity_tests.bzl"
native="quality/native_config.bzl"
matrix="quality/testdata/runner_matrix_scala_dotnet.bzl"
cases="quality/testdata/runner_matrix_cases.bzl"
subjects="quality/testdata/BUILD.bazel"
real_rs="quality/runner/src/real.rs"
commands="quality/adapter/src/commands.rs"
integrations="docs/quality/tool-integrations.md"
runner_doc="docs/quality/runner-matrix.md"
verify="docs/testing/verification-matrix.md"
targets="tools/ci/ci_targets_d.bzl"
dogfood="tools/ci/dogfood_freshness.sh"

# Per-tool fixture dirs stay present (three new format fixtures plus the
# three decided lint fixtures under #490/#492/#493).
if [[ -d "scala/tests/fixtures/scalafmt" && -d "scala/tests/fixtures/scalafix" && -d "csharp/tests/fixtures/csharpier" && -d "csharp/tests/fixtures/roslyn" && -d "fsharp/tests/fixtures/fantomas" && -d "fsharp/tests/fixtures/fsharplint" ]]; then
  ok
else
  bad "scala-dotnet adapter fixtures missing (want scalafmt plus scalafix plus csharpier plus roslyn plus fantomas plus fsharplint dirs)"
fi

# New format fixtures pin versions plus routes plus invocation shapes.
if grep -q -F -e 'SCALAFMT_VERSION = "3.11.4"' scala/tests/fixtures/scalafmt/pins.bzl &&
  grep -q -F -e 'CSHARPIER_VERSION = "1.3.0"' csharp/tests/fixtures/csharpier/pins.bzl &&
  grep -q -F -e 'FANTOMAS_LINE = "7.x stable"' fsharp/tests/fixtures/fantomas/pins.bzl &&
  grep -q -F -e 'compatible JVM artifact' scala/tests/fixtures/scalafmt/pins.bzl &&
  grep -q -F -e 'declared DLLs' csharp/tests/fixtures/csharpier/pins.bzl &&
  grep -q -F -e 'declared DLLs' fsharp/tests/fixtures/fantomas/pins.bzl; then
  ok
else
  bad "format fixtures lost their version plus route pins under issue #797"
fi

# REAL_ADAPTERS claims all six cohort tools with the right families.
if grep -q -F -e '"scalafmt": {"format": ["scala"]}' "$adapters" &&
  grep -q -F -e '"scalafix": {"lint": ["scala"]}' "$adapters" &&
  grep -q -F -e '"csharpier": {"format": ["csharp"]}' "$adapters" &&
  grep -q -F -e '"roslyn": {"lint": ["csharp"]}' "$adapters" &&
  grep -q -F -e '"fantomas": {"format": ["fsharp"]}' "$adapters" &&
  grep -q -F -e '"fsharplint": {"lint": ["fsharp"]}' "$adapters"; then
  ok
else
  bad "REAL_ADAPTERS lost a Scala/.NET cohort claim (want all six under issue #797)"
fi

# Parity no longer defers the three delivered classes.
if ! grep -q -F -e '"scala":' "$parity" &&
  ! grep -q -F -e '"csharp":' "$parity" &&
  ! grep -q -F -e '"fsharp":' "$parity"; then
  ok
else
  bad "parity still defers a delivered Scala/.NET class (want none under issue #797)"
fi

# Matrix carries all twelve cohort cells.
cohort_matrix=""
for cell in matrix_scala_format_pass matrix_scala_format_fail matrix_scala_lint_pass matrix_scala_lint_fail matrix_csharp_format_pass matrix_csharp_format_fail matrix_csharp_lint_pass matrix_csharp_lint_fail matrix_fsharp_format_pass matrix_fsharp_format_fail matrix_fsharp_lint_pass matrix_fsharp_lint_fail; do
  grep -q -F -e "$cell" "$matrix" || cohort_matrix="$cohort_matrix $cell:missing"
done
if [[ -z "$cohort_matrix" ]] && grep -q -F -e 'SCALA_DOTNET_CASES' "$cases"; then
  ok
else
  bad "runner matrix lost Scala/.NET cells:$cohort_matrix (want all twelve under issue #797)"
fi

# Every cohort tool keeps its parser with pass plus fail samples.
cohort_parser=""
for tool in scalafmt scalafix csharpier roslyn fantomas fsharplint; do
  [[ -f "quality/adapter/src/parsers/$tool.rs" ]] || cohort_parser="$cohort_parser $tool:missing"
done
if [[ -z "$cohort_parser" ]] &&
  grep -q -F -e 'pub fn parse_scalafmt' quality/adapter/src/parsers/scalafmt.rs &&
  grep -q -F -e 'pub fn parse_scalafix' quality/adapter/src/parsers/scalafix.rs &&
  grep -q -F -e 'pub fn parse_csharpier' quality/adapter/src/parsers/csharpier.rs &&
  grep -q -F -e 'pub fn parse_roslyn' quality/adapter/src/parsers/roslyn.rs &&
  grep -q -F -e 'pub fn parse_fantomas' quality/adapter/src/parsers/fantomas.rs &&
  grep -q -F -e 'pub fn parse_fsharplint' quality/adapter/src/parsers/fsharplint.rs; then
  ok
else
  bad "parsers lost a Scala/.NET tool:$cohort_parser (want all six under issue #797)"
fi

# Native-config binds the four configurable tools.
if grep -q -F -e 'scalafmt_config' "$native" &&
  grep -q -F -e 'scalafix_config' "$native" &&
  grep -q -F -e 'csharpier_config' "$native" &&
  grep -q -F -e 'fsharplint_config' "$native"; then
  ok
else
  bad "native-config lost a Scala/.NET binding (want scalafmt plus scalafix plus csharpier plus fsharplint under issue #797)"
fi

# Runner dispatches all six tools.
cohort_dispatch=""
for tool in '"scalafmt"' '"scalafix"' '"csharpier"' '"roslyn"' '"fantomas"' '"fsharplint"'; do
  grep -q -F -e "$tool" "$real_rs" || cohort_dispatch="$cohort_dispatch $tool:missing"
done
if [[ -z "$cohort_dispatch" ]]; then
  ok
else
  bad "runner lost Scala/.NET dispatch:$cohort_dispatch (want all six under issue #797)"
fi

# Commands pin the six invocation shapes.
if grep -q -F -e 'pub fn scalafmt_check' "$commands" &&
  grep -q -F -e 'pub fn scalafix_check' "$commands" &&
  grep -q -F -e 'pub fn csharpier_check' "$commands" &&
  grep -q -F -e 'pub fn fantomas_check' "$commands" &&
  grep -q -F -e 'pub fn roslyn_errorlog' "$commands" &&
  grep -q -F -e 'pub fn fsharplint_check' "$commands"; then
  ok
else
  bad "commands lost a Scala/.NET invocation shape (want all six under issue #797)"
fi

# Runner-matrix doc owns the three cohort sections.
if grep -q -F -e '## Scala (`scala`: scalafmt format, scalafix lint)' "$runner_doc" &&
  grep -q -F -e '## C# (`csharp`: csharpier format, roslyn lint)' "$runner_doc" &&
  grep -q -F -e '## F# (`fsharp`: fantomas format, fsharplint lint)' "$runner_doc" &&
  grep -q -F -e 'opt-in adapters delivered under #797' "$runner_doc"; then
  ok
else
  bad "runner-matrix doc lost its Scala/.NET sections under issue #797"
fi

# Tool integrations claim the six adapters over the decided routes.
if grep -q -F -e 'adapters `scalafmt` (format `scala`)' "$integrations" &&
  grep -q -F -e '`csharpier` (format `csharp`)' "$integrations" &&
  grep -q -F -e '`fantomas` (format `fsharp`)' "$integrations" &&
  grep -q -F -e 'adapters qualified seed-only under #797' "$integrations" &&
  grep -q -F -e 'scala_dotnet_adapters_qualification' "$integrations"; then
  ok
else
  bad "tool-integrations lost its Scala/.NET adapter claims under issue #797"
fi

# Verification matrix flips the three Layer-2 cells to Delivered.
if grep -q -F -e '| Scala | Delivered (code ownership) | Delivered |' "$verify" &&
  grep -q -F -e '| C# | Delivered (code ownership) | Delivered |' "$verify" &&
  grep -q -F -e '| F# | Delivered (code ownership) | Delivered |' "$verify" &&
  grep -q -F -e 'Scala/.NET Layer-2 delivered under #797' "$verify"; then
  ok
else
  bad "verification-matrix lost its Scala/.NET Layer-2 Delivered flip under issue #797"
fi

# Verification matrix lists the harness in dogfood-freshness plus Green.
if grep -q -F -e ':scala_dotnet_adapters_qualification' "$verify" &&
  grep -q -F -e '`scala_dotnet_adapters_qualification` 18/18' "$verify"; then
  ok
else
  bad "verification-matrix lost its #797 adapters harness record (want dogfood plus Green 18/18)"
fi

# Targets own the harness plus dogfood wires it.
if grep -q -F -e 'name = "scala_dotnet_adapters_qualification"' "$targets" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:scala_dotnet_adapters_qualification' "$dogfood"; then
  ok
else
  bad "ci_targets or dogfood lost the scala_dotnet_adapters_qualification wiring (want target plus dogfood-freshness)"
fi

# Fake format doubles stay present for the seed-only matrix cells.
if [[ -f "quality/testdata/fake_scalafmt.sh" && -f "quality/testdata/fake_csharpier.sh" && -f "quality/testdata/fake_fantomas.sh" ]] &&
  grep -q -F -e 'fake_scalafmt' "$subjects" &&
  grep -q -F -e 'fake_csharpier' "$subjects" &&
  grep -q -F -e 'fake_fantomas' "$subjects"; then
  ok
else
  bad "matrix fake doubles missing (want three fake_*.sh plus BUILD targets under issue #797)"
fi

# Live proof: adapter plus runner unit suites stay green.
if bazel test //quality/adapter:all //quality/runner:all --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "scala-dotnet adapters live proof failed (want //quality/adapter:all plus //quality/runner:all green)"
fi

# Live proof: the twelve cohort matrix cells stay green.
if bazel test //quality/testdata:matrix_scala_format_pass //quality/testdata:matrix_scala_format_fail //quality/testdata:matrix_scala_lint_pass //quality/testdata:matrix_scala_lint_fail //quality/testdata:matrix_csharp_format_pass //quality/testdata:matrix_csharp_format_fail //quality/testdata:matrix_csharp_lint_pass //quality/testdata:matrix_csharp_lint_fail //quality/testdata:matrix_fsharp_format_pass //quality/testdata:matrix_fsharp_format_fail //quality/testdata:matrix_fsharp_lint_pass //quality/testdata:matrix_fsharp_lint_fail --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "scala-dotnet matrix live proof failed (want twelve cohort cells green)"
fi

# Live proof: foundation fixtures stay green (adapters change only).
if bazel test //scala/tests/fixtures/hello:hello_test //csharp/tests/fixtures/hello:hello_test //fsharp/tests/fixtures/hello:hello_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "scala-dotnet adapters live proof failed (want scala plus csharp plus fsharp hello green)"
fi

dx_test_summary "scala-dotnet adapters qualification harness"
