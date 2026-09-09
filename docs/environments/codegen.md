# Generated Code

## Purpose

`dx codegen` builds Bazel-owned generated source artifacts and exposes stable,
read-only filesystem projections for language servers and editors. It does not update
BUILD files, invoke Gazelle, copy generated files into source packages, or prepare
language runtimes and package environments.

The command is independent from `dx generate` and `dx env`:

- `dx generate` updates Gazelle-maintained BUILD files and codegen wrappers.
- `dx codegen` executes registered generators and selects generated-source
  projections.
- `dx env` prepares and selects interpreters, dependencies, language-native
  environments, and terminal tools.

Neither `dx codegen` nor `dx env` invokes the other. Each reports when its selected
scope differs from the other projection and identifies the command and scope kind needed
to align it without rendering command-line arguments. A mismatch is not a command failure.
In NDJSON mode it is a `notice` with code `selection_scope_mismatch`.

`dx setup [target]` explicitly prepares both workflows for one scope. It commits neither
selection unless environment and codegen preparation and validation both succeed. One
Bazel request applies both aspects to the union of required roots and requests both
output groups, rather than executing the independent command plans sequentially.

Environment and LSP configuration always follows the stable
`.dx/setups/current/generated` path rather than pinning one generated generation in the
environment identity. A successful `dx codegen` selection is therefore visible
immediately without another `dx env`. Scope identities are compared only for warnings
and diagnostics; they do not couple the two lifecycles.

## Scope

With no argument, `dx codegen` selects every codegen projection in the currently
declared Bazel BUILD graph and atomically selects one repository-wide generated-source
projection. It does not inspect unowned files or decide that BUILD metadata ought to
exist. The physical root-selection mechanism remains provisional pending the benchmark
and legality checks below.

Repository-wide selection includes every registered production, test, example, and
development projection. It does not omit test-only generators to improve performance.
Users select one consumer or schema target when they want a smaller projection.

`dx codegen <target>` accepts exactly one explicit target label. A provider-collecting
aspect derives generated source artifacts and language import/source roots from that
configured target's transitive closure. It builds and selects only that exact
projection; no generated sibling target is required for the selected consumer.

When the label is a registered bare schema target rather than a language consumer,
codegen builds every declared generated-language projection that directly or through a
tested projection edge consumes that schema. Because an aspect cannot traverse reverse
dependencies, the CLI asks Bazel query for registered projection reverse dependents and
passes the returned labels back to Bazel analysis. Rust does not infer wrappers or
language semantics. Missing wrappers are absent BUILD graph facts, not stale-metadata
errors; users run `dx generate` when they want Gazelle to create them.

`dx setup <bare-schema>` uses the same all-projection expansion and carries the selected
environment generation forward because the schema has no environment capability. Its first-run
empty counterpart and later carry-forward behavior follow
[Managed Environment State](managed-state.md#selection-and-carry-forward).

Paths, directories, target patterns, multiple labels, profiles, and language selectors
are rejected. A compatible target whose closure has no generated sources successfully
selects an empty exact projection so stale generated imports from a prior selection do
not remain visible.

## Repository Root Selection

During explicit generation, first-party `rules_dx` Gazelle extensions discover supported sources and may create language
projection wrappers required by authoritative upstream rules, such as a Python proto projection,
without requiring a prior manifest or target. Attributes requiring project or dependency metadata
still come only from authoritative manifests, lockfiles, package-manager outputs, and public
upstream Bazel metadata. The extensions do not reconstruct generator or dependency semantics.
Once written, those
wrappers are ordinary BUILD graph targets. Repository-wide execution selects declared
wrappers or semantic consumer roots without moving dependency interpretation into Rust.

The correctness baseline applies the codegen collection aspect to `//...`. It naturally
handles top-level visibility, test-only targets, platform-incompatible target-pattern
skipping, and current repository membership, but pays recursive package discovery and
may analyze irrelevant targets.

The leading optimization candidate asks Bazel query for declared codegen candidates in
the current BUILD graph and writes those returned labels to a temporary target-pattern
file. No-argument codegen passes it to Bazel with `--target_pattern_file`, the codegen
aspect, and the codegen output group. Bazel, rather than Rust, interprets every label and
configured dependency closure. This avoids a central eager `label_list` dependency node
and preserves top-level treatment of private, test-only, and incompatible targets.
Command-length limits do not apply because Bazel reads the file.

A monolithic aggregate rule and package-local aggregate shards remain benchmark
candidates, not accepted architecture. Bazel eagerly analyzes `label_list` dependencies;
an aggregate does not make the selected graph lazy or automatically execute dependency
actions. It must explicitly propagate aspect providers and expose generated artifacts
through a requested output group. Central dependencies also introduce visibility,
test-only, platform compatibility, cycle, and high-fan-out invalidation concerns.

No candidate may reduce roots based only on an unconfigured query graph: transitions,
toolchains, and `select()` can make a consumer closure differ from a directly selected
dependency. Initial indexes list every designated semantic root and let Bazel deduplicate
identical configured targets.

All candidates operate on BUILD metadata present when the command starts. Adding an
unowned file or schema does not make setup fail and does not participate until a BUILD
target or wrapper is declared, manually or through `dx generate`. No env, codegen, or
setup command runs a Gazelle freshness preflight or attempts to infer missing metadata.

## Provider Contract

Codegen integrations use verified authoritative transitive upstream providers wherever
they identify generated artifacts and their import/source-root mapping. When required
metadata is absent, a narrow ruleset-specific adapter normalizes the upstream semantics
into the internal projection provider. Unsupported rule kinds fail closed. Collectors do
not fall back to `DefaultInfo.files` or traverse a broad union of dependency-like
attributes: that would analyze irrelevant tool/data edges and cannot reliably distinguish
generated language sources, resources, executables, or import roots.

Each projection record identifies:

- The producer target and output language/file class.
- Generated artifacts or tree artifacts.
- Their deterministic logical workspace path and import/source-root mapping.
- Required generated package or namespace semantics.
- Read-only status and any runtime dependency context needed by environment adapters.

Multiple producers claiming an incompatible language import path fail before selection.
No traversal-order winner is accepted. Code generation remains a Bazel action with
declared inputs and outputs; aspects collect providers and may adapt outputs but do not
infer late dependency edges.

A generated logical module/path that conflicts with checked-in source also fails before
selection. Root ordering never silently chooses checked-in or generated content. An
integration may permit a generated replacement only through an explicit tested adapter
contract identifying the replaced source and generated artifact; a generic user flag
cannot waive the conflict. Generated-to-generated duplicates require identical
artifacts and explicit mergeable semantics, otherwise they fail.

Each ruleset adapter defines the narrow tested edge and provider mapping needed to
preserve that ruleset's authoritative transitive semantics. Generic collectors consume
only those normalized providers rather than encoding every upstream attribute. Each
contributing configured target emits one small normalized binary Protobuf plan shard.
Its projection provider collects those shards and required generated artifacts in
transitive depsets; targets with no direct contribution forward only their transitive
collections.

One private codegen output group contains the collected shards and every generated
artifact referenced by them, ensuring generator actions execute and remote outputs are
materialized locally. Shards use a reserved controlled filename suffix. The CLI obtains
every candidate path from that output group's BEP named sets, recognizes shards by the
reserved suffix, parses them, and indexes the remaining BEP-reported artifacts by the
execution paths declared in the shards. It rejects missing, duplicate, or unreported
artifacts and never scans or guesses paths under `bazel-out`.

Because the projection is a tree of ordinary filesystem symlinks, every referenced
artifact must exist locally when selection commits. With remote cache or execution, the
Bazel request downloads the requested codegen output group rather than leaving those
artifacts remote-only. It does not request unrelated build outputs, and `dx` never
implements a second remote-cache downloader.

Rust deterministically merges shards by configured-target identity, validates logical
path and integration conflicts, and hashes the normalized complete plan. Repository
roots do not emit a second complete closure manifest, so shared closures are not
serialized once per selected root and a narrow contribution change need not invalidate
an independently generated repository-sized manifest.

## Filesystem Projection

Canonical generated artifacts remain under Bazel's output tree. Within each immutable generated-code
generation, deterministic logical workspace-relative paths mirror generated files. Each mirror leaf
is a managed symlink to its Bazel output; storage is not divided into top-level language directories.
The stable
`.dx/setups/current/generated` path exposes the selected mirror after every requested artifact and
mapping validates successfully. Shared directory layout, identities, reuse validation, selection,
empty counterparts, atomic commit, ownership, and retention are defined in
[Managed Environment State](managed-state.md).

Language integrations may configure LSPs with the complete stable mirror or with
provider-derived root directories inside it. They do not require language-specific
storage trees. There is no root `.generated` facade and no generated symlink is inserted
beside checked-in source files.

Generated paths are read-only quality context. Format, lint-fix, and other replacement
workflows never mutate them. `bazel clean` or an output-base change can make links stale;
only explicit `dx codegen` repairs the projection.

## Performance Model

The repository-wide cold run is proportional to the selected graph and must execute
uncached generator actions. A query-produced target-pattern file is expected to reduce
target-pattern discovery and unrelated loading compared with `//...`, but it still loads
packages containing indexed labels and analyzes every selected configured closure.
Bazel's persistent server and action cache are expected to improve unchanged warm runs;
the required evidence remains a milestone gate rather than a verified claim.

Exact-target runs bypass repository root selection and analyze only one configured
target closure. Generator action invalidation may be narrow after one schema change,
while validating and rebuilding a repository-wide link projection may still be linear
in all selected artifacts. `dx` maintains no independent generator cache or persistent
semantic graph.

Milestone benchmarks compare `//...`, a query-produced target-pattern file, one aggregate,
and package-local shards with identical effective target, aspect, output-group, and
configuration semantics. They measure cold and warm loading/analysis, source and BUILD
edits, target add/remove, actions, materialized bytes, projection time, and retained
memory. Among equivalent-semantics candidates the fastest wins on both cold and warm,
with warm weighted above cold for internal paths; otherwise the `//...` correctness
baseline remains. See [O34](../open-decisions.md).

## Test Requirements

Codegen tests must cover:

- Repository-wide `//...`, query-produced target-pattern file, aggregate candidates, and exact-target
  closure selection with equivalent effective roots and outputs.
- Inclusion of production, test, example, and development projections in repository-
  wide mode without performance-motivated omission.
- Bare-schema selection of every registered language projection and consumer-target
  closure selection without reverse-dependency inference.
- Empty exact projections and transitions between root, exact, and empty selections.
- Gazelle-created wrappers and current BUILD-graph query selection for every supported
  generator/language pair.
- Newly added unowned files remaining outside setup until BUILD metadata is generated,
  without a freshness failure or implicit Gazelle execution.
- Generated files, tree artifacts, import roots, namespaces, paths with spaces, and
  incompatible producer collisions.
- Per-contributor Protobuf shards, transitive depset deduplication, deterministic merge,
  reserved-suffix recognition only among BEP-reported files, and rejection of missing,
  duplicate, or unreported referenced artifacts.
- Checked-in/generated logical path conflicts, explicit adapter replacement contracts,
  and no root-order shadowing.
- Workspace-relative mirror paths, cross-language coexistence, leaf symlinks to Bazel
  outputs, and no root or source-directory overlay facade.
- Read-only generated artifacts and exclusion from formatting or replacement.
- Stale links after `bazel clean` and output-base changes without implicit refresh.
- Cold/warm, source/BUILD edit, target churn, memory, materialization, and projection
  evidence comparing every repository-root candidate.
- Scope-mismatch warnings between codegen and environment selections without implicit
  cross-command execution.
- Immediate LSP visibility after current-pointer replacement without environment
  regeneration or a pinned codegen identity.

Cross-cutting fixture, platform, remote, and evidence requirements remain in
[Testing Strategy](../testing/README.md). Shared identity, pointer, reuse, carry-forward,
concurrency, ownership, and retention requirements remain in
[Managed Environment State](managed-state.md#test-requirements).
