#!/usr/bin/env bash
# Warnings-as-errors enforcement gate.
#
# Pins warnings-as-errors per toolchain and proves violation fixtures
# exist. Rust uses --deny=warnings; C/C++ uses -Werror; Java uses
# -Werror plus -Xlint:all; Kotlin uses warn=error; Scala uses
# -Xfatal-warnings; C#/F# use treat_warnings_as_errors; Go uses go vet;
# Python uses PYTHONWARNINGS=error plus Ruff; JS/TS use Biome plus tsc
# strict. Adapter families use check-mode plus --fail_on warning
# (issue #1056): Ruby uses rubocop --format json plus standardrb --check;
# PowerShell uses psscriptanalyzer text; Shell uses shellcheck
# --format=gcc plus shfmt -d; CUE uses cue fmt --check --diff; QML uses
# qmlformat --check plus qmllint --json -; Protobuf uses buf lint
# --error-format=json plus buf format --diff --exit-code; keep_sorted is
# check-only; Error Prone uses -Xplugin:ErrorProne; detekt stays pending
# with no strict pin. Fixtures in tools/ci/testdata/warnings/ carry one
# warning pattern per language as uncompiled data; the gate proves each
# fixture and each pin is present.
#
# Versioned here, run by CI via `bazel run //tools/ci:warnings_as_errors`,
# following //tools/ci:build_hygiene.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"
dx_bootstrap "tools/sh/guards.sh"

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

# Adapter families use check-mode plus evaluator --fail_on warning
# (issue #1056): lint warnings fail at warning threshold, format diffs
# fail independently of severity.
dx_guards_contains config/BUILD.bazel "evaluator threshold drifted (want fail_on warning default, issue #1056)" \
  'fail_on' \
  'build_setting_default = "warning"'
dx_guard_contains quality/aspects.bzl '--fail_on' "quality aspects lost evaluator threshold (want --fail_on, issue #1056)"

# Adapter dispatch owns the missing families (detekt stays pending with
# no adapter, so it is absent here and recorded pending in the doc).
dx_guards_contains quality/adapters.bzl "adapter dispatch drifted (want ruby/powershell/shell/cue/qml/protobuf/text families, issue #1056)" \
  'rubocop' \
  'standardrb' \
  'psscriptanalyzer' \
  'shellcheck' \
  'shfmt' \
  'cue' \
  'buf' \
  'qmlformat' \
  'qmllint' \
  'keep_sorted'

# Adapter check invocations stay strict check-mode (issue #1056).
dx_guards_contains quality/adapter/src/commands.rs "adapter check invocations drifted (want ruby/powershell/shell/cue/qml/buf/keep_sorted/error_prone checks, issue #1056)" \
  'rubocop_check' \
  'standardrb_check' \
  'psscriptanalyzer_check' \
  'shellcheck_check' \
  'shfmt_check' \
  'cue_check' \
  'buf_lint_check' \
  'buf_format_check' \
  'qmlformat_check' \
  'qmllint_check' \
  'keep_sorted_check' \
  'error_prone_check'
dx_guards_contains quality/adapter/src/commands.rs "adapter strict flags drifted (want check-mode pins, issue #1056)" \
  '"--format", "json"' \
  '"--format=gcc"' \
  '"--error-format=json"' \
  '"--exit-code"' \
  '"--json", "-"' \
  '"fmt", "--check", "--diff"' \
  '-Xplugin:ErrorProne'

# Doc owns the adapter-family pin table plus detekt pending record.
dx_guards_contains "$doc" "docs/quality/warnings-as-errors.md lost its adapter-family pins (want rubocop plus standardrb plus psscriptanalyzer plus shellcheck plus shfmt plus cue plus buf plus qml plus keep_sorted plus error_prone plus detekt pending, issue #1056)" \
  'rubocop' \
  'standardrb' \
  'psscriptanalyzer' \
  'shellcheck' \
  'shfmt' \
  'cue' \
  'buf' \
  'qmlformat' \
  'qmllint' \
  'keep_sorted' \
  'error_prone' \
  'detekt' \
  '--fail_on warning' \
  'issue #1056'

# Adapter violation fixtures: one warning pattern per family as
# uncompiled data (detekt stays pending with no fixture).
dx_guard_contains "$fixtures/ruby_offense.rb" "puts 'hello'" "warning fixture lost pattern (want ruby puts single quotes, issue #1056)"
dx_guard_contains "$fixtures/shell_unquoted.sh" 'echo $unquoted' "warning fixture lost pattern (want shell unquoted var, issue #1056)"
dx_guard_contains "$fixtures/powershell_writehost.ps1" 'Write-Host' "warning fixture lost pattern (want powershell Write-Host, issue #1056)"
dx_guard_contains "$fixtures/cue_badfmt.cue" 'value:"hello"' "warning fixture lost pattern (want cue missing space, issue #1056)"
dx_guard_contains "$fixtures/qml_unqualified.qml" 'greet: foo' "warning fixture lost pattern (want qml unqualified, issue #1056)"
dx_guard_contains "$fixtures/proto_mismatch.proto" 'package foo' "warning fixture lost pattern (want proto package foo, issue #1056)"
dx_guard_contains "$fixtures/keep_sorted_unsorted.txt" 'zebra' "warning fixture lost pattern (want keep_sorted zebra, issue #1056)"
dx_guard_contains "$fixtures/error_prone_deadexception.java" 'new IllegalArgumentException' "warning fixture lost pattern (want error_prone DeadException, issue #1056)"

dx_test_summary "warnings-as-errors harness"
