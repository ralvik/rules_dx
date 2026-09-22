# Lifecycle

Activation, selection, execution, foundation sequence, and inspectability.
The [Architecture index](README.md) stays short; this document owns the lifecycle record.

## Activation And Laziness

Each release makes supported foundations and quality families available at
tested defaults with no language enable list. Availability alone performs no
analysis or execution: operational behavior activates only when analyzed
targets and providers exist, and an unused foundation creates no configured
target, action, environment projection, toolchain payload, runtime download, or
application payload fetch. Quality policy is independent of foundation activation, and
environment/codegen selections stay independent workflows. See
[Environments](../environments/) for projection and commit boundaries.

Accepted fit (2026-09-21, #959): Bzlmod version resolution plus extension Starlark
that reads checked-in locks and manifests is version-resolution cost, not
payload acquisition. `MODULE.bazel` extension declarations and
`MODULE.bazel.lock` size are that cost; Maven, npm, uv, and crate payloads
download only when analyzed targets need their repositories. Live-manifest
fetches must use fixed-manifest plus package-index inputs, and deferred EULA
failure never proves laziness: see
[ADR 0014](../decisions/0014-tested-platform-release-stack.md#decision) and the
[Windows acquisition fit](../native-toolchains.md#windows-acquisition-and-compatibility).
The consumer one-dependency surface is distinct from the internal module graph,
and `.bazelrc` stays single-sourced through the vendored preset plus collision
detector: see [Local Workflows](../contributing/local-workflows.md#preset-update-loop).

## Selection Model

Quality selection starts from Bazel targets, not filesystem paths: the CLI
normalizes input to target requests, analysis determines ownership and
configuration, and aspects attach work to applicable source-owning targets. Only
direct, main-workspace, non-generated sources are mutation subjects. Action
granularity is accepted under
[ADR 0003](../decisions/0003-action-granularity.md). See
[Target Resolution](../cli/target-resolution.md) and [Quality](../quality/).

## Execution Boundaries

Bazel actions analyze declared inputs and emit versioned results plus optional
candidate replacements without mutating the checkout; direct Bazel CI can fail
on those results without the CLI. Mutating workflows validate materialized
results and apply one proposed replacement per accepted file; conflicting
candidates reject only their path. Generation is a separate Gazelle-owned
mutation boundary; environments and codegen projections are explicit managed
mutations outside ordinary build/test/lint/typecheck actions. See
[Quality](../quality/), [Generation](../generation/), and [Environments](../environments/).

## Foundation Sequence

The repository first establishes the shared quality core and exercises it directly through Bazel on
its own Rust code. This dogfood path validates source ownership, configuration, tool acquisition,
result protocols, runners, and mutation safety before application-language expansion.

Rust is the first application foundation. Its build, test, toolchain, generation, and environment
integration follows the quality-core dogfood rather than preceding it. The first-party Rust Gazelle
extension is the first concrete implementation of the accepted generation contract.

Python and JavaScript/TypeScript follow the Rust foundation. The Rust extension does not begin with
a universal language abstraction: shared Gazelle mechanics are extracted only when later language
implementations demonstrate real reuse. This sequence preserves the accepted foundation decisions
while avoiding speculative cross-language APIs.

The delivery order is tracked in GitHub issues and
delivered work under closed #470-#505. Foundation constraints are recorded in
[ADR 0010](../decisions/0010-python-foundation.md),
[ADR 0013](../decisions/0013-rust-javascript-typescript-foundations.md), and
[ADR 0015](../decisions/0015-first-party-gazelle-extensions.md).

## Inspectability

Every user-visible operation has one inspectable chain from CLI request to
Bazel selection, declared actions, normalized results, and optional mutation,
without exposing unstable subprocess command lines as API. CI checks run
directly through Bazel; ownership is inspectable through query interfaces.
Command, output, and report behavior is authoritative under [CLI](../cli/);
quality evidence under [Quality](../quality/). Known design conflicts and
their last-agreed resolutions live in the [contradiction catalog](contradiction-catalog.md).
Open work lives in GitHub issues and delivered work under closed #470-#512.
