# ADR 0011: Configuration Composition

## Status

Accepted.

## Context

Lint, typecheck, format, audit, language, generation, and environment integrations need
different configuration types and validation boundaries. One generic settings rule would couple
unrelated capabilities and allow one integration to interpret another's policy. Requiring every
consumer to assemble those pieces manually would undermine the setup-oriented product.

Quality tools also have native configuration whose scope is local to source ownership rather than
global workspace selection. Bazel actions need explicit, reviewable inputs, while editors and direct
tool invocations need the same checked-in policy. Ambient filesystem discovery and a workspace-wide
registry cannot satisfy both requirements without broad inputs or a second source of truth.

Aspects implemented in the external `rules_dx` module cannot safely hardcode a main-repository label.
Custom rules and third-party environment integrations also need narrow public composition boundaries
without exposing private adapter or managed-environment implementation.

## Decision

Configuration remains modular and typed. Each capability or language package owns its policy type,
validation, and provider. Implementations consume only their own typed section and do not interpret
settings owned by another capability.

Those sections compose into one canonical workspace policy in the consumer's `//dx` package. The
normal setup surface supplies opinionated defaults; advanced composition may replace typed sections
without replacing the canonical policy target. External aspects select that policy through an
explicit label setting rather than embedding a consumer-repository label.

The exact aggregate provider constructor, fields, exports, and label-setting binding remain governed
by [open decision O17](../open-decisions.md). This ADR accepts typed composition and one canonical
workspace policy, not a provisional Starlark API shape.

Supported application foundations and quality policy families are available by default but remain
lazy. Declared target providers and semantic source classes activate applicable behavior. Typed
language policy changes language-owned choices; it is not an enable switch. Typed quality-family
policy selects tools for that family's classes, while ruleset-owned adapter metadata determines
capability and class support. No use means no action or tool payload fetch.

Checked-in native tool files are the sole user-authored behavioral policy for quality tools. In the
absence of an applicable file, the pinned tool's upstream defaults apply, except for the approved
config-required Vale integration: upstream supplies no default configuration, so an applicable
selected adapter fails with actionable guidance rather than synthesizing policy or silently skipping.
The [native-config contract](../quality/native-configuration.md#authority) owns the exception and
its qualification boundary. `rules_dx` does not add a second behavioral preset, global exclusion layer,
or ambient config search. Native config inputs and
their closures must be declared source artifacts, not generated policy.

Explicit generation is the native-config discovery boundary. It creates package-local typed config
targets and binds the locally relevant targets directly to source owners through ordinary Bazel graph
edges. Native config labels are not stored in aggregate workspace policy, and there is no global or
package-level config registry. Quality analysis and execution consume the generated binding and do
not scan the workspace or require a generation freshness check.

The generated local binding preserves these architectural boundaries:

- A recognized config has a predictable local typed target and explicit ownership.
- Config directories may establish Bazel package boundaries.
- Source owners receive direct local config hints rather than an intermediate aggregator.
- Gazelle owns only its generated rules and hint entries and otherwise uses conservative standard
  merge and lifecycle behavior.
- Each action receives only applicable configs and their declared closures; native per-file
  resolution remains the tool's responsibility where that behavior is proven hermetic.
- Config targets are ordinary source owners for compatible quality tools, and the resulting policy
  graph must remain acyclic.

The authoritative filename, naming, visibility, closure, merge, lifecycle, exclusion, isolation, and
mutation behavior is defined in
[Native Configuration](../quality/native-configuration.md). Quality actions follow the explicit-input
and pipeline boundaries in [Quality Action Model](../quality/action-model.md).

`QualitySourcesInfo` is the public concept by which custom rules expose directly owned repository
sources classified by stable semantic file classes. It is source ownership and classification only:
it does not select capabilities, tools, native configs, or action granularity. Built-in wrappers and
supported adapters publish equivalent canonical source facts.

The provider's exact constructor, load label, field representation, class registry, and validation
surface remain governed by [open decision O15](../open-decisions.md). The durable contract and current
candidate details live in
[Quality Sources and Applicability](../quality/quality-sources.md).

Quality applicability is the intersection of declared direct-source classes, adapter support,
workspace policy, and capability. Native configuration can narrow tool behavior within that declared
set but cannot add undeclared sources. The chosen action granularity and stage ordering remain subject
to their own evidence and are documented in
[Quality Action Model](../quality/action-model.md), not fixed by this ADR.

There is one public PATH-tool contribution concept shared by first-party and third-party
integrations. A contribution carries validated logical command names and Bazel executable metadata;
typed configuration composes contributions, and the final environment rejects collisions. Built-in
quality contributions use the same executable and runtime selection as their Bazel adapters rather
than a second editor installation.

This public concept covers reusable PATH tools only. Persistent language environments remain
language-specific until concrete implementations demonstrate a shared abstraction. The public
`EnvironmentInfo` and `environment_tool` surface and managed installation, symlink projection,
selection, ownership, and mutation behavior are defined in
[Developer Environments](../environments/environment.md#public-tool-api) and
[Managed Environment State](../environments/managed-state.md).

Generation, environment, codegen, and quality workflows operate on the currently declared Bazel
graph. Generation may create and maintain ordinary BUILD targets, but the other workflows neither
discover undeclared sources nor implicitly run Gazelle. The common generation boundary is documented
in [Common Generation Contract](../generation/common.md), with language detail in
[Rust](../generation/rust.md), [Python](../generation/python.md),
[JavaScript and TypeScript](../generation/javascript-typescript.md), and
[Framework Adapters](../generation/framework-adapters.md).

## Consequences

- Most repositories can use one concise setup declaration while retaining typed, independently
  validated policy underneath it.
- The canonical workspace policy gives external aspects a stable selection point without hardcoded
  main-repository labels.
- Native tool policy remains directly usable by Bazel, editors, language servers, and upstream tools;
  generated BUILD metadata only binds that policy into the graph.
- Local config edges keep action inputs and invalidation scoped, at the cost of explicit generation
  after config files are added, moved, or removed.
- Generated config package boundaries and ordinary Gazelle lifecycle can change source ownership and
  leave empty BUILD files for users to remove deliberately.
- Custom rules can participate in quality workflows without depending on private wrappers, while
  source classification remains separate from tool selection and execution policy.
- First-party and third-party PATH tools compose through one validated public concept; persistent
  environments do not acquire a premature universal API.
- Exact provider and constructor APIs are not implied by this accepted record and must be resolved in
  the linked open decisions before publication.

## Rejected Alternatives

- One untyped rule containing every capability's settings.
- Mandatory setup boilerplate for every capability.
- Hardcoding the consumer's canonical policy label inside an external aspect.
- Discovering native configuration during analysis or execution by scanning ambient files.
- A global native-config registry or supplying every config to every action.
- Generated native behavioral policy or a second `rules_dx` exclusion layer.
- A private first-party-only PATH contribution mechanism.
- A generic persistent-language-environment API before multiple implementations prove reuse.
