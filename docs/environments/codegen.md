# Generated Code

`dx codegen` builds Bazel-owned generated source artifacts and exposes
stable, read-only filesystem projections for language servers and editors.
Open work is tracked in GitHub issues.

## Purpose

`dx codegen` does not update BUILD files, invoke Gazelle, copy generated
files into source packages, or prepare language runtimes and package
environments.

The command is independent from `dx generate` and `dx env`:

- `dx generate` updates Gazelle-maintained BUILD files and codegen wrappers.
- `dx codegen` executes registered generators and selects generated-source
  projections.
- `dx env` prepares and selects interpreters, dependencies, language-native
  environments, and terminal tools.

Neither `dx codegen` nor `dx env` invokes the other. Each reports when its
selected scope differs from the other projection and identifies the command
and scope kind needed to align it. A mismatch is not a command failure. In
NDJSON mode it is a `notice` with code `selection_scope_mismatch`.

`dx setup [target]` explicitly prepares both workflows for one scope. It
commits neither selection unless environment and codegen preparation and
validation both succeed. One Bazel request applies both aspects to the union
of required roots and requests both output groups.

Environment and LSP configuration always follows the stable
`.dx/setups/current/generated` path rather than pinning one generated
generation in the environment identity. A successful `dx codegen` selection
is therefore visible immediately without another `dx env`.

## Scope

With no argument, `dx codegen` selects every codegen projection in the
currently declared Bazel BUILD graph and atomically selects one
repository-wide generated-source projection. It does not inspect unowned
files or decide that BUILD metadata ought to exist. The canonical selection
identity is the empty `//dx:codegen` filegroup. The physical root-selection
mechanism is frozen to the `//...` baseline. Repository-wide selection
includes every registered production, test, example, and development
projection.

`dx codegen <target>` accepts exactly one explicit target label. A
provider-collecting aspect derives generated source artifacts and language
import/source roots from that configured target's transitive closure, plus
its reverse-dependent expansion to registered projections via one
unconfigured `bazel query`. An empty projection set selects an empty exact
projection. A bare schema expands to every registered generated-language
projection consuming that schema. Missing wrappers are absent BUILD graph
facts, not stale-metadata errors; users run `dx generate` when they want
Gazelle to create them.

`dx setup <target>` with a capability-absent side carries the selected
generation forward, or the managed empty generation on first run.

Paths, directories, target patterns, multiple labels, profiles, and language
selectors are rejected. A compatible target whose closure has no generated
sources successfully selects an empty exact projection so stale generated
imports from a prior selection do not remain visible.

## Repository Root Selection

First-party `rules_dx` Gazelle extensions discover supported sources and may
create language projection wrappers required by authoritative upstream rules.
Attributes requiring project or dependency metadata still come only from
authoritative manifests, lockfiles, package-manager outputs, and public
upstream Bazel metadata. Once written, those wrappers are ordinary BUILD
graph targets.

The correctness baseline applies the codegen collection aspect to `//...`.
A monolithic aggregate rule and package-local aggregate shards are rejected
alternatives, not pending measurement. No candidate may reduce roots based
only on an unconfigured query graph: transitions, toolchains, and `select()`
can make a consumer closure differ from a directly selected dependency.

All candidates operate on BUILD metadata present when the command starts.
Adding an unowned file or schema does not make setup fail and does not
participate until a BUILD target or wrapper is declared. No env, codegen, or
setup command runs a Gazelle freshness preflight.

## Provider Contract

Codegen integrations use verified authoritative transitive upstream providers
wherever they identify generated artifacts and their import/source-root
mapping. When required metadata is absent, a narrow ruleset-specific adapter
normalizes the upstream semantics into the internal projection provider.
Unsupported rule kinds fail closed. Collectors do not fall back to
`DefaultInfo.files` or traverse a broad union of dependency-like attributes.

Each projection record identifies the producer target and output
language/file class, generated artifacts or tree artifacts, deterministic
logical workspace path and import/source-root mapping, required package or
namespace semantics, and read-only status with any runtime dependency context.

Multiple producers claiming an incompatible language import path fail before
selection. A generated logical module/path that conflicts with checked-in
source also fails before selection, unless the claiming entry carries the
explicit tested replacement contract (`replaces` equal to the logical path
with a non-empty `exec_path` binding the replacing generated artifact).
Generated-to-generated duplicates require identical artifacts and explicit
mergeable semantics, otherwise they fail.

Each contributing configured target emits one small normalized binary
Protobuf plan shard, collected in transitive depsets. One private codegen
output group contains the collected shards and every referenced generated
artifact, ensuring generator actions execute and remote outputs materialize
locally. The CLI recognizes shards only among BEP-reported files and rejects
missing, duplicate, or unreported artifacts without scanning `bazel-out`.
With remote cache or execution, the Bazel request downloads the requested
output group; `dx` never implements a second remote-cache downloader.

## Filesystem Projection

Canonical generated artifacts remain under Bazel's output tree. Within each
immutable generated-code generation, deterministic logical workspace-relative
paths mirror generated files. Each mirror leaf is a managed symlink to its
Bazel output; storage is not divided into top-level language directories. The
stable `.dx/setups/current/generated` path exposes the selected mirror.
Shared directory layout, identities, reuse validation, selection, empty
counterparts, atomic commit, ownership, and retention are defined in
[Managed Environment State](managed-state.md).

Language integrations may configure LSPs with the complete stable mirror or
with provider-derived root directories inside it. There is no root
`.generated` facade and no generated symlink is inserted beside checked-in
source files. Generated paths are read-only quality context. `bazel clean`
or an output-base change can make links stale; only explicit `dx codegen`
repairs the projection.

## Scaling Model

The repository-wide cold run is proportional to the selected graph and must
execute uncached generator actions. Bazel's persistent server and action
cache improve unchanged warm runs by design; no timing claims are made.
Concurrency means parallel Bazel preparation with serialized commit plus
re-read under the commit lock; there is no multi-workspace coordination.
Interruption preserves the prior pointer, and Bazel owns mid-action state.
Remote materialization is Bazel-owned download of the requested output group
with fail-closed local collection; remote cache and execution stay unwired.
Reuse certifies per-workspace against the current BEP result only; Bazel
stays authoritative for freshness. Root selection stays on the frozen `//...`
baseline.

Exact-target runs bypass repository root selection and analyze only one
configured target closure. `dx` maintains no independent generator cache or
persistent semantic graph.

## Admitted-Pairs Evolution and New Generator Onboarding

The admitted generator/language pairs stay frozen in
`generation/codegen.bzl:DX_CODEGEN_ADMITTED_PAIRS`. Adding a pair edits that
registry data only and follows the checklist below. No pair is admitted by
editing call sites or adding a second registry.

Each newly admitted pair must prove the same five evidence slices the first
pair proves: provider contract, BEP output groups, projection, roots, and
cold-warm. Per-pair fixtures plus qualification coverage are required for
every pair, not just the first.

Onboarding checklist (every box required per pair):

- Registry: append `(schema_kind, language)` with schema and pair errors
  green and the registry accessor returning the data unchanged.
- Provider contract: narrow adapter normalizing verified authoritative
  transitive upstream providers; unsupported rule kinds fail closed with no
  `DefaultInfo` fallback; one normalized plan shard per contributing target
  with transitive collection and deterministic merge plus conflict handling.
- BEP output groups: private output group carrying shards plus every
  referenced artifact with the reserved shard suffix; the CLI recognizes
  shards only among BEP-reported files and rejects missing, duplicate, or
  unreported artifacts; remote outputs materialize through the requested
  output group.
- Projection: deterministic logical paths with a symlink-only read-only
  mirror under `.dx/setups/current/generated`; collisions fail closed unless
  the claiming entry carries the explicit tested replacement contract.
- Roots: frozen `//...` baseline plus exact-target bypass plus bare-schema
  expansion; declared-BUILD-only with no Gazelle freshness preflight.
- Cold-warm: cold proportional to the selected graph with warm reuse through
  the persistent server plus action cache and no timing claims.
- Per-pair fixtures: one shard plus one adapter-rule fixture plus one plan
  subject with fingerprint, plus qualification fixture entries naming the
  pair and its fixture labels.
- Qualification: qualification iterates every admitted pair and proves its
  fixtures build plus its provider, BEP, projection, roots, and cold-warm
  evidence; Starlark unit plus analysis tests stay green.

## Test Requirements

Codegen tests must cover repository-wide `//...` and exact-target selection
with equivalent effective roots and outputs; inclusion of production, test,
example, and development projections; bare-schema expansion; empty exact
projections and transitions between selections; Gazelle-created wrappers;
unowned files staying outside setup without implicit Gazelle execution;
generated files, tree artifacts, import roots, namespaces, paths with spaces,
and incompatible collisions; per-contributor shards with deterministic merge
and fail-closed artifact validation; checked-in/generated collisions with the
explicit replacement contract only; workspace-relative mirror paths with leaf
symlinks; read-only generated artifacts; stale links after `bazel clean`;
deterministic root-candidate coverage on the frozen baseline; scope-mismatch
warnings without implicit cross-command execution; immediate LSP visibility
after pointer replacement; and admitted-pairs onboarding with per-pair
fixtures plus qualification coverage.

Cross-cutting fixture, platform, remote, and evidence requirements remain in
[Testing Strategy](../testing/README.md). Shared identity, pointer, reuse,
carry-forward, concurrency, ownership, and retention requirements remain in
[Managed Environment State](managed-state.md#test-requirements).
