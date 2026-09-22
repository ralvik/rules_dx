#!/usr/bin/env bash
# xUnit v3 runner qualification harness.
#
# Qualifies the owned gap from support-matrix provisional test runners:
# C#/F# named xUnit v3 4.0.0 with closed as owner only, and covers
# adapters not the runner version.
# - decided: xUnit v3 4.0.0 plus xunit.analyzers 2.0.0 from the 4.0.0 release
#   notes. Pins live in `third_party/dotnet/paket.dependencies`
#   (`nuget xunit.v3 4.0.0`, `nuget xunit.analyzers 2.0.0`) with the MTP
#   runner transitive closure pinned in `paket.lock` and per-package sha512
#   in `third_party/dotnet/deps/paket.main.bzl` (single shared Paket lock
#   for C# plus F#). Unpinned runner rejected.
# - mapping: plain `csharp_test`/`fsharp_test` executables with
#   `[Fact]`/`[Theory]` (`[<Fact>]` in F#) sources plus checked-in MTP
#   entry-point shims (`XunitEntryPoint` plus `SelfRegisteredExtensions`,
#   adapted from `dotnet new xunit3` MSBuild output because rules_dotnet
#   compiles with csc/fsc, not MSBuild). The `xunit.v3` meta-package is
#   empty; strict-deps compilation names the transitive hub labels
#   directly. Proven by `csharp/tests/fixtures/xunit/` and
#   `fsharp/tests/fixtures/xunit/` under `bazel test` (In-Process Runner
#   v4.0.0 on net10.0). Hello plain-executable fixtures stay as smoke
#   coverage, not the runner mapping.
# - scope: test only. Per-platform SDK acquisition plus Paket lock wiring
#   beyond the runner closure, platform plus consumer plus release
#   evidence, and NUnit/MSTest secondary runners stay open; no Supported
#   claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:xunit_qualification`,
# following //tools/ci:exact_target_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

deps="third_party/dotnet/paket.dependencies"
lock="third_party/dotnet/paket.lock"
hub="third_party/dotnet/deps/paket.main.bzl"
cs_build="csharp/tests/fixtures/xunit/BUILD.bazel"
cs_test="csharp/tests/fixtures/xunit/GreeterTest.cs"
cs_entry="csharp/tests/fixtures/xunit/XunitEntryPoint.cs"
cs_ext="csharp/tests/fixtures/xunit/SelfRegisteredExtensions.cs"
fs_build="fsharp/tests/fixtures/xunit/BUILD.bazel"
fs_test="fsharp/tests/fixtures/xunit/GreeterTest.fs"
fs_entry="fsharp/tests/fixtures/xunit/XunitEntryPoint.fs"
fs_ext="fsharp/tests/fixtures/xunit/SelfRegisteredExtensions.fs"
matrix="docs/product/support-matrix.md"
gen_readme="docs/generation/foundation-qualification.md"
module="MODULE.bazel"
cs_defs="csharp/rules/defs.bzl"
fs_defs="fsharp/rules/defs.bzl"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Paket pins the runner plus analyzers exactly.
if grep -q -F -e 'nuget xunit.v3 4.0.0' "$deps" &&
  grep -q -F -e 'nuget xunit.analyzers 2.0.0' "$deps"; then
  ok
else
  bad "paket.dependencies lost the xunit.v3 4.0.0 plus xunit.analyzers 2.0.0 pins"
fi

# Paket lock pins the transitive MTP runner closure.
if grep -q -F -e 'xunit.v3 (4.0)' "$lock" &&
  grep -q -F -e 'xunit.analyzers (2.0)' "$lock" &&
  grep -q -F -e 'xunit.v3.core.mtp-v2' "$lock" &&
  grep -q -F -e 'xunit.v3.runner.inproc.console' "$lock" &&
  grep -q -F -e 'Microsoft.Testing.Platform (' "$lock"; then
  ok
else
  bad "paket.lock lost the xUnit v3 plus MTP transitive pins"
fi

# Generated hub carries per-package sha512 plus exact versions.
if grep -q -F -e '"name": "xunit.v3", "id": "xunit.v3", "version": "4.0.0"' "$hub" &&
  grep -q -F -e '"name": "xunit.analyzers", "id": "xunit.analyzers", "version": "2.0.0"' "$hub" &&
  grep -q -F -e 'sha512-' "$hub"; then
  ok
else
  bad "paket.main.bzl lost the xunit.v3 4.0.0 plus analyzers 2.0.0 sha512 pins"
fi

# C# fixture maps csharp_test over the pinned hub labels.
if grep -q -F -e 'csharp_test' "$cs_build" &&
  grep -q -F -e '@paket.main//xunit.v3' "$cs_build" &&
  grep -q -F -e '@paket.main//xunit.analyzers' "$cs_build" &&
  grep -q -F -e '@paket.main//xunit.v3.runner.inproc.console' "$cs_build" &&
  grep -q -F -e '@paket.main//microsoft.testing.platform' "$cs_build"; then
  ok
else
  bad "csharp xunit fixture lost its csharp_test plus pinned hub mapping"
fi

# C# test sources use xUnit facts plus theories.
if grep -q -F -e '[Fact]' "$cs_test" &&
  grep -q -F -e '[Theory]' "$cs_test" &&
  grep -q -F -e 'Assert.Equal' "$cs_test"; then
  ok
else
  bad "csharp xunit fixture lost its [Fact]/[Theory] plus Assert mapping"
fi

# C# entry-point shim routes direct plus MTP server modes.
if grep -q -F -e 'ConsoleRunner.Run' "$cs_entry" &&
  grep -q -F -e 'TestPlatformTestFramework.RunAsync' "$cs_entry" &&
  grep -q -F -e 'SelfRegisteredExtensions' "$cs_entry"; then
  ok
else
  bad "csharp xunit entry point lost its console plus MTP mapping"
fi

# C# MTP hook registers the pinned extensions.
if grep -q -F -e 'AddSelfRegisteredExtensions' "$cs_ext" &&
  grep -q -F -e 'TestingPlatformBuilderHook.AddExtensions' "$cs_ext"; then
  ok
else
  bad "csharp SelfRegisteredExtensions lost its MTP hook mapping"
fi

# F# fixture maps fsharp_test over the pinned hub labels.
if grep -q -F -e 'fsharp_test' "$fs_build" &&
  grep -q -F -e '@paket.main//xunit.v3' "$fs_build" &&
  grep -q -F -e '@paket.main//xunit.analyzers' "$fs_build" &&
  grep -q -F -e '@paket.main//xunit.v3.runner.inproc.console' "$fs_build" &&
  grep -q -F -e '@paket.main//fsharp.core' "$fs_build"; then
  ok
else
  bad "fsharp xunit fixture lost its fsharp_test plus pinned hub mapping"
fi

# F# test sources use xUnit facts.
if grep -q -F -e '[<Fact>]' "$fs_test" &&
  grep -q -F -e 'Assert.Equal' "$fs_test"; then
  ok
else
  bad "fsharp xunit fixture lost its [<Fact>] plus Assert mapping"
fi

# F# entry-point shim stays last-ordered with an entry point.
if grep -q -F -e 'ConsoleRunner.Run' "$fs_entry" &&
  grep -q -F -e '[<EntryPoint>]' "$fs_entry"; then
  ok
else
  bad "fsharp xunit entry point lost its console mapping or entry point"
fi

# F# MTP hook registers the pinned extensions.
if grep -q -F -e 'AddSelfRegisteredExtensions' "$fs_ext" &&
  grep -q -F -e 'TestingPlatformBuilderHook.AddExtensions' "$fs_ext"; then
  ok
else
  bad "fsharp SelfRegisteredExtensions lost its MTP hook mapping"
fi

# MODULE owns the pinned runner, not an open selection.
if grep -q -F -e 'xUnit v3' "$module" &&
  grep -q -F -e '4.0.0 runner is pinned' "$module" &&
  ! grep -q -F -e 'xUnit/NUnit runner selection stays open' "$module"; then
  ok
else
  bad "MODULE.bazel lost its xUnit v3 4.0.0 pinned record"
fi

# Wrappers own the qualified mapping, not an open selection.
if grep -q -F -e 'xUnit v3 4.0.0' "$cs_defs" &&
  grep -q -F -e 'xUnit v3 4.0.0' "$fs_defs" &&
  ! grep -q -F -e 'selection stays open' "$cs_defs" &&
  ! grep -q -F -e 'selection stays open' "$fs_defs"; then
  ok
else
  bad "csharp/fsharp wrappers lost their qualified mapping record"
fi

# Generation README owns the qualified mapping with the harness.
if grep -q -F -e 'qualified under issue #477' "$gen_readme" &&
  grep -q -F -e 'csharp/tests/fixtures/xunit/' "$gen_readme" &&
  grep -q -F -e 'bazel run //tools/ci:xunit_qualification' "$gen_readme"; then
  ok
else
  bad "generation README lost its #477 qualified mapping record"
fi

# Harness stays wired in BUILD plus CI.
if grep -q -F -e 'name = "xunit_qualification"' "$build" &&
  grep -q -F -e '//tools/ci:xunit_qualification' "$ci"; then
  ok
else
  bad "xunit_qualification lost its BUILD plus CI wiring"
fi

dx_test_summary "xUnit v3 qualification harness"
