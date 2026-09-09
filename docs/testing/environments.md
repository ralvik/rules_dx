# Environment, Codegen, And Setup Test Matrix

This matrix covers the environment, codegen, and setup workflows. The authoritative
contracts are [Developer Environments](../environments/environment.md),
[Python Environment](../environments/python-environment.md), and
[Code Generation](../environments/codegen.md).

## Consumer Fixtures

Python environment fixtures satisfy [Python Environment](../environments/python-environment.md).
Cross-language selection, PATH-tool, ownership, and platform fixtures satisfy
[Developer Environments](../environments/environment.md). Run both suites against real
Bzlmod consumer workspaces rather than mocked provider graphs.

Python configuration tests require no language or version declaration for the tested release
default. They verify that default on every selected platform, optional alternate registration,
effective-default selection, and rejection of versions outside the declared `pyproject.toml`
`requires-python` range. Binary, test, venv, and native dependency graphs test default
inheritance and per-target overrides. Metadata ranges never choose the toolchain automatically.

Equivalent tests apply to each language that exposes selectable versions. They
cover registered-set validation, default membership, per-target graph selection,
version-neutral source libraries, environment defaults, and rejection of ignored
version settings for non-versioned integrations.

## Automatic Editor Analysis

Verify the approved Rust and Go integrations under
[Ownership And Refresh](../environments/environment.md#ownership-and-refresh). For Go, drive `gopls`
package requests through the pinned upstream rules_go `GOPACKAGESDRIVER`, not a mocked package graph.
Source and declared BUILD-graph changes must become visible through upstream requests without an
extra `dx setup` refresh while the managed launcher remains usable.

- Capture Bazel queries/builds and downloads; requests use the selected configuration and isolated
  IDE output base. Exact-target mode must not widen to unrelated packages or configurations.
- Verify repeated requests reuse upstream/Bazel state; failures, cancellation, and concurrent editor
  requests must not produce a successful incomplete graph or silently fall back to ambient Go tools.
- Verify editor requests never invoke Gazelle, update manifests/locks, mutate tracked BUILD files,
  replace `.dx/setups/current`, or refresh managed environment/codegen selections.
- Verify an unused Go foundation performs no driver work or payload acquisition. Required editor
  requests may acquire only their declared graph/tool closure and never bypass license acceptance.
- Exercise pure Go, generated-source, build-constraint, and cgo fixtures on every claimed platform.
  Record unsupported cgo completion and diagnostic behavior as gaps rather than treating pure-Go
  success as full IDE qualification.
- Verify missing launchers/dangling projections retain documented explicit recovery, while normal
  metadata refresh does not require a manual setup step. For other explicit-only integrations,
  document the upstream limitation or additional complexity justifying that choice under the
  [automatic-workflow policy](../product/scope.md#automatic-workflows).

## BEP And Projection Tests

Verify BEP-based collection with local execution, a clean output base, and remote
output materialization. Tests must fail if the collector depends on output-tree
layout or discovers artifacts that were not reported by Bazel.

Remote fixtures verify that every artifact referenced by an environment or codegen shard
is downloaded through its requested private output group before symlink commit, while an
unrelated build output may remain remote-only. No CLI network fetch is permitted.

Warm reuse tests verify that every invocation still obtains the current plan from Bazel,
accepts exact metadata and managed links, and reconstructs or fails safely for malformed,
missing, unexpected, or incorrectly targeted links. Instrumented fixtures verify that
`dx` does not read and independently hash every Bazel-owned artifact during reuse.

Environment and codegen fixtures verify one normalized binary Protobuf shard per
contributing configured target, transitive depset deduplication, deterministic Rust
merge independent of BEP ordering, and one private shard/artifact output group per
capability. They reject malformed shards and missing, duplicate, or unreported artifact
execution paths.

Ruleset fixtures prove that authoritative transitive providers or narrow adapters retain
required semantic dependencies without traversing unrelated `data`, tool, or other
dependency-like edges. Unsupported rule kinds fail closed instead of silently producing
partial plans. Analysis benchmarks compare configured-aspect counts and memory against a
broad traversal implementation to retain the adapter model's efficiency rationale.

## Command Tests

- Verify no-argument `codegen` invokes only `//dx:codegen`; target `codegen` accepts
  one explicit label and rejects paths, patterns, multiple labels, profiles, and
  language selectors.
- Verify codegen selects an empty exact projection when the closure has no generated
  sources and does not invoke generate or env.
- Verify no-argument `env` invokes only the repository-defined environment target.
- Verify target `env` accepts one explicit label and rejects paths, patterns, multiple
  labels, profiles, and language selectors.
- Verify `env` does not launch a shell or forward application arguments to
  environment tools.
- Verify env does not invoke codegen, and both commands report non-fatal selection-scope
  mismatches with the corresponding command name and scope kind without rendering argv.
- Verify setup accepts no scope or one explicit label, rejects other scope forms, does
  not invoke Gazelle, and selects neither new generation when either preparation fails.
- Verify setup performs one Bazel request with the root union, both aspects/output
  groups, and one BEP stream, while preserving provider-specific applicability and
  deduplicating common configured closures.
- Verify env carries forward selected codegen, codegen carries forward selected env,
  and setup commits both through one `.dx/setups/current` replacement without observable
  half-updated state.
- Verify concurrent Bazel preparations overlap, commit phases serialize, and racing env
  and codegen commands re-read current selection under the lock without losing either
  update. Verify identical immutable installation is idempotent, conflicting records fail
  without pointer replacement, interruption releases the OS advisory lock, a short wait
  succeeds, and timeout reports a busy workspace without breaking lock ownership.
- Verify first independent runs use typed empty counterpart generations, `.dx/bin`
  bootstrap remains independent of setup selection, and current setup records cannot be
  removed as unselected cache state.
- Verify `dx clean` prunes only validated unselected generations and setup records, preserves
  `.dx/setups/current` and its selected generations, refuses unmanaged or digest-spoofed
  paths, and recovers dangling links only through explicit `env`, `codegen`, or `setup`
  workflows. Verify `dx clean --bazel` forwards `bazel clean` with recovery guidance and
  `--dry-run` deletes nothing.
- Verify direnv scaffolding per [Direnv Integration](../environments/environment.md#direnv-integration):
  `dx init` writes the committed `.envrc` absent-only and refuses an existing unmanaged
  `.envrc` without overwrite; the snippet adds `.dx/bin` to `PATH` only, watches `.dx/bin`,
  and errors with `dx env` or `bazel run //dx:env` guidance when the directory is missing
  instead of invoking Bazel; `dx env` reports direnv hook and `PATH` status; no shell
  profile, registry, or global environment mutation occurs on any host.

Environment configuration and public API conformance requirements are maintained in
[Developer Environments](../environments/environment.md).
