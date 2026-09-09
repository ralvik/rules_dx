# ADR 0015: First-Party Gazelle Extensions

## Status

Accepted.

## Context

`rules_dx` needs consistent source ownership, target kinds, naming, dependency validation, merge
behavior, and setup across its language foundations. Existing Gazelle extensions differ in those
contracts and may require checked-in derived integration files. Exposing such differences would make
equivalent foundations behave differently and create additional freshness workflows.

Generating targets from source and authoritative ecosystem metadata is not the same as implementing
a compiler, package manager, resolver, or language build rules. Gazelle already provides the
configuration, indexing, resolution, rule-generation, merge, and BUILD-file infrastructure needed to
own generation without taking over those other systems.

## Decision

`rules_dx` owns the Gazelle extensions for the first-release Rust, Python, JavaScript, and TypeScript
foundations. They generate stable `rules_dx` wrapper kinds and conform to one tested generation
contract. Framework containers use separately named first-party adapters rather than implicit core
JavaScript or TypeScript handling.

A future foundation may reuse an upstream extension only when conformance fixtures prove that its
behavior and generated output satisfy the complete `rules_dx` contract. A material gap or output
shape that cannot conform through supported Gazelle configuration and kind mapping requires a
first-party extension. Ownership is based on conformance, not a preference to replace mature work.

Rust is the first concrete implementation. It is implemented directly against Gazelle and the
selected public Rust ruleset APIs. It does not introduce a universal language model, runtime plugin
framework, or speculative shared helper layer. Shared generation code is extracted only after a
later language implementation demonstrates concrete reuse.

First-party extensions use Gazelle's standard APIs and lifecycle. They emit desired and conservative
empty rules, declare only the attributes they own, match conservatively, preserve `# keep` and
unfamiliar user content, and leave BUILD syntax and writes to Gazelle. They do not maintain a second
BUILD parser, writer, or ownership database.

Extensions may parse checked-in sources and authoritative project metadata to generate graph edges.
Manifests, lockfiles, package-manager outputs, and public upstream Bazel metadata remain authoritative
for dependency identity, version, scope, features, platforms, target declarations, and build
semantics. Generation does not select versions, install packages, invoke an ambient package manager,
reconstruct locks, edit ecosystem policy, compile sources, or replace language build rules.

Dependency resolution is strict and target-scoped. Every recognized literal dependency reference
must resolve through language standard-library knowledge, an explicit user mapping, the local target
index, authoritative active locked metadata, or an explicit exact user ignore. Unknown, ambiguous,
missing, and wrong-scope dependencies fail generation with actionable context. Extensions never
guess or silently omit an edge, invent an unresolved marker, activate a dependency because source
mentions it, or defer a known graph error to a later build.

Literal recognized runtime loads have the same strict treatment as ordinary imports. Computed
identities are the manual boundary: users own any required Bazel dependencies and protect them with
normal merge controls. Source-level conditions do not cause generation to infer Bazel configuration
semantics. The precise resolution order, directive behavior, and diagnostics are defined in
[Common Generation Contract](../generation/common.md).

Source ownership is deterministic and language-appropriate. A source has one generated physical
owner; dependency edges belong to that owner, and test-only references do not widen production
ownership. Rust retains crate and module ownership rather than imitating file-based languages.
Framework adapters retain one physical container owner rather than creating duplicate owners for
embedded regions. Ambiguous ownership fails closed.

Generated names are deterministic and stable. Authoritative ecosystem names take precedence where
the language contract permits them; otherwise the documented language normalizer applies. Collisions
report all claimants and fail rather than overwriting, dropping targets, or inventing numeric,
language, hash, or filename-dependent suffixes.

Merge and lifecycle follow Gazelle. Removed or renamed inputs produce conservative stale-rule
handling; user-owned content and kept values survive, and existing BUILD files are never deleted
automatically. Extensions do not strip unfamiliar fields merely to make a stale generated rule
disappear.

Test and production resources are user-owned. Extensions do not infer, validate, remove, or relocate
resource entries from source calls, path literals, manifests, generated producers, framework
conventions, or directory names. Resource attributes remain explicit upstream-rule content preserved
through normal Gazelle merge behavior.

An importable source with a recognized executable entry has one reusable source owner and a thin
binary that depends on it. The binary carries launcher or entry metadata rather than owning the source
or repeating source-derived dependencies. If public upstream rules cannot preserve runtime semantics
with that shape, support remains blocked instead of duplicating ownership, producing a binary-only
owner, or requiring source rearrangement.

Normal consumer source trees contain no generated Gazelle integration sidecars, import maps,
dependency indexes, provider inventories, or helper manifests. Legacy sidecars are inert. Required
derived metadata is a versioned declared Bazel artifact outside the source tree and an input to the
canonical generation workflow. This boundary does not prohibit Bazel outputs or explicitly managed
developer projections.

The complete common ownership, naming, resolution, merge, resource, executable-entry, and no-sidecar
contracts live in [Common Generation Contract](../generation/common.md). Language-specific behavior
lives in [Rust Generation](../generation/rust.md),
[Python Generation](../generation/python.md),
[JavaScript and TypeScript Generation](../generation/javascript-typescript.md), and
[Framework Adapter Generation](../generation/framework-adapters.md). Those documents may state
current detailed behavior without turning provisional implementation APIs into accepted ADR surface.

Exact wrapper/provider mappings, parser recognizers, transport schemas, and upstream integration
choices remain in [Open Decisions](../open-decisions.md), including O13, O22, O24, O25, O27,
O29, and O40-O42.
This ADR does not resolve those choices. In particular, Rust's first implementation must prove its
exact public mappings before they become supported API or justify abstraction.

## Consequences

- Consumers get one project-owned generation contract across first-release language foundations
  instead of adapting to unrelated extension conventions.
- Rust establishes the implementation pattern through concrete fixtures, while later languages are
  free to reveal the smallest genuinely shared layer.
- `rules_dx` owns parser, Gazelle-API, merge, golden-output, and authoritative-metadata compatibility
  tests for its first-party extensions.
- Package managers and upstream rulesets remain semantic authorities; first-party generation does
  not create a parallel resolver or build engine.
- Strict resolution catches incomplete or inadmissible dependency graphs before generation reports
  success and requires explicit, auditable exceptions.
- Single ownership, fail-closed naming, conservative merge, manual resources, and thin executable
  entries produce predictable BUILD diffs at the cost of rejecting unsupported or ambiguous shapes.
- Consumers maintain source, authoritative ecosystem metadata, directives, and BUILD policy, not
  derived integration sidecars.
- A foundation or adapter remains planned rather than supported until its public upstream mappings
  and complete conformance fixtures satisfy these boundaries.

## Rejected Alternatives

- Requiring checked-in generated language-integration sidecars.
- Reusing an upstream extension solely because it exists despite incompatible behavior.
- Reimplementing package resolution, compiler semantics, or language build rules in Gazelle.
- Silently accepting unknown dependencies or generating broad ignore exceptions.
- Duplicating source ownership to simplify tests, executable entries, or framework regions.
- Inferring resources from source or directory conventions.
- Building a generic language schema, parser framework, or plugin runtime before concrete reuse
  exists.
