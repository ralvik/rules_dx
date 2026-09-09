# ADR 0009: Project-Owned Starlark Testing

## Status

Accepted.

## Context

The repository needs direct tests for `.bzl` files, including pure functions,
rules, aspects, providers, actions, and expected analysis failures. Depending on a
lightly released external assertion framework would not provide the desired
ownership or solve Starlark coverage. A separate Starlark interpreter would test
different semantics from Bazel.

## Decision

The project owns a Bazel-native Starlark testing capability. It ships as part of
the single `rules_dx` Bzlmod module and release and is reusable by consumer
repositories. Tests load and execute `.bzl` code through the exact supported Bazel
version. The capability provides:

- Load tests for public `.bzl` entry points and symbols.
- Unit-test-style assertions for pure Starlark functions and values.
- Analysis tests for rules, aspects, providers, registered actions, toolchains,
  configurations, output groups, and expected failures.
- Ordinary Bazel test protocol behavior, logs, and standard test outputs suitable
  for Bazel, BEP, and CI collection.
- Small fixture helpers without mocking or reimplementing Bazel semantics.

The initial public authoring API is one `starlark_test` macro facade with an
explicit test mode. It dispatches to internal load, unit, or analysis test
implementations because those modes execute in different Bazel phases. The unified
facade does not pretend that one rule implementation can directly invoke arbitrary
loaded functions or rule implementation functions.

Test cases and assertions are authored as functions in test `.bzl` files using a
small project-owned `expect` API. The public `starlark_test` factory binds a test
function and mode to a callable test rule or macro during loading. The test `.bzl`
file exports that callable, and BUILD instantiates it with an ordinary target name,
tags, visibility, and other common test attributes. BUILD files do not encode
assertion data.

The initial `expect` API includes subjects for primitive values, strings,
collections, dictionaries, errors, targets, providers, actions, files, depsets,
and runfiles. It supports custom subjects for project-specific providers. Broad
matcher, transformation, and snapshot APIs require concrete use cases rather than
being included speculatively. The exact `expect` subject and matcher surface is provisional
pending M01 qualification; do not treat the list above as frozen API through this record.

Expected-failure tests identify the required failure phase (`load`, `unit`, or
`analysis`) and one or more diagnostic substrings. The framework may retain the
complete normalized diagnostic for reporting, but exact full-message snapshots are
not the default contract because Bazel wording can change when the pinned version
is updated.

Expectation mismatches accumulate within one test function and are reported
together in deterministic declaration order. An infrastructure error that makes
continued evaluation invalid may fail immediately, but diagnostics must distinguish
that error from collected expectation failures.

Each bound test function is instantiated as one addressable Bazel test target.
Tests may be grouped with ordinary `test_suite` targets, but grouping does not
combine execution, caching, retries, tags, or failure reporting.

This intentionally resembles language test rules such as `python_test` at the
BUILD layer, but it cannot accept an arbitrary `.bzl` file in `srcs` and discover
functions dynamically. Bazel `load()` statements are static, and rule attributes
cannot carry Starlark function values. The API must not hide that constraint with
a second interpreter.

Rust may orchestrate real Bazel fixture workspaces and consume Bazel test/BEP
results across platforms. It does not interpret Starlark or emulate loading,
analysis, providers, aspects, toolchains, transitions, or actions.

Tests use Bazel's standard test contract: process exit status, test logs, and
Bazel-provided output paths such as `XML_OUTPUT_FILE` when applicable. The
framework does not introduce a separate per-test JSON result protocol. Behavioral
matrix metadata is a build/test manifest and is not a replacement result stream.

The custom tooling must first investigate genuine Starlark executable-line
instrumentation under the pinned real Bazel. The central
[coverage policy](../testing/README.md#coverage) permits a checked behavioral matrix
only when documented evidence demonstrates that reliable line measurement is
infeasible. The fallback covers entry points, functions, rules, aspects, providers,
attributes, configurations, actions, and failure paths with meaningful assertions;
it is never reported as source-line or branch coverage. Loaded files, test counts,
and mappings alone do not establish coverage. Instrumentation, reasoned ignore
support, feasibility evidence, and measurement mechanics follow the resolved
[coverage policy](../testing/README.md#coverage) (O47 resolved 2026-09-09).

## Consequences

- `rules_dx` does not require `rules_testing` for its canonical Starlark tests.
- Framework conformance tests run under the pinned Bazel version on every required
  OS/CPU pair.
- Implementation languages receive Bazel-owned source coverage under the central
  policy. Starlark may use the evidence-backed behavioral fallback only after the
  real-Bazel instrumentation investigation; mutation tests may supplement it.
- Starlark production logic remains small, with parsing and stateful processing in
  Rust where practical.
- The Starlark testing capability shares `rules_dx` versioning, compatibility,
  platforms, and release gates; it is not independently resolved or published.
- Internal test rule types may differ by phase without becoming separate public
  authoring APIs.
- Tests remain readable Starlark programs, while BUILD files instantiate ordinary
  test targets instead of becoming declarative assertion manifests.
- Per-function targets provide precise Bazel filtering, caching, retries, and
  diagnostics at the cost of additional analysis targets.
- The initial assertion API can test `rules_dx` internals without committing to a
  large general-purpose matcher library.
- Expected-failure tests prove the relevant error without coupling every test to
  complete Bazel diagnostic text.
- Collecting mismatches reduces repeated test cycles while preserving one Bazel
  result per test function.
- Existing Bazel and CI test collection works without a `rules_dx`-specific result
  adapter.

## Rejected Alternatives

- Implementing or embedding a second Starlark interpreter.
- Treating fixture builds alone as sufficient unit and analysis testing.
- Claiming `.bzl` line coverage from test counts or loaded files.
- Exposing an external testing framework as the permanent public test API.
- A second Bzlmod module or separate release for the testing capability.
- Encoding provider and action assertions as large BUILD attribute structures.
