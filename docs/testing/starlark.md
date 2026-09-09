# Starlark Testing

This contract details the project-owned `starlark_test` facade used under the
pinned Bazel version. The [testing strategy](README.md) indexes it alongside
the CLI, CI, generation, environment, tool, and quality matrices.

## Modes

Authors use the unified `starlark_test` macro with an explicit load, unit, or
analysis mode. Load tests cover public `.bzl` entry points. Unit tests exercise
pure Starlark functions and values. Analysis tests inspect rules, aspects,
providers, actions, inputs, arguments, environment, execution requirements,
toolchain resolution, target tags, output groups, configurations, and expected
failures. Conformance tests verify that each mode delegates to the appropriate
Bazel loading or analysis mechanism rather than emulating phase behavior.

Quality result, diagnostic, evaluator, cache, deterministic-output, apply-safety,
and tool-parity suites are defined in
[Quality Workflow Testing](../quality/quality-testing.md).

## Authoring

Test functions live in test `.bzl` files and use the project-owned `expect` API.
The test module exports callable test definitions generated while it loads; BUILD
files instantiate them as ordinary named test targets. Tests cover binding
failures, duplicate names, common test attributes, assertion diagnostics, and
attempts to use a function in the wrong test mode.

Assertion conformance covers primitive values, strings, collections,
dictionaries, errors, targets, providers, actions, files, depsets, runfiles,
and custom provider subjects. Diagnostic tests require deterministic
expected/actual rendering.

Negative tests match the expected load, unit, or analysis phase and required
diagnostic substrings. They also prove that the wrong phase, a successful
target, or a failure missing any required fragment does not pass.

Assertion tests verify that multiple mismatches are retained and rendered in
declaration order. Separate cases verify that unrecoverable framework errors
fail immediately and are labeled as infrastructure errors rather than
mismatches.

Each function is one Bazel test target. Conformance tests verify independent
filtering, tags, cache status, retries, failure reporting, and optional grouping
through native `test_suite`.

## Orchestration And Protocol

Rust orchestration runs Bazel fixtures and consumes ordinary Bazel test and BEP
results across the required platform matrix. Rust does not interpret Starlark or
emulate Bazel analysis.

`starlark_test` follows Bazel's standard test protocol, including exit status,
test logs, and `XML_OUTPUT_FILE` when Bazel supplies it. Bazel and BEP own test
result collection. No custom per-test JSON transport is introduced.
