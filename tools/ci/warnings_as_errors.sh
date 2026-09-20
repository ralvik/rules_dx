#!/usr/bin/env bash
# Warnings-as-errors enforcement gate.
#
# Pins warnings-as-errors per toolchain and proves violation fixtures
# exist. Rust uses --deny=warnings; C/C++ uses -Werror; Java uses
# -Werror plus -Xlint:all; Kotlin uses warn=error; Scala uses
# -Xfatal-warnings; C#/F# use treat_warnings_as_errors; Go uses go vet;
# Python uses PYTHONWARNINGS=error plus Ruff; JS/TS use Biome plus tsc
# strict. Fixtures in tools/ci/testdata/warnings/ carry one warning
# pattern per language as uncompiled data; the gate proves each fixture
# and each pin is present.
#
# Versioned here, run by CI via `bazel run //tools/ci:warnings_as_errors`,
# following //tools/ci:build_hygiene.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

bazelrc=".bazelrc"
doc="docs/quality/warnings-as-errors.md"

# Root pins: Rust plus Java plus Python runtime (C/C++ stays wrapper-only
# so third-party external compiles stay green).
if grep -q -F -e '--@rules_rust//rust/settings:extra_rustc_flags=--deny=warnings' "$bazelrc" &&
  grep -q -F -e '--javacopt=-Werror' "$bazelrc" &&
  grep -q -F -e '--javacopt=-Xlint:all' "$bazelrc" &&
  grep -q -F -e '--test_env=PYTHONWARNINGS=error' "$bazelrc" &&
  grep -q -F -e 'issue #455' "$bazelrc"; then
  ok
else
  bad "root .bazelrc lost warnings-as-errors pins (want rust --deny=warnings plus java -Werror/-Xlint:all plus PYTHONWARNINGS=error, issue #455)"
fi

# C/C++ wrapper defaults to -Werror with no global --copt pin.
if grep -q -F -e '-Werror' cc/rules/defs.bzl &&
  grep -q -F -e '_cc_with_werror' cc/rules/defs.bzl &&
  ! grep -q -F -e '--copt=-Werror' "$bazelrc"; then
  ok
else
  bad "cc pin drifted (want wrapper _cc_with_werror -Werror with no global --copt, issue #455)"
fi

# Java wrapper defaults to -Werror plus -Xlint:all.
if grep -q -F -e '-Werror' java/rules/defs.bzl &&
  grep -q -F -e '-Xlint:all' java/rules/defs.bzl &&
  grep -q -F -e '_java_with_werror' java/rules/defs.bzl; then
  ok
else
  bad "java/rules/defs.bzl lost its -Werror/-Xlint:all default (want _java_with_werror, issue #455)"
fi

# Kotlin wrapper defaults to the warnings_as_errors kotlinc options.
if grep -q -F -e 'warnings_as_errors' kotlin/rules/defs.bzl &&
  grep -q -F -e 'kt_kotlinc_options' kotlin/rules/BUILD.bazel &&
  grep -q -F -e 'warn = "error"' kotlin/rules/BUILD.bazel &&
  grep -q -F -e 'language_version = "2.2"' kotlin/rules/BUILD.bazel &&
  grep -q -F -e 'api_version = "2.2"' kotlin/rules/BUILD.bazel; then
  ok
else
  bad "kotlin/rules lost its warn=error default (want kotlinc_opts warnings_as_errors with 2.2, issue #455)"
fi

# Scala wrapper defaults to -Xfatal-warnings.
if grep -q -F -e '-Xfatal-warnings' scala/rules/defs.bzl &&
  grep -q -F -e '_scala_with_werror' scala/rules/defs.bzl; then
  ok
else
  bad "scala/rules/defs.bzl lost its -Xfatal-warnings default (want _scala_with_werror, issue #455)"
fi

# .NET wrappers treat warnings as errors.
if grep -q -F -e 'treat_warnings_as_errors' csharp/rules/defs.bzl &&
  grep -q -F -e 'treat_warnings_as_errors' fsharp/rules/defs.bzl; then
  ok
else
  bad "csharp/fsharp wrappers lost treat_warnings_as_errors (want True default, issue #455)"
fi

# TypeScript fixtures pin strict; JS/TS lint stays --fail_on warning.
if grep -q -F -e '"strict"' typescript/tests/fixtures/hello/tsconfig.json &&
  grep -q -F -e '"strict"' typescript/tests/fixtures/entries/tsconfig.json; then
  ok
else
  bad "typescript fixtures lost strict (want strict true in hello plus entries tsconfig, issue #455)"
fi

# Doc owns the per-toolchain pin table plus gate plus fixtures.
if [[ -f "$doc" ]] &&
  grep -q -F -e 'Per-toolchain' "$doc" &&
  grep -q -F -e '--deny=warnings' "$doc" &&
  grep -q -F -e '-Werror' "$doc" &&
  grep -q -F -e '-Xlint:all' "$doc" &&
  grep -q -F -e 'warn = "error"' "$doc" &&
  grep -q -F -e '-Xfatal-warnings' "$doc" &&
  grep -q -F -e 'treat_warnings_as_errors' "$doc" &&
  grep -q -F -e 'go vet' "$doc" &&
  grep -q -F -e 'PYTHONWARNINGS=error' "$doc" &&
  grep -q -F -e 'strict' "$doc" &&
  grep -q -F -e '//tools/ci:warnings_as_errors' "$doc" &&
  grep -q -F -e 'tools/ci/testdata/warnings/' "$doc" &&
  grep -q -F -e 'issue #455' "$doc"; then
  ok
else
  bad "docs/quality/warnings-as-errors.md lost its pin table or gate/fixture record (want all toolchains plus gate plus fixtures, issue #455)"
fi

# Doc stays linked from the quality index and corpus.
if grep -q -F -e 'warnings-as-errors.md' docs/quality/README.md &&
  grep -q -F -e 'quality/warnings-as-errors.md' docs/BUILD.bazel; then
  ok
else
  bad "warnings-as-errors doc lost its index or corpus link (want README plus docs/BUILD.bazel, issue #455)"
fi

# Violation fixtures: one warning pattern per language as uncompiled data.
fixtures="tools/ci/testdata/warnings"
missing=""
for f in rust_unused.rs python_unused.py js_unused.js ts_implicit_any.ts go_printf.go java_unused.java kotlin_unused.kt scala_unused.scala csharp_unreachable.cs fsharp_unused.fs cc_unused.cc; do
  if [[ ! -f "$fixtures/$f" ]]; then
    missing="$missing $f:missing"
  fi
done
if [[ -z "$missing" ]] &&
  grep -q -F -e 'let unused' "$fixtures/rust_unused.rs" &&
  grep -q -F -e 'import os' "$fixtures/python_unused.py" &&
  grep -q -F -e 'const unused' "$fixtures/js_unused.js" &&
  grep -q -F -e 'warningFn' "$fixtures/ts_implicit_any.ts" &&
  grep -q -F -e 'Printf' "$fixtures/go_printf.go" &&
  grep -q -F -e 'import java.util.List' "$fixtures/java_unused.java" &&
  grep -q -F -e 'val unused' "$fixtures/kotlin_unused.kt" &&
  grep -q -F -e 'val unused' "$fixtures/scala_unused.scala" &&
  grep -q -F -e 'int unused' "$fixtures/csharp_unreachable.cs" &&
  grep -q -F -e 'let unused' "$fixtures/fsharp_unused.fs" &&
  grep -q -F -e 'int unused' "$fixtures/cc_unused.cc"; then
  ok
else
  bad "warning fixtures incomplete or lost patterns:$missing (want 11 fixtures with warning patterns, issue #455)"
fi

dx_test_summary "warnings-as-errors harness"
