#!/usr/bin/env bash
# Third-party env plugin model plus Go cgo scope qualification.
#
# Qualifies the two areas it explicitly does not cover:
# - plugin model: third-party language-integration (persistent-environment)
#   plugins stay out of scope with an explicit design owner plus
#   acceptance criteria instead of an owner-less exclusion;
#   repurposing `EnvironmentInfo` as a persistent-environment plugin API is
#   wont-fix in v1, and the private first-party contribution path stays
#   rejected;
# - cgo boundary: the supported Go IDE boundary is pure-Go packages
#   (including declared build constraints and platform source selection)
#   through the pinned upstream rules_go 0.63.0 `GOPACKAGESDRIVER`, with
#   failure propagation, separate IDE output base, exact-target isolation,
#   and no environment/codegen selection or tracked-file mutation; cgo
#   completion plus cgo diagnostics stay the explicit out-of-scope
#   exception because upstream does not guarantee cgo completion, so cgo
#   fixtures record gaps rather than claiming generic IDE parity;
# - cgo scope (issue #789): source-only module identity, strict dependency
#   resolution, and cgo/race scope resolution; generation fails closed on `import "C"`
#   (cgo stays handwritten), generated rules carry no cgo/race scope attrs,
#   handwritten wrappers may set them upstream, and the live
#   `go/tests/fixtures/cgo/` pair proves the scope with race requiring cgo;
# - fixtures: `env/tests/fixtures/env_plugins_cgo/` (`pins.bzl` plus
#   `env_plugins_cgo.expected`) pins dispositions plus rejected routes
#   plus honesty; seed only, no Supported claim.
# - scope: env only; no PATH-tool collision rule change.
#
# Versioned here, run by CI via `bazel run //tools/ci:env_plugins_cgo_qualification`,
# following //tools/ci:env_codegen_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

env_doc="docs/environments/environment.md"
testing_doc="docs/testing/environments.md"
matrix_support="docs/product/support-matrix.md"
pins="env/tests/fixtures/env_plugins_cgo/pins.bzl"
expected="env/tests/fixtures/env_plugins_cgo/env_plugins_cgo.expected"
fixture_build="env/tests/fixtures/env_plugins_cgo/BUILD.bazel"
build="tools/ci/ci_targets_c.bzl"
ci="tools/ci/dogfood_freshness.sh"
verify="docs/testing/verification-matrix.md"

# Plugin model stays Not planned with no v1 plugin model or extension point.
if grep -q -F -e 'Not planned: third-party language-integration plugins' "$env_doc" &&
  grep -q -F -e 'There is no v1 persistent-environment plugin' "$env_doc" &&
  grep -q -F -e 'no `EnvironmentInfo` extension point' "$env_doc"; then
  ok
else
  bad "environment.md lost its Not planned plugin model under #587"
fi

# Repurposing EnvironmentInfo as a persistent-environment plugin API is wont-fix.
if grep -q -F -e 'EnvironmentInfo` stays PATH-tool-only' "$env_doc" &&
  grep -q -F -e 'is not a persistent-environment plugin API' "$env_doc" &&
  grep -q -F -e 'wont-fix' "$env_doc"; then
  ok
else
  bad "environment.md lost its EnvironmentInfo persistent-plugin wont-fix under #587"
fi

# No private first-party contribution path.
if grep -q -F -e 'no private first-party contribution path' "$env_doc"; then
  ok
else
  bad "environment.md lost its no-private-path record under #587"
fi

# Public PATH-tool API stays pinned to the same validated constructors.
if grep -q -F -e 'environment_tool(name, executable, bin_name)' "$env_doc" &&
  grep -q -F -e 'environment_tool' env/defs.bzl &&
  grep -q -F -e 'EnvironmentInfo' env/defs.bzl &&
  [[ -f "env/defs_tests.bzl" ]]; then
  ok
else
  bad "public PATH-tool API lost its defs.bzl pin under #587"
fi

# Plugin model stays pinned by the env_plugins_cgo fixtures with no Supported claim.
if grep -q -F -e 'Pinned by fixtures in' "$env_doc" &&
  grep -q -F -e 'env/tests/fixtures/env_plugins_cgo/' "$env_doc" &&
  grep -q -F -e 'env_plugins_cgo_qualification' "$env_doc"; then
  ok
else
  bad "environment.md lost its plugin fixture pin under #587"
fi

# Go driver reuse stays upstream with no static snapshot or replacement graph.
if grep -q -F -e 'GOPACKAGESDRIVER' "$env_doc" &&
  grep -q -F -e 'rather than creating a static package' "$env_doc" &&
  grep -q -F -e 'or replacement package graph' "$env_doc"; then
  ok
else
  bad "environment.md lost its GOPACKAGESDRIVER plus no-snapshot record under #587"
fi

# Supported boundary stays pure-Go with constraints plus platform on pinned rules_go.
if grep -q -F -e 'pure-Go packages' "$env_doc" &&
  grep -q -F -e 'build constraints and platform source selection' "$env_doc" &&
  grep -q -F -e 'rules_go 0.63.0' "$env_doc"; then
  ok
else
  bad "environment.md lost its pure-Go plus constraints plus rules_go pin under #587"
fi

# Failure propagation plus separate base plus no mutation stay recorded.
if grep -q -F -e 'propagate failures rather' "$env_doc" &&
  grep -q -F -e 'Use a separate IDE output base' "$env_doc" &&
  grep -q -F -e 'do not mutate the selected environment' "$env_doc"; then
  ok
else
  bad "environment.md lost its failure-propagation plus separate-base plus no-mutation record under #587"
fi

# cgo stays the explicit exception with upstream non-guarantee and gap recording.
if grep -q -F -e 'cgo completion is out of scope' "$env_doc" &&
  grep -q -F -e 'does not admit the complete Go foundation' "$env_doc" &&
  grep -q -F -e 'upstream does not guarantee cgo' "$env_doc" &&
  grep -q -F -e 'record cgo and platform gaps' "$env_doc"; then
  ok
else
  bad "environment.md lost its explicit cgo exception boundary under #587"
fi

# Test requirements carry the cgo gap plus the fixture wiring.
if grep -q -F -e 'record cgo and platform gaps' "$env_doc" &&
  grep -q -F -e 'env/tests/fixtures/env_plugins_cgo/' "$env_doc" &&
  grep -q -F -e 'env_plugins_cgo_qualification' "$env_doc"; then
  ok
else
  bad "environment.md Test Requirements lost its cgo plus #587 fixture wiring"
fi

# Testing matrix wires the explicit cgo exception boundary under.
if grep -q -F -e 'explicit cgo exception boundary' "$testing_doc" &&
  grep -q -F -e 'env/tests/fixtures/env_plugins_cgo/' "$testing_doc" &&
  grep -q -F -e 'does not guarantee cgo completion' "$testing_doc" &&
  grep -q -F -e '#587' "$testing_doc"; then
  ok
else
  bad "testing/environments.md lost its explicit cgo boundary wiring under #587"
fi

# Support matrix wires the pure-Go boundary plus cgo exception under.
if grep -q -F -e 'explicit cgo exception' "$matrix_support" &&
  grep -q -F -e 'env/tests/fixtures/env_plugins_cgo/' "$matrix_support" &&
  grep -q -F -e 'does not guarantee cgo completion' "$matrix_support" &&
  grep -q -F -e '#587' "$matrix_support"; then
  ok
else
  bad "support-matrix.md lost its cgo exception wiring under #587"
fi

# Fixture pins stay present with plugin plus cgo plus rejected plus honesty.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'PLUGIN_DISPOSITION = "out of scope"' "$pins" &&
  grep -q -F -e 'PLUGIN_OWNER = "//env plus //cli/env"' "$pins" &&
  grep -q -F -e 'PLUGIN_ENV_INFO_WONT_FIX = "EnvironmentInfo stays PATH-tool-only"' "$pins" &&
  grep -q -F -e 'PLUGIN_NO_PRIVATE_PATH' "$pins" &&
  grep -q -F -e 'GO_DRIVER = "GOPACKAGESDRIVER"' "$pins" &&
  grep -q -F -e 'CGO_OUT_OF_SCOPE = "cgo completion is out of scope"' "$pins" &&
  grep -q -F -e 'CGO_UPSTREAM_NON_GUARANTEE' "$pins" &&
  grep -q -F -e 'REJECTED_STATIC_SNAPSHOT' "$pins" &&
  grep -q -F -e 'REJECTED_GENERIC_PARITY' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #587' "$pins"; then
  ok
else
  bad "env_plugins_cgo pins fixture lost its plugin plus cgo plus rejected wiring under #587"
fi

# Expected fixture pins the decision plus boundary plus rejected plus honesty lines.
if grep -q -F -e 'out of scope with design owner' "$expected" &&
  grep -q -F -e 'is not a persistent-environment plugin API' "$expected" &&
  grep -q -F -e 'no private first-party contribution path' "$expected" &&
  grep -q -F -e 'GOPACKAGESDRIVER on rules_go 0.63.0' "$expected" &&
  grep -q -F -e 'cgo completion is out of scope' "$expected" &&
  grep -q -F -e 'does not guarantee cgo completion' "$expected" &&
  grep -q -F -e 'static package snapshot' "$expected" &&
  grep -q -F -e 'generic IDE parity' "$expected" &&
  grep -q -F -e 'qualified seed-only under issue #587' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "env_plugins_cgo.expected lost its decision plus boundary plus honesty lines under #587"
fi

# Docs plus build plus CI plus matrix own the qualified seed-only record under.
if grep -q -F -e 'env/tests/fixtures/env_plugins_cgo/pins.bzl' "$env_doc" &&
  grep -q -F -e 'qualified seed-only' "$env_doc" &&
  grep -q -F -e 'issue #587' "$env_doc" &&
  grep -q -F -e 'env_plugins_cgo_qualification' "$build" &&
  grep -q -F -e 'env_plugins_cgo_qualification' "$ci" &&
  grep -q -F -e 'env_plugins_cgo_qualification' "$verify" &&
  grep -q -F -e '#587' "$verify"; then
  ok
else
  bad "docs/build/CI/matrix lost the #587 qualified seed-only wiring"
fi

# Live proof: the fixture plus the focused Go env plan build green.
if bazel build //env/tests/fixtures/env_plugins_cgo/... //go/env:hello_lib_plan --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "env_plugins_cgo fixture plus Go env plan build failed (want green on the seed host, issue #587)"
fi

# Go cgo scope stays handwritten in generation (issue #789).
if grep -q -F -e 'imports cgo' gazelle/go/lang.go &&
  grep -q -F -e 'IsCgoImport' gazelle/go/parser.go &&
  grep -q -F -e 'gazelle:exclude go/tests/fixtures/cgo' BUILD.bazel; then
  ok
else
  bad "Go generation lost its handwritten cgo fail-closed scope under #789"
fi

# Generated rules stay pure-Go without cgo/race scope attrs (issue #789).
if grep -q -F -e 'cgo/race scope attrs' gazelle/go/lang.go &&
  grep -q -F -e 'Handwritten wrappers may set upstream cgo scope attrs' go/rules/defs.bzl; then
  ok
else
  bad "Go generation lost its pure-Go no-cgo-race-attrs scope under #789"
fi

# Source-only module identity stays pinned (issue #789).
if grep -q -F -e 'SOURCE_ONLY_IMPORTPATH' "$pins" &&
  grep -q -F -e 'SOURCE_ONLY_OMITTED' "$pins" &&
  grep -q -F -e 'SOURCE_ONLY_NO_GUESS' "$pins" &&
  grep -q -F -e 'goImportPath' gazelle/go/lang.go &&
  grep -q -F -e 'omits importpath' gazelle/go/lang.go; then
  ok
else
  bad "Go source-only module identity lost its pins plus generation wiring under #789"
fi

# Strict dependency resolution stays pinned (issue #789).
if grep -q -F -e 'STRICT_LOCAL_STEMS' "$pins" &&
  grep -q -F -e 'STRICT_THIRD_PARTY_EXACT' "$pins" &&
  grep -q -F -e 'STRICT_STDLIB_FILTERED' "$pins" &&
  grep -q -F -e 'STRICT_UNRESOLVED_FAILS' "$pins" &&
  grep -q -F -e 'STRICT_AMBIGUOUS_FAILS' "$pins" &&
  grep -q -F -e 'STRICT_TEST_EMBED' "$pins"; then
  ok
else
  bad "Go strict dependency resolution lost its pins wiring under #789"
fi

# Handwritten cgo fixture stays present with cgo plus race scope (issue #789).
if [[ -f "go/tests/fixtures/cgo/BUILD.bazel" && -f "go/tests/fixtures/cgo/pins.bzl" ]] &&
  grep -q -F -e 'cgo = True' go/tests/fixtures/cgo/BUILD.bazel &&
  grep -q -F -e 'race = "on"' go/tests/fixtures/cgo/BUILD.bazel &&
  grep -q -F -e 'CGO_LIB = "//go/tests/fixtures/cgo:cgo"' go/tests/fixtures/cgo/pins.bzl &&
  grep -q -F -e 'RACE_REQUIRES_CGO' go/tests/fixtures/cgo/pins.bzl; then
  ok
else
  bad "go/tests/fixtures/cgo lost its handwritten cgo plus race scope under #789"
fi

# Env pins carry the cgo plus race scope with the live fixture labels (issue #789).
if grep -q -F -e 'CGO_SCOPE_HANDWRITTEN' "$pins" &&
  grep -q -F -e 'CGO_SCOPE_FAILS_CLOSED' "$pins" &&
  grep -q -F -e 'CGO_SCOPE_NO_GENERATED_ATTRS' "$pins" &&
  grep -q -F -e 'CGO_SCOPE_PASSTHROUGH' "$pins" &&
  grep -q -F -e 'CGO_FIXTURE_LIB = "//go/tests/fixtures/cgo:cgo"' "$pins" &&
  grep -q -F -e 'RACE_REQUIRES_CGO' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #789' "$pins"; then
  ok
else
  bad "env_plugins_cgo pins lost its cgo plus race scope wiring under #789"
fi

# Expected fixture pins the #789 scope plus honesty lines.
if grep -q -F -e 'source-only module identity' "$expected" &&
  grep -q -F -e 'strict dependency resolution' "$expected" &&
  grep -q -F -e 'cgo stays handwritten' "$expected" &&
  grep -q -F -e 'never cgo scope attrs' "$expected" &&
  grep -q -F -e 'race requires cgo' "$expected" &&
  grep -q -F -e 'qualified seed-only under issue #789' "$expected"; then
  ok
else
  bad "env_plugins_cgo.expected lost its #789 scope plus honesty lines"
fi

# Live proof: the handwritten cgo scope builds and tests green on the seed host.
if bazel build //go/tests/fixtures/cgo/... --noshow_progress >/dev/null 2>&1 &&
  bazel test //go/tests/fixtures/cgo:cgo_test --test_output=errors >/dev/null 2>&1; then
  ok
else
  bad "go/tests/fixtures/cgo build plus test failed (want green on the seed host, issue #789)"
fi

dx_test_summary "env plugins plus cgo qualification harness"
