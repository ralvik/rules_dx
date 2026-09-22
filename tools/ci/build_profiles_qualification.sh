#!/usr/bin/env bash
# Build-profile flags plus DX_PROFILE forwarding qualification harness.
#
# Qualifies the as-built remainder of ADR 0021 with fixture evidence
# (see docs/cli/commands/build-test-coverage.md#build-profiles plus
# docs/deploy/authoring.md plus docs/testing/cli.md):
# - flags: `dx build`, `dx run`, `dx test`, and `dx deploy` accept
#   `--debug`/`--release` (mutually exclusive, exit 2); every other
#   command rejects them as unsupported; there is no `--dev` flag;
#   `dx coverage` takes no profile flags with unchanged argv;
# - configs: `--debug` maps to `--config=dx_debug` (`dbg`), bare maps to
#   `--config=dx_dev` (`fastbuild`), `--release` maps to
#   `--config=dx_release` (`opt`); defaults are `dev` for build/run/test
#   and `release` for deploy;
# - precedence plus forwarding: explicit flag over deploy target
#   `profile` over command default, with `DX_PROFILE=debug|dev|release`
#   forwarded on the deploy run env; deploy target vocabulary comes from
#   `DxDeployInfo` (`debug`, `dev`, `release`);
# - fixtures: `cli/cli/tests/fixtures/build_profiles/` (`pins.bzl` plus
#   `build_profiles.expected`) plus unit fixtures in
#   `cli/cli/src/args/profile.rs` plus `cli/cli/src/plan/workflow.rs`
#   plus `cli/cli/src/plan/run_deploy.rs` plus
#   `cli/cli/src/exec/deploy.rs` plus `deploy/rules/deploy_tests.bzl`;
# - scope: CLI plus deploy provider only; deploy macro slices stay under
#   #723-#730 and shell divergence under #748. Seed only: no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:build_profiles_qualification`,
# following //tools/ci:cli_strict_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

command_doc="docs/cli/commands/build-test-coverage.md"
authoring="docs/deploy/authoring.md"
testing="docs/testing/cli.md"
adr="docs/decisions/0021-build-profiles.md"
pins="cli/cli/tests/fixtures/build_profiles/pins.bzl"
expected="cli/cli/tests/fixtures/build_profiles/build_profiles.expected"
fixture_build="cli/cli/tests/fixtures/build_profiles/BUILD.bazel"
grammar="cli/cli/src/args/grammar.rs"
parser="cli/cli/src/args/parser.rs"
profile="cli/cli/src/args/profile.rs"
workflow="cli/cli/src/plan/workflow.rs"
run_deploy="cli/cli/src/plan/run_deploy.rs"
deploy_exec="cli/cli/src/exec/deploy.rs"
defs="deploy/rules/defs.bzl"
deploy_tests="deploy/rules/deploy_tests.bzl"

# Command doc owns the build-profiles record with fixtures plus qualification.
if grep -q -F -e '## Build Profiles' "$command_doc" &&
  grep -q -F -e 'cli/cli/tests/fixtures/build_profiles/' "$command_doc" &&
  grep -q -F -e 'bazel run //tools/ci:build_profiles_qualification' "$command_doc" &&
  grep -q -F -e 'qualified seed-only under issue #814' "$command_doc" &&
  grep -q -F -e 'DX_PROFILE=debug|dev|release' "$command_doc"; then
  ok
else
  bad "build-test-coverage.md lost its #814 build-profiles record with fixtures plus qualification"
fi

# Authoring owns the deploy profile plus DX_PROFILE record with fixtures plus qualification.
if grep -q -F -e 'explicit `--debug`/`--release` flag always wins' "$authoring" &&
  grep -q -F -e 'DX_PROFILE' "$authoring" &&
  grep -q -F -e 'cli/cli/tests/fixtures/build_profiles/' "$authoring" &&
  grep -q -F -e 'bazel run //tools/ci:build_profiles_qualification' "$authoring" &&
  grep -q -F -e 'qualified seed-only under issue #814' "$authoring"; then
  ok
else
  bad "authoring.md lost its #814 deploy profile plus DX_PROFILE record with fixtures plus qualification"
fi

# Testing matrix pins the build-profile plus forwarding behavior.
if grep -q -F -e 'build-profiles' "$testing" &&
  grep -q -F -e 'cli/cli/tests/fixtures/build_profiles/' "$testing" &&
  grep -q -F -e 'bazel run //tools/ci:build_profiles_qualification' "$testing" &&
  grep -q -F -e 'issue #814' "$testing" &&
  grep -q -F -e 'DX_PROFILE' "$testing"; then
  ok
else
  bad "testing/cli.md lost its #814 build-profiles plus forwarding pin"
fi

# ADR keeps the shared-config scope with the #814 successor for the CLI plus deploy remainder.
if grep -q -F -e 'issue #814' "$adr" &&
  grep -q -F -e 'cli/cli/tests/fixtures/build_profiles/' "$adr"; then
  ok
else
  bad "0021-build-profiles.md lost its #814 successor plus fixture pointer"
fi

# Fixture pins stay present with flags plus configs plus defaults plus precedence plus forwarding.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'PROFILE_COMMANDS = [' "$pins" &&
  grep -q -F -e 'PROFILE_FLAGS = ["--debug", "--release"]' "$pins" &&
  grep -q -F -e 'PROFILE_DEBUG_CONFIG = "dx_debug"' "$pins" &&
  grep -q -F -e 'PROFILE_DEV_CONFIG = "dx_dev"' "$pins" &&
  grep -q -F -e 'PROFILE_RELEASE_CONFIG = "dx_release"' "$pins" &&
  grep -q -F -e 'PROFILE_DEFAULT_DEPLOY = "release"' "$pins" &&
  grep -q -F -e 'PROFILE_PRECEDENCE = "flag over attr over default"' "$pins" &&
  grep -q -F -e 'PROFILE_NO_DEV_FLAG = True' "$pins" &&
  grep -q -F -e 'PROFILE_COVERAGE_UNCHANGED = True' "$pins" &&
  grep -q -F -e 'DX_PROFILE_ENV = "DX_PROFILE"' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #814' "$pins"; then
  ok
else
  bad "build_profiles pins fixture lost its flags plus forwarding wiring under #814"
fi

# Expected pins the flags plus forwarding plus rejected plus honesty lines.
if grep -q -F -e '--debug selects dx_debug' "$expected" &&
  grep -q -F -e 'bare deploy means dx_release' "$expected" &&
  grep -q -F -e 'no --dev flag' "$expected" &&
  grep -q -F -e 'coverage takes no profile flags' "$expected" &&
  grep -q -F -e 'DX_PROFILE forwarded' "$expected" &&
  grep -q -F -e 'qualified seed-only under issue #814' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "build_profiles.expected lost its flags plus forwarding plus honesty lines under #814"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'build_profiles.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "build_profiles BUILD.bazel lost its pins plus expected exports with corpus under #814"
fi

# Grammar owns --debug/--release as profile flags.
if grep -q -F -e 'pub(crate) debug: bool' "$grammar" &&
  grep -q -F -e 'pub(crate) release: bool' "$grammar" &&
  grep -q -F -e 'Use dx_debug config' "$grammar" &&
  grep -q -F -e 'Use dx_release config' "$grammar"; then
  ok
else
  bad "args/grammar.rs lost its --debug/--release profile flags"
fi

# Parser keeps the four-command ownership plus mutual exclusion.
if grep -q -F -e 'Command::Build | Command::Run | Command::Test | Command::Deploy' "$parser" &&
  grep -q -F -e 'ConflictingProfiles' "$parser" &&
  grep -q -F -e 'profile_flags_parse_on_build_run_test' "$profile"; then
  ok
else
  bad "args/parser.rs lost its four-command profile ownership plus conflict pin"
fi

# Profile vocabulary keeps the shared configs plus precedence plus DX_PROFILE name.
if grep -q -F -e 'pub enum Profile' "$profile" &&
  grep -q -F -e '"dx_debug"' "$profile" &&
  grep -q -F -e '"dx_dev"' "$profile" &&
  grep -q -F -e '"dx_release"' "$profile" &&
  grep -q -F -e 'pub const DX_PROFILE_ENV' "$profile" &&
  grep -q -F -e 'pub fn resolve_profile' "$profile" &&
  grep -q -F -e 'profile_vocabulary_maps_to_shared_configs' "$profile" &&
  grep -q -F -e 'profile_precedence_is_flag_over_attr_over_default' "$profile"; then
  ok
else
  bad "args/profile.rs lost its vocabulary plus precedence plus DX_PROFILE fixtures"
fi

# Workflow planning pins the config flag with coverage unchanged.
if grep -q -F -e 'profile.config_flag()' "$workflow" &&
  grep -q -F -e 'workflow_profile_pins_config_flag_in_order' "$workflow" &&
  grep -q -F -e 'coverage' "$workflow"; then
  ok
else
  bad "plan/workflow.rs lost its config-flag plus coverage-unchanged pin"
fi

# Run/deploy planning pins the config flag on both build and run argv.
if grep -q -F -e 'profile.config_flag()' "$run_deploy" &&
  grep -q -F -e 'run_profile_pins_config_flag' "$run_deploy" &&
  grep -q -F -e 'DX_PROFILE' "$deploy_exec" &&
  grep -q -F -e 'deploy_flag_over_attr_precedence' "$deploy_exec"; then
  ok
else
  bad "plan/run_deploy.rs plus exec/deploy.rs lost their config plus DX_PROFILE forwarding pins"
fi

# Deploy provider keeps the profile vocabulary with analysis failure on invalid.
if grep -q -F -e 'DxDeployInfo = provider' "$defs" &&
  grep -q -F -e 'VALID_DEPLOY_PROFILES = ["debug", "dev", "release"]' "$defs" &&
  grep -q -F -e 'deploy_profile_error' "$defs" &&
  grep -q -F -e 'profile vocabulary pins debug, dev, release' "$deploy_tests"; then
  ok
else
  bad "deploy/rules/defs.bzl lost its DxDeployInfo plus profile vocabulary pin"
fi

dx_test_summary "build_profiles_qualification"
