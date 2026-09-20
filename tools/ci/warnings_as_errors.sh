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
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib", "//tools/sh:guards"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/guards.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/guards.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/guards.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/guards.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/guards.sh"

dx_cd_workspace

dx_test_init

bazelrc=".bazelrc"
doc="docs/quality/warnings-as-errors.md"

# Root pins: Rust plus Java plus Python runtime (C/C++ stays wrapper-only
# so third-party external compiles stay green).
dx_guards_contains "$bazelrc" "root .bazelrc lost warnings-as-errors pins (want rust --deny=warnings plus java -Werror/-Xlint:all plus PYTHONWARNINGS=error, issue #455)" \
  '--@rules_rust//rust/settings:extra_rustc_flags=--deny=warnings' \
  '--javacopt=-Werror' \
  '--javacopt=-Xlint:all' \
  '--test_env=PYTHONWARNINGS=error' \
  'issue #455'

# C/C++ wrapper defaults to -Werror with no global --copt pin.
dx_guards_contains cc/rules/defs.bzl "cc pin drifted (want wrapper _cc_with_werror -Werror, issue #455)" \
  '-Werror' \
  '_cc_with_werror'
dx_guard_absent "$bazelrc" '--copt=-Werror' "cc pin drifted (want no global --copt=-Werror, issue #455)"

# Java wrapper defaults to -Werror plus -Xlint:all.
dx_guards_contains java/rules/defs.bzl "java/rules/defs.bzl lost its -Werror/-Xlint:all default (want _java_with_werror, issue #455)" \
  '-Werror' \
  '-Xlint:all' \
  '_java_with_werror'

# Kotlin wrapper defaults to the warnings_as_errors kotlinc options.
dx_guard_contains kotlin/rules/defs.bzl 'warnings_as_errors' "kotlin/rules lost its warn=error default (want kotlinc_opts warnings_as_errors, issue #455)"
dx_guards_contains kotlin/rules/BUILD.bazel "kotlin/rules lost its warn=error default (want kotlinc_opts with 2.2, issue #455)" \
  'kt_kotlinc_options' \
  'warn = "error"' \
  'language_version = "2.2"' \
  'api_version = "2.2"'

# Scala wrapper defaults to -Xfatal-warnings.
dx_guards_contains scala/rules/defs.bzl "scala/rules/defs.bzl lost its -Xfatal-warnings default (want _scala_with_werror, issue #455)" \
  '-Xfatal-warnings' \
  '_scala_with_werror'

# .NET wrappers treat warnings as errors.
dx_guard_contains csharp/rules/defs.bzl 'treat_warnings_as_errors' "csharp wrapper lost treat_warnings_as_errors (want True default, issue #455)"
dx_guard_contains fsharp/rules/defs.bzl 'treat_warnings_as_errors' "fsharp wrapper lost treat_warnings_as_errors (want True default, issue #455)"

# TypeScript fixtures pin strict; JS/TS lint stays --fail_on warning.
dx_guard_contains typescript/tests/fixtures/hello/tsconfig.json '"strict"' "typescript hello fixture lost strict (want strict true, issue #455)"
dx_guard_contains typescript/tests/fixtures/entries/tsconfig.json '"strict"' "typescript entries fixture lost strict (want strict true, issue #455)"

# Doc owns the per-toolchain pin table plus gate plus fixtures.
dx_guards_contains "$doc" "docs/quality/warnings-as-errors.md lost its pin table or gate/fixture record (want all toolchains plus gate plus fixtures, issue #455)" \
  'Per-toolchain' \
  '--deny=warnings' \
  '-Werror' \
  '-Xlint:all' \
  'warn = "error"' \
  '-Xfatal-warnings' \
  'treat_warnings_as_errors' \
  'go vet' \
  'PYTHONWARNINGS=error' \
  'strict' \
  '//tools/ci:warnings_as_errors' \
  'tools/ci/testdata/warnings/' \
  'issue #455'

# Doc stays linked from the quality index and corpus.
dx_guard_contains docs/quality/README.md 'warnings-as-errors.md' "warnings-as-errors doc lost its index link (want README, issue #455)"
dx_guard_contains docs/BUILD.bazel 'quality/warnings-as-errors.md' "warnings-as-errors doc lost its corpus link (want docs/BUILD.bazel, issue #455)"

# Violation fixtures: one warning pattern per language as uncompiled data.
fixtures="tools/ci/testdata/warnings"
dx_guard_contains "$fixtures/rust_unused.rs" 'let unused' "warning fixture lost pattern (want rust let unused, issue #455)"
dx_guard_contains "$fixtures/python_unused.py" 'import os' "warning fixture lost pattern (want python import os, issue #455)"
dx_guard_contains "$fixtures/js_unused.js" 'const unused' "warning fixture lost pattern (want js const unused, issue #455)"
dx_guard_contains "$fixtures/ts_implicit_any.ts" 'warningFn' "warning fixture lost pattern (want ts warningFn, issue #455)"
dx_guard_contains "$fixtures/go_printf.go" 'Printf' "warning fixture lost pattern (want go Printf, issue #455)"
dx_guard_contains "$fixtures/java_unused.java" 'import java.util.List' "warning fixture lost pattern (want java import, issue #455)"
dx_guard_contains "$fixtures/kotlin_unused.kt" 'val unused' "warning fixture lost pattern (want kotlin val unused, issue #455)"
dx_guard_contains "$fixtures/scala_unused.scala" 'val unused' "warning fixture lost pattern (want scala val unused, issue #455)"
dx_guard_contains "$fixtures/csharp_unreachable.cs" 'int unused' "warning fixture lost pattern (want csharp int unused, issue #455)"
dx_guard_contains "$fixtures/fsharp_unused.fs" 'let unused' "warning fixture lost pattern (want fsharp let unused, issue #455)"
dx_guard_contains "$fixtures/cc_unused.cc" 'int unused' "warning fixture lost pattern (want cc int unused, issue #455)"

dx_test_summary "warnings-as-errors harness"
