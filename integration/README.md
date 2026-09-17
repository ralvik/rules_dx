# Stage 4 E2E (issue #55): CLI-contract end-to-end suite

Thin true end-to-end layer pinning the `dx ... --check` exit contract
in consumer-shaped child workspaces. Deliberately few scenarios, not a
per-language matrix (slow nested-Bazel layer; the matrix lives in
Layer-2). Each scenario is a standalone Bazel workspace staged to a
scratch dir at test time; the driver runs the parent-built `dx` binary
against it with a version-pinned Bazel first on `PATH`.

## Layout

- `clean/` — all-green workspace: `dx test //...` must exit 0.
- `dirty/` — one failing `sh_test`: `dx test //...` must exit 3
  (Bazel test-failure code, preserved verbatim by `dx` per the CLI
  contract). Paired with `clean/`, this is the negative control:
  always-pass detectors fail `dirty`, always-fail detectors fail
  `clean`.
- `format-roundtrip/` — planned slice 2 (format round-trip). The suite
  stays red-free: only landed scenarios are wired into `//tools/ci:e2e`.

## Why `integration/` is invisible to Bazel

`.bazelignore` carries `integration/`, so the parent universe never
sees these files: `bazel build //...`, `bazel test //...`, and every
`dx ... //...` scope (all resolved through `bazel query`) skip this
tree, and the corpus audit carves it out (see `tools/ci/corpus_audit.sh`).
Scenario files reach the nested run by shell copy, never by label.

## Explicit invocation (the only way to run the suite)

```sh
E2E_WORKSPACE=$PWD bazel test //tools/ci:e2e --test_env=E2E_WORKSPACE
```

`bazel test` runs drivers in a sandbox without `.git` and without
`BUILD_WORKSPACE_DIRECTORY`, so the harness locates the parent checkout
via `$E2E_WORKSPACE` (CI passes `E2E_WORKSPACE=$GITHUB_WORKSPACE` with
`--test_env=E2E_WORKSPACE`). `bazel run` from the checkout still works
via `git rev-parse` fallback. The drivers are `manual` (+`exclusive`,
`local`, `no-sandbox`): wildcard expansion skips
them under both `bazel test //...` and `dx test --check //...` (the
`//...` pattern passes through `resolve_for_test` unresolved, so both
CLIs inherit Bazel's manual-tag semantics). `bazel query` still lists
them. File-scoped `dx test` over files under `integration/` fails
before execution (no query owner): E2E scenarios run only via the
explicit suite label above. CI invokes the suite in its own `e2e` job.

## Negative control

`clean/` (exit 0) plus `dirty/` (exit 3) prove the suite is not
theater: the same harness asserts opposite exit codes over staged
workspaces that differ by one failing test. A deliberately broken
detector stops failing exactly one half (always-pass breaks `dirty`,
always-fail breaks `clean`). To re-prove manually, invert one
expectation (e.g. run the driver with `dirty` + want-exit `0`) and
observe the harness fail. The `format-roundtrip/` scenario remains
slice 2 (quality-adapter staging in the child, heavier than the
test-contract pin).

## Consumer-ci verdict (open decision, kept alternative)

The `consumer-ci` self-call stays build-only. The E2E suite asserts
CLI behavior; the self-call smoke-tests the versioned consumer
workflow file. Different purposes, both cheap: no deletion.
