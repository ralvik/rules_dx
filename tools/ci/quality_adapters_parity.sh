#!/usr/bin/env bash
# Quality-adapter parity/provenance guards (qualified).
#
# Required-core adapters plus Buildifier/Taplo/Vale probes are qualified
# with fixture evidence; the ten missing families keep frozen routes owned
# by ADR 0019 with no false adapter claim. Buildifier, Taplo,
# and Vale probes stay provisional: research notes are unproven mappings,
# versions are observations not pins.
#
# This harness machine-checks the functional half verifiable on a clean
# tree today (no docs-prose guards): no false adapter claim for the ten
# missing families, parity deferrals with owner plus route, every
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
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

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

# No false adapter claim for the ten missing families: none of their
# tool IDs appear in REAL_ADAPTERS. Classification exists in
# REAL_CLASS_TO_FAMILY; adapter claim does not.
missing_claim=""
for tool in buf qmlformat qmllint google-java-format checkstyle pmd spotbugs ktfmt ktlint scalafmt scalafix csharpier fantomas psscriptanalyzer rubocop standardrb clang-format clang-tidy cppcheck; do
  if grep -q -F -e "\"$tool\":" "$adapters"; then
    missing_claim="$missing_claim $tool:claimed"
  fi
done
# Classification for the ten exists in REAL_CLASS_TO_FAMILY plus
# PARITY_DEFERRED; adapter claim does not (enforced via the parity unit
# gate shape below).
if [[ -z "$missing_claim" ]]; then
  ok
else
  bad "false adapter claim for missing families:$missing_claim"
fi

# Parity deferrals name owner plus route for the ten plus the wider
# deferred set (mirrors parity_tests.bzl unit gate so CI fails here too).
deferred_fail=""
for cls in protobuf qml java kotlin scala csharp fsharp powershell ruby c cpp; do
  grep -q -F -e "\"$cls\":" "$parity" || deferred_fail="$deferred_fail $cls:missing"
done
grep -q -F -e '"protobuf": ["ADR 0019"' "$parity" || deferred_fail="$deferred_fail protobuf:owner"
grep -q -F -e '"qml": ["ADR 0019"' "$parity" || deferred_fail="$deferred_fail qml:owner"
grep -q -F -e '"java": ["ADR 0019"' "$parity" || deferred_fail="$deferred_fail java:owner"
grep -q -F -e '"kotlin": ["ADR 0019"' "$parity" || deferred_fail="$deferred_fail kotlin:owner"
grep -q -F -e '"scala": ["ADR 0019"' "$parity" || deferred_fail="$deferred_fail scala:owner"
grep -q -F -e '"csharp": ["ADR 0019"' "$parity" || deferred_fail="$deferred_fail csharp:owner"
grep -q -F -e '"fsharp": ["ADR 0019"' "$parity" || deferred_fail="$deferred_fail fsharp:owner"
grep -q -F -e '"powershell": ["ADR 0019"' "$parity" || deferred_fail="$deferred_fail powershell:owner"
grep -q -F -e '"ruby": ["ADR 0019"' "$parity" || deferred_fail="$deferred_fail ruby:owner"
grep -q -F -e '"c": ["ADR 0019"' "$parity" || deferred_fail="$deferred_fail c:owner"
grep -q -F -e '"cpp": ["ADR 0019"' "$parity" || deferred_fail="$deferred_fail cpp:owner"
grep -q -F -e 'PARITY_DEFERRED = {' "$parity" || deferred_fail="$deferred_fail shape:missing"
grep -q -F -e 'REAL_ADAPTERS = {' "$adapters" || deferred_fail="$deferred_fail adapters:missing"
grep -q -F -e 'REAL_CLASS_TO_FAMILY = {' "$adapters" || deferred_fail="$deferred_fail taxonomy:missing"
if [[ -z "$deferred_fail" ]]; then
  ok
else
  bad "parity deferrals drifted:$deferred_fail"
fi

# Every adapter-backed class has real subjects plus matrix evidence.
# The eleven backed classes: javascript, json, jsx, markdown, python,
# python_stub, rust, starlark, toml, tsx, typescript. python_stub rides
# generated .pyi matrix cases; the rest ride real_source_target subjects.
class_fail=""
for cls in javascript json jsx markdown python rust starlark toml tsx typescript; do
  grep -q -F -e "$cls" "$subjects" || class_fail="$class_fail $cls:subject"
done
grep -q -F -e 'python_stub' "$matrix" || class_fail="$class_fail python_stub:matrix"
grep -q -F -e 'matrix_rust_format_pass' "$matrix" || class_fail="$class_fail rust:matrix"
grep -q -F -e 'matrix_python_lint_pass' "$matrix" || class_fail="$class_fail python:matrix"
grep -q -F -e 'matrix_javascript_lint_pass' "$matrix" || class_fail="$class_fail javascript:matrix"
grep -q -F -e 'matrix_json_lint_pass' "$matrix" || class_fail="$class_fail json:matrix"
grep -q -F -e 'matrix_markdown_lint_pass' "$matrix" || class_fail="$class_fail markdown:matrix"
grep -q -F -e 'matrix_starlark_lint_pass' "$matrix" || class_fail="$class_fail starlark:matrix"
grep -q -F -e 'matrix_toml_lint_pass' "$matrix" || class_fail="$class_fail toml:matrix"
if [[ -z "$class_fail" ]]; then
  ok
else
  bad "adapter-backed class evidence drifted:$class_fail"
fi

# Every adapter-backed tool has parser plus matrix pass/fail evidence.
# Sixteen tools: biome, buildifier, clippy, eslint, flake8, markdown_check,
# prettier, pydoclint, pylint, ruff, rustc, rustfmt, taplo, tsc, ty, vale.
# tsc is target-coupled (needs TsConfigInfo) so its evidence lives in
# quality/tools/typescript/BUILD.bazel plus the tsc parser, not the
# provider-less runner matrix.
tool_fail=""
for tool in biome buildifier eslint flake8 markdown prettier pydoclint pylint ruff rustfmt taplo tsc ty vale; do
  [[ -f "quality/adapter/src/parsers/$tool.rs" ]] || tool_fail="$tool_fail $tool:parser"
done
# clippy/rustc parse through rust.rs delegated diagnostics, not separate files.
grep -q -F -e 'parse_clippy' quality/adapter/src/parsers/rust.rs || tool_fail="$tool_fail clippy:parse"
grep -q -F -e 'parse_rustc' quality/adapter/src/parsers/rust.rs || tool_fail="$tool_fail rustc:parse"
for tool in biome buildifier clippy eslint flake8 markdown_check prettier pydoclint pylint ruff rustc rustfmt taplo ty vale; do
  grep -q -F -e "$tool" "$matrix" || tool_fail="$tool_fail $tool:matrix"
done
# tsc target-coupled evidence: authoritative mapping plus parser plus
# aspect coupling note, never a bare-file matrix case. The coupling-note
# match is case-neutral (`coupled tsc`) so sentence-case comments
# (`Target-coupled tsc` in real_aspects.bzl) satisfy it.
grep -q -F -e 'tsc' quality/tools/typescript/BUILD.bazel || tool_fail="$tool_fail tsc:mapping"
grep -q -F -e 'coupled tsc' "$aspects" || tool_fail="$tool_fail tsc:coupled"
# Pass plus fail cells exist for the matrix (dirty plus clean).
grep -q -F -e '_fail' "$matrix" || tool_fail="$tool_fail matrix:fail-cells"
grep -q -F -e '_pass' "$matrix" || tool_fail="$tool_fail matrix:pass-cells"
# Fix/format replacements exist where applicable (formatters emit replacements).
grep -q -F -e 'replacement ' "$matrix" || tool_fail="$tool_fail matrix:replacements"
if [[ -z "$tool_fail" ]]; then
  ok
else
  bad "adapter-backed tool evidence drifted:$tool_fail"
fi

# Native-config bindings exist for the seven tools that take native policy;
# the other nine are explicitly config-free, upstream-delegated,
# target-coupled, or check-only by design (no native-config rule expected).
config_fail=""
for tool in biome buildifier eslint ruff rustfmt taplo vale; do
  grep -q -F -e "\"$tool\":" "$native" || grep -q -F -e "_NATIVE_CONFIG_EXTENSIONS" "$native" || config_fail="$config_fail $tool:extension"
  grep -q -F -e "${tool}_config" "$native" || config_fail="$config_fail $tool:rule"
done
# Config-free/delegated status is documented in code, not silently missing.
grep -q -F -e 'check-only' "$adapters" || config_fail="$config_fail adapters:check-only-note"
grep -q -F -e 'upstream-delegated' "$aspects" || grep -q -F -e 'Delegated' "$aspects" || config_fail="$config_fail aspects:delegated-note"
grep -q -F -e 'target-coupled' "$aspects" || config_fail="$config_fail aspects:coupled-note"
grep -q -F -e 'no usable upstream default' "$aspects" || config_fail="$config_fail aspects:vale-note"
if [[ -z "$config_fail" ]]; then
  ok
else
  bad "native-config bindings drifted:$config_fail"
fi

# Standalone artifact metadata plus regeneration policy: six tools pinned
# with immutable URL, digest, size, executable, licenses, plus the
# maintainer regeneration entry point with --verify-only.
artifact_fail=""
for tool in biome buildifier ruff taplo ty vale; do
  [[ -f "quality/artifacts/$tool.linux_x86_64.bzl" ]] || artifact_fail="$artifact_fail $tool:file"
  grep -q -F -e "$tool" "$metadata" || artifact_fail="$artifact_fail $tool:pin"
  grep -q -F -e "\"$tool\":" "$update" || grep -q -F -e "'$tool':" "$update" || grep -q -F -e "\"$tool\"" "$update" || artifact_fail="$artifact_fail $tool:update"
done
grep -q -F -e 'Regenerate with: bazel run //quality/artifacts:update' quality/artifacts/biome.linux_x86_64.bzl || artifact_fail="$artifact_fail regen:biome"
grep -q -F -e '--verify-only' "$update" || artifact_fail="$artifact_fail regen:verify-flag"
grep -q -F -e 'metadata_tests' quality/artifacts/BUILD.bazel || artifact_fail="$artifact_fail build:metadata"
grep -q -F -e 'FROZEN_SCHEMA_KEYS' "$metadata" || artifact_fail="$artifact_fail schema:frozen"
if [[ -z "$artifact_fail" ]]; then
  ok
else
  bad "artifact metadata/regeneration drifted:$artifact_fail"
fi

# Packaging single-correct-path plus manifest completeness plus candidate
# wire profile URIs plus verification binding (qualification planning,
# the issue tracker own execution; this crate never signs or publishes).
packaging_fail=""
grep -q -F -e 'packaging_uses_single_correct_path' "$qual" || packaging_fail="$packaging_fail packaging:fn"
grep -q -F -e 'manifest_covers_payload' "$qual" || packaging_fail="$packaging_fail manifest:fn"
grep -q -F -e 'CANDIDATE_SPDX_PREDICATE_URI' "$qual" || packaging_fail="$packaging_fail spdx:uri"
grep -q -F -e 'CANDIDATE_SLSA_PREDICATE_URI' "$qual" || packaging_fail="$packaging_fail slsa:uri"
grep -q -F -e 'CANDIDATE_STATEMENT_TYPE_URI' "$qual" || packaging_fail="$packaging_fail statement:uri"
grep -q -F -e 'attestation_binds_exact_bytes' "$qual" || packaging_fail="$packaging_fail attestation:fn"
grep -q -F -e 'verify_rejection' "$qual" || packaging_fail="$packaging_fail verify:fn"
grep -q -F -e 'self_attested_level_admissible' "$qual" || packaging_fail="$packaging_fail self-attested:fn"
if [[ -z "$packaging_fail" ]]; then
  ok
else
  bad "packaging/provenance qualification drifted:$packaging_fail"
fi

# Buildifier/Taplo/Vale probes stay provisional: research notes are
# unproven mappings, versions are observations not pins. Biome/ESLint/
# Prettier/tsc carry direct-probe evidence with pinned versions.
provisional_fail=""
grep -q -F -e 'unproven mappings' "$integrations" || provisional_fail="$provisional_fail integrations:unproven"
grep -q -F -e 'observations, not pins' "$integrations" || provisional_fail="$provisional_fail integrations:observations"
grep -q -F -e 'Direct probes show' "$integrations" || provisional_fail="$provisional_fail integrations:direct-probes"
if [[ -z "$provisional_fail" ]]; then
  ok
else
  bad "provisional probe record drifted:$provisional_fail"
fi

dx_test_summary "quality adapters parity harness"
