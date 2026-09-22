#!/usr/bin/env bash
# Quality-adapter parity/provenance guards (qualified).
#
# Required-core adapters plus Buildifier/Taplo/Vale probes are qualified
# with fixture evidence; the seven missing families keep frozen routes owned
# by ADR 0019 with no false adapter claim (Scala/.NET delivered under #797).
# Buildifier, Taplo,
# and Vale probes stay provisional: research notes are unproven mappings,
# versions are observations not pins.
#
# This harness machine-checks the functional half verifiable on a clean
# tree today (table-driven guard rows via `tools/sh/guards.sh` plus two
# custom OR loops, no docs-prose guards): no false adapter claim for the
# seven missing families, parity deferrals with owner plus route, every
# adapter-backed class with real subjects plus matrix pass/fail plus
# fix/format evidence, every adapter-backed tool with parser plus matrix
# evidence plus native-config or explicit config-free/delegated status,
# standalone artifact metadata plus regeneration policy, and packaging
# single-correct-path plus manifest completeness plus candidate wire
# profile URIs plus verification binding. Platform gaps beyond the seed
# Linux x86_64 host stay owned with clean refusal, never
# a silent pass; no Supported claim without platform plus consumer plus
# release evidence (gate).
#
# Versioned here, run by CI via `bazel run //tools/ci:quality_adapters_parity`,
# following //tools/ci:release_policy.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"
dx_bootstrap "tools/sh/guards.sh"

dx_cd_workspace

dx_test_init

adapters="quality/adapters.bzl"
parity="quality/parity_tests.bzl"
matrix="quality/testdata/runner_matrix_cases.bzl"
subjects="quality/testdata/BUILD.bazel"
native="quality/native_config.bzl"
aspects="quality/real_aspects.bzl"
metadata="quality/artifacts/metadata_tests.bzl"
update="quality/artifacts/update.py"
qual="cli/qualification/src/lib.rs"
integrations="docs/quality/tool-integrations.md"

# No false adapter claim: all cohorts delivered, no missing tool IDs remain
# among those cohorts. Classification exists in
# REAL_CLASS_TO_FAMILY; adapter claim does not. Scala/.NET (scalafmt,
# scalafix, csharpier, fantomas, roslyn, fsharplint) delivered under #797
# plus JVM (google-java-format, checkstyle, pmd, spotbugs, ktfmt, ktlint)
# delivered under #796 plus native (clang_format, clang_tidy, cppcheck,
# gofumpt, staticcheck, govet, errcheck) delivered under #798 plus
# Structured (buf, qmlformat, qmllint) delivered under #799 plus
# interpreted/file-family (cue, djlint, stylelint, rubocop, standardrb,
# psscriptanalyzer and eleven more) delivered under #800.
# All cohort tool IDs are now claimed; no absent check remains for those
# cohorts (remaining deferred framework regions have no tool IDs).

# Parity deferrals name owner plus route for the remaining deferred set
# (mirrors parity_tests.bzl unit gate so CI fails here too).
# Scala/.NET classes delivered under #797 plus JVM classes under #796 plus
# native classes under #798 (including CUDA via clang-format) plus
# Structured classes delivered under #799 plus file-family classes
# delivered under #800.
dx_guards_contains "$parity" "parity deferrals drifted (want class plus ADR 0019 owner plus shape)" \
  '"astro":' \
  '"astro": ["ADR 0019"' \
  'PARITY_DEFERRED = {'
dx_guards_contains "$adapters" "parity deferrals drifted (want adapter plus taxonomy shape)" \
  'REAL_ADAPTERS = {' \
  'REAL_CLASS_TO_FAMILY = {'

# Every adapter-backed class has real subjects plus matrix evidence.
# The thirty-nine backed classes: c, cpp, cuda, csharp, css, cue, fsharp,
# gherkin, go, go_module, java, javascript, json, jsonnet, jsx, kotlin,
# less, markdown, pkl, powershell, protobuf, python, python_stub, qml,
# ruby, rust, scala, scss, shell, sql, starlark, terraform, text, toml,
# tsx, typescript, xml, yaml, html_template. python_stub rides generated
# .pyi matrix cases; scala/csharp/fsharp ride generated plus delegated
# matrix cases (issue #797); java/kotlin ride real_source_target subjects
# plus JVM matrix cells (issue #796); c/cpp/cuda/go ride generated plus
# delegated matrix cases (issue #798, cuda format via clang-format fake
# double); protobuf/qml ride generated plus delegated matrix cells (issue
# #799); file-family rides generated plus delegated matrix cases (issue
# #800); the rest ride real_source_target subjects.
dx_guards_contains "$subjects" "adapter-backed class evidence drifted (want subjects)" \
  'javascript' \
  'json' \
  'jsx' \
  'markdown' \
  'python' \
  'rust' \
  'starlark' \
  'toml' \
  'tsx' \
  'typescript'
dx_guards_contains "$matrix" "adapter-backed class evidence drifted (want matrix aggregator)" \
  'SCALA_DOTNET_CASES' \
  'JVM_CASES' \
  'NATIVE_CASES' \
  'STRUCTURED_CASES' \
  'FILE_FAMILY_CASES'
dx_guards_tree_contains "runner_matrix_*.bzl" "adapter-backed class evidence drifted (want matrix cases)" \
  'python_stub' \
  'matrix_rust_format_pass' \
  'matrix_python_lint_pass' \
  'matrix_javascript_lint_pass' \
  'matrix_json_lint_pass' \
  'matrix_markdown_lint_pass' \
  'matrix_starlark_lint_pass' \
  'matrix_toml_lint_pass'

# Every adapter-backed tool has parser plus matrix pass/fail evidence.
# Fifty-three tools: biome, buf, buildifier, checkstyle, clang_format, clang_tidy,
# clippy, cppcheck, csharpier, cue, djlint, errcheck, eslint, fantomas,
# flake8, fsharplint, gofumpt, google_java_format, govet, jsonnetfmt,
# keep_sorted, ktfmt, ktlint, markdown_check, modfmt, pkl, pmd,
# prettier, psscriptanalyzer, pydoclint, pylint, qmlformat, qmllint, roslyn,
# rubocop, ruff, rustc, rustfmt, scalafix, scalafmt, shellcheck, shfmt, spotbugs,
# standardrb, staticcheck, stylelint, taplo, terraform, tsc, ty, vale,
# yamlfmt, yamllint.
# tsc is target-coupled (needs TsConfigInfo) so its evidence lives in
# quality/tools/typescript/BUILD.bazel plus the tsc parser, not the
# provider-less runner matrix. Roslyn/scalafix/fsharplint run delegated
# (recorded upstream diagnostics like clippy/rustc, issue #797);
# clang-tidy/cppcheck/staticcheck/govet/errcheck run delegated the same way
# (issue #798). Buf lint plus qmllint run delegated in the matrix (recorded upstream
# diagnostics like clippy/rustc, issue #799) but spawn in production.
# SpotBugs is target-coupled (needs JavaInfo) so it has a
# parser but no matrix cell, like tsc (issue #796).
dx_guard_file quality/adapter/src/parsers/biome.rs "adapter-backed tool evidence drifted (want biome parser)"
dx_guard_file quality/adapter/src/parsers/buf.rs "adapter-backed tool evidence drifted (want buf parser)"
dx_guard_file quality/adapter/src/parsers/buildifier.rs "adapter-backed tool evidence drifted (want buildifier parser)"
dx_guard_file quality/adapter/src/parsers/checkstyle.rs "adapter-backed tool evidence drifted (want checkstyle parser)"
dx_guard_file quality/adapter/src/parsers/clang_format.rs "adapter-backed tool evidence drifted (want clang_format parser)"
dx_guard_file quality/adapter/src/parsers/clang_tidy.rs "adapter-backed tool evidence drifted (want clang_tidy parser)"
dx_guard_file quality/adapter/src/parsers/cppcheck.rs "adapter-backed tool evidence drifted (want cppcheck parser)"
dx_guard_file quality/adapter/src/parsers/csharpier.rs "adapter-backed tool evidence drifted (want csharpier parser)"
dx_guard_file quality/adapter/src/parsers/errcheck.rs "adapter-backed tool evidence drifted (want errcheck parser)"
dx_guard_file quality/adapter/src/parsers/eslint.rs "adapter-backed tool evidence drifted (want eslint parser)"
dx_guard_file quality/adapter/src/parsers/fantomas.rs "adapter-backed tool evidence drifted (want fantomas parser)"
dx_guard_file quality/adapter/src/parsers/flake8.rs "adapter-backed tool evidence drifted (want flake8 parser)"
dx_guard_file quality/adapter/src/parsers/fsharplint.rs "adapter-backed tool evidence drifted (want fsharplint parser)"
dx_guard_file quality/adapter/src/parsers/gofumpt.rs "adapter-backed tool evidence drifted (want gofumpt parser)"
dx_guard_file quality/adapter/src/parsers/google_java_format.rs "adapter-backed tool evidence drifted (want google-java-format parser)"
dx_guard_file quality/adapter/src/parsers/govet.rs "adapter-backed tool evidence drifted (want govet parser)"
dx_guard_file quality/adapter/src/parsers/ktfmt.rs "adapter-backed tool evidence drifted (want ktfmt parser)"
dx_guard_file quality/adapter/src/parsers/ktlint.rs "adapter-backed tool evidence drifted (want ktlint parser)"
dx_guard_file quality/adapter/src/parsers/markdown.rs "adapter-backed tool evidence drifted (want markdown parser)"
dx_guard_file quality/adapter/src/parsers/pmd.rs "adapter-backed tool evidence drifted (want pmd parser)"
dx_guard_file quality/adapter/src/parsers/prettier.rs "adapter-backed tool evidence drifted (want prettier parser)"
dx_guard_file quality/adapter/src/parsers/pydoclint.rs "adapter-backed tool evidence drifted (want pydoclint parser)"
dx_guard_file quality/adapter/src/parsers/pylint.rs "adapter-backed tool evidence drifted (want pylint parser)"
dx_guard_file quality/adapter/src/parsers/qmlformat.rs "adapter-backed tool evidence drifted (want qmlformat parser)"
dx_guard_file quality/adapter/src/parsers/qmllint.rs "adapter-backed tool evidence drifted (want qmllint parser)"
dx_guard_file quality/adapter/src/parsers/roslyn.rs "adapter-backed tool evidence drifted (want roslyn parser)"
dx_guard_file quality/adapter/src/parsers/ruff.rs "adapter-backed tool evidence drifted (want ruff parser)"
dx_guard_file quality/adapter/src/parsers/rustfmt.rs "adapter-backed tool evidence drifted (want rustfmt parser)"
dx_guard_file quality/adapter/src/parsers/scalafix.rs "adapter-backed tool evidence drifted (want scalafix parser)"
dx_guard_file quality/adapter/src/parsers/scalafmt.rs "adapter-backed tool evidence drifted (want scalafmt parser)"
dx_guard_file quality/adapter/src/parsers/spotbugs.rs "adapter-backed tool evidence drifted (want spotbugs parser)"
dx_guard_file quality/adapter/src/parsers/staticcheck.rs "adapter-backed tool evidence drifted (want staticcheck parser)"
dx_guard_file quality/adapter/src/parsers/taplo.rs "adapter-backed tool evidence drifted (want taplo parser)"
dx_guard_file quality/adapter/src/parsers/tsc.rs "adapter-backed tool evidence drifted (want tsc parser)"
dx_guard_file quality/adapter/src/parsers/ty.rs "adapter-backed tool evidence drifted (want ty parser)"
dx_guard_file quality/adapter/src/parsers/vale.rs "adapter-backed tool evidence drifted (want vale parser)"
dx_guard_file quality/adapter/src/parsers/cue.rs "adapter-backed tool evidence drifted (want cue parser)"
dx_guard_file quality/adapter/src/parsers/djlint.rs "adapter-backed tool evidence drifted (want djlint parser)"
dx_guard_file quality/adapter/src/parsers/stylelint.rs "adapter-backed tool evidence drifted (want stylelint parser)"
dx_guard_file quality/adapter/src/parsers/rubocop.rs "adapter-backed tool evidence drifted (want rubocop parser)"
dx_guard_file quality/adapter/src/parsers/psscriptanalyzer.rs "adapter-backed tool evidence drifted (want psscriptanalyzer parser)"
dx_guard_file quality/adapter/src/parsers/yamllint.rs "adapter-backed tool evidence drifted (want yamllint parser)"
dx_guard_file quality/adapter/src/parsers/shellcheck.rs "adapter-backed tool evidence drifted (want shellcheck parser)"
dx_guard_file quality/adapter/src/parsers/keep_sorted.rs "adapter-backed tool evidence drifted (want keep_sorted parser)"
dx_guard_file quality/adapter/src/parsers/jsonnetfmt.rs "adapter-backed tool evidence drifted (want jsonnetfmt parser)"
dx_guard_file quality/adapter/src/parsers/modfmt.rs "adapter-backed tool evidence drifted (want modfmt parser)"
dx_guard_file quality/adapter/src/parsers/pkl.rs "adapter-backed tool evidence drifted (want pkl parser)"
dx_guard_file quality/adapter/src/parsers/shfmt.rs "adapter-backed tool evidence drifted (want shfmt parser)"
dx_guard_file quality/adapter/src/parsers/standardrb.rs "adapter-backed tool evidence drifted (want standardrb parser)"
dx_guard_file quality/adapter/src/parsers/terraform.rs "adapter-backed tool evidence drifted (want terraform parser)"
dx_guard_file quality/adapter/src/parsers/yamlfmt.rs "adapter-backed tool evidence drifted (want yamlfmt parser)"
# clippy/rustc parse through rust.rs delegated diagnostics, not separate files.
dx_guards_contains quality/adapter/src/parsers/rust.rs "adapter-backed tool evidence drifted (want clippy/rustc delegated parse)" \
  'parse_clippy' \
  'parse_rustc'
dx_guards_tree_contains "runner_matrix_*.bzl" "adapter-backed tool evidence drifted (want matrix entries)" \
  'biome' \
  'buildifier' \
  'clippy' \
  'eslint' \
  'flake8' \
  'markdown_check' \
  'prettier' \
  'pydoclint' \
  'pylint' \
  'ruff' \
  'rustc' \
  'rustfmt' \
  'taplo' \
  'ty' \
  'vale'
# Scala/.NET cohort matrix evidence lives in the split file (issue #797).
dx_guards_contains quality/testdata/runner_matrix_scala_dotnet.bzl "adapter-backed tool evidence drifted (want scala-dotnet matrix entries)" \
  'csharpier' \
  'fantomas' \
  'fsharplint' \
  'roslyn' \
  'scalafix' \
  'scalafmt'
# JVM cohort matrix evidence lives in the split file (issue #796; spotbugs
# target-coupled with no matrix cell, like tsc).
dx_guards_contains quality/testdata/runner_matrix_jvm.bzl "adapter-backed tool evidence drifted (want jvm matrix entries)" \
  'google_java_format' \
  'checkstyle' \
  'pmd' \
  'ktfmt' \
  'ktlint'
# Native cohort matrix evidence lives in the split file (issue #798).
dx_guards_contains quality/testdata/runner_matrix_native.bzl "adapter-backed tool evidence drifted (want native matrix entries)" \
  'clang_format' \
  'clang_tidy' \
  'cppcheck' \
  'gofumpt' \
  'staticcheck' \
  'govet' \
  'errcheck'
# Structured cohort matrix evidence lives in the split file (issue #799;
# lint via delegated recorded diagnostics like clippy/rustc, format via
# fake doubles seed-only).
dx_guards_contains quality/testdata/runner_matrix_structured.bzl "adapter-backed tool evidence drifted (want structured matrix entries)" \
  'buf' \
  'qmlformat' \
  'qmllint'
# Interpreted/file-family cohort matrix evidence lives in the split file (issue #800).
dx_guards_contains quality/testdata/runner_matrix_file_family.bzl "adapter-backed tool evidence drifted (want file-family matrix entries)" \
  'cue' \
  'djlint' \
  'stylelint' \
  'rubocop' \
  'psscriptanalyzer' \
  'yamllint' \
  'shellcheck' \
  'keep_sorted'
# tsc target-coupled evidence: authoritative mapping plus parser plus
# aspect coupling note, never a bare-file matrix case. The coupling-note
# match is case-neutral (`coupled tsc`) so sentence-case comments
# (`Target-coupled tsc` in real_aspects.bzl) satisfy it.
dx_guard_contains quality/tools/typescript/BUILD.bazel 'tsc' "adapter-backed tool evidence drifted (want tsc mapping)"
dx_guard_contains "$aspects" 'coupled tsc' "adapter-backed tool evidence drifted (want tsc coupled note)"
# Pass plus fail cells exist for the matrix (dirty plus clean).
dx_guard_tree_contains "runner_matrix_*.bzl" '_fail' "adapter-backed tool evidence drifted (want matrix fail cells)"
dx_guard_tree_contains "runner_matrix_*.bzl" '_pass' "adapter-backed tool evidence drifted (want matrix pass cells)"
# Fix/format replacements exist where applicable (formatters emit replacements).
dx_guard_tree_contains "runner_matrix_*.bzl" 'replacement ' "adapter-backed tool evidence drifted (want matrix replacements)"

# Native-config bindings exist for the twenty-two tools that take native policy;
# the rest are explicitly config-free, upstream-delegated,
# target-coupled, or check-only by design (no native-config rule expected).
# Scala/.NET adds scalafmt, scalafix, csharpier, fsharplint (issue #797);
# JVM adds checkstyle (issue #796); native adds clang_format, clang_tidy,
# cppcheck, staticcheck (issue #798); Structured adds buf, qmlformat, qmllint
# (issue #799); file-family adds stylelint, djlint, yamllint (issue #800);
# roslyn is SDK-coupled and fantomas
# takes .editorconfig (no typed rule); gofumpt, govet, and errcheck take no
# native config file by design.
config_fail=""
for tool in biome buf buildifier checkstyle clang_format clang_tidy cppcheck csharpier djlint eslint fsharplint qmlformat qmllint ruff rustfmt scalafix scalafmt staticcheck stylelint taplo vale yamllint; do
  grep -q -F -e "\"$tool\":" "$native" || grep -q -F -e "_NATIVE_CONFIG_EXTENSIONS" "$native" || config_fail="$config_fail $tool:extension"
done
if [[ -z "$config_fail" ]]; then
  ok
else
  bad "native-config bindings drifted:$config_fail"
fi
dx_guards_contains "$native" "native-config bindings drifted (want per-tool rules)" \
  'biome_config' \
  'buf_config' \
  'buildifier_config' \
  'checkstyle_config' \
  'clang_format_config' \
  'clang_tidy_config' \
  'cppcheck_config' \
  'csharpier_config' \
  'djlint_config' \
  'eslint_config' \
  'fsharplint_config' \
  'qmlformat_config' \
  'qmllint_config' \
  'ruff_config' \
  'rustfmt_config' \
  'scalafix_config' \
  'scalafmt_config' \
  'staticcheck_config' \
  'stylelint_config' \
  'taplo_config' \
  'vale_config' \
  'yamllint_config'
# Config-free/delegated status is documented in code, not silently missing.
dx_guard_contains "$adapters" 'check-only' "native-config bindings drifted (want adapters check-only note)"
dx_guard_re_contains "$aspects" 'upstream-delegated|Delegated' "native-config bindings drifted (want aspects delegated note)"
dx_guard_contains "$aspects" 'target-coupled' "native-config bindings drifted (want aspects coupled note)"
dx_guard_contains "$aspects" 'no usable upstream default' "native-config bindings drifted (want aspects vale note)"

# Standalone artifact metadata plus regeneration policy: six tools pinned
# with immutable URL, digest, size, executable, licenses, plus the
# maintainer regeneration entry point with --verify-only.
dx_guard_file quality/artifacts/biome.linux_x86_64.bzl "artifact metadata/regeneration drifted (want biome file)"
dx_guard_file quality/artifacts/buildifier.linux_x86_64.bzl "artifact metadata/regeneration drifted (want buildifier file)"
dx_guard_file quality/artifacts/ruff.linux_x86_64.bzl "artifact metadata/regeneration drifted (want ruff file)"
dx_guard_file quality/artifacts/taplo.linux_x86_64.bzl "artifact metadata/regeneration drifted (want taplo file)"
dx_guard_file quality/artifacts/ty.linux_x86_64.bzl "artifact metadata/regeneration drifted (want ty file)"
dx_guard_file quality/artifacts/vale.linux_x86_64.bzl "artifact metadata/regeneration drifted (want vale file)"
dx_guards_contains "$metadata" "artifact metadata/regeneration drifted (want pins)" \
  'biome' \
  'buildifier' \
  'ruff' \
  'taplo' \
  'ty' \
  'vale'
artifact_update_fail=""
for tool in biome buildifier ruff taplo ty vale; do
  grep -q -F -e "\"$tool\":" "$update" || grep -q -F -e "'$tool':" "$update" || grep -q -F -e "\"$tool\"" "$update" || artifact_update_fail="$artifact_update_fail $tool:update"
done
if [[ -z "$artifact_update_fail" ]]; then
  ok
else
  bad "artifact metadata/regeneration drifted:$artifact_update_fail"
fi
dx_guard_contains quality/artifacts/biome.linux_x86_64.bzl 'Regenerate with: bazel run //quality/artifacts:update' "artifact metadata/regeneration drifted (want regen biome)"
dx_guard_contains "$update" '--verify-only' "artifact metadata/regeneration drifted (want regen verify flag)"
dx_guard_contains quality/artifacts/BUILD.bazel 'metadata_tests' "artifact metadata/regeneration drifted (want build metadata)"
dx_guard_contains "$metadata" 'FROZEN_SCHEMA_KEYS' "artifact metadata/regeneration drifted (want schema frozen)"

# Packaging single-correct-path plus manifest completeness plus candidate
# wire profile URIs plus verification binding (qualification planning,
# the issue tracker own execution; this crate never signs or publishes).
dx_guards_contains "$qual" "packaging/provenance qualification drifted (want fns plus URIs plus binding)" \
  'packaging_uses_single_correct_path' \
  'manifest_covers_payload' \
  'CANDIDATE_SPDX_PREDICATE_URI' \
  'CANDIDATE_SLSA_PREDICATE_URI' \
  'CANDIDATE_STATEMENT_TYPE_URI' \
  'attestation_binds_exact_bytes' \
  'verify_rejection' \
  'self_attested_level_admissible'

# Buildifier/Taplo/Vale probes stay provisional: research notes are
# unproven mappings, versions are observations not pins. Biome/ESLint/
# Prettier/tsc carry direct-probe evidence with pinned versions.
dx_guards_contains "$integrations" "provisional probe record drifted (want unproven plus observations plus direct probes)" \
  'unproven mappings' \
  'observations, not pins' \
  'Direct probes show'

dx_test_summary "quality adapters parity harness"
