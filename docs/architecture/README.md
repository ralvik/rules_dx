# Architecture

Status: design only; no implementation exists yet. This document describes the planned
system shape, not built behavior.

`rules_dx` is a Bazel module and developer workflow layer that makes the Bazel graph the common
source of truth for local development, coding agents, and CI. This document summarizes the stable
system shape. Detailed contracts live in the linked domain documentation and
[decision record index](../decisions/).

## Principles

1. Bazel is the only authoritative graph and action engine.
2. The CLI is a thin, inspectable adapter over stable Bazel interfaces.
3. Non-mutating CI checks are directly executable as Bazel targets or aspect workflows.
4. Bazel analysis determines source ownership, configuration, and transitive inputs.
5. Mutating commands are explicit, and check modes expose the same proposed changes without
   writing. Generation retains Gazelle's native write and merge semantics.
6. Language rulesets and ecosystem resolvers remain authoritative for language semantics;
   `rules_dx` does not build parallel compilers, package managers, or dependency graphs.
7. Domain-specific integrations stay separate until multiple concrete implementations demonstrate
   a reusable boundary.
8. Unsupported, ambiguous, or incomplete graph facts fail closed instead of being guessed.

These constraints are grounded in the accepted records for
[Bazel-owned execution](../decisions/0001-bazel-owns-execution.md),
[target-oriented workflows](../decisions/0002-targets-not-files.md), and
[explicit mutation](../decisions/0005-mutating-operations.md).

## Component Flow

```text
developer or coding agent
        |
        v
dx CLI: discovery, normalization, diagnostics, command composition
        |
        +--> Bazel launcher --> build / test / coverage / aspect workflows
        |
        +--> Bazel query and cquery --> ownership and workflow selection
                                      |
                                      v
project-owned Starlark configuration, aspects, wrappers, and toolchains
        |
        +--> authoritative upstream language rules and providers
        +--> per-target quality and audit actions
        +--> normalized results and proposed source replacements
        +--> canonical consumer workflow targets
        |
        v
project-owned runners and apply logic
        +--> execute declared tools inside Bazel actions
        +--> validate outputs and atomically apply each accepted file
```

The CLI does not parse BUILD syntax, infer dependencies, walk the checkout to assemble source
lists, or invoke CI analyzers outside Bazel. It asks Bazel for targets and artifacts, then presents
or applies the resulting operations. First-party Gazelle extensions may inspect checked-in language
sources during explicit generation through Gazelle APIs; that does not make generation logic part
of the CLI graph model.

## Ownership Boundaries

### Repository And Packages

The repository is one `rules_dx` Bzlmod module, version, and release unit. Source packages are
capability-oriented: CLI, configuration, quality capabilities, generation, environments, language
foundations, runners, protocols, toolchains, tools, tests, examples, and documentation.

Only capability packages required by approved milestones are created. Domain-specific shared code
stays with its domain. General runner, protocol, or apply code is extracted only when there is a
concrete cross-package caller.

Consumer repositories own a conventional `//dx` package for workspace policy and canonical
environment, code-generation, and generation workflows. Quality checks remain aspects over selected
source targets rather than a parallel tree of generated workflow targets. The exact consumer
surface belongs to the [CLI](../cli/), [quality](../quality/),
[generation](../generation/), and [environment](../environments/) contracts.

### Physical Layout

Logical capability packages map to these top-level paths. Only packages required
by approved milestones are created; M00 creates only the seed subset.

```text
MODULE.bazel          # Bzlmod module, version, dependency pins
.bazelrc              # shared build/test/coverage flags (no local-only flags)
.bazelversion         # Bazelisk pin, frozen from M00 executed runs
BUILD.bazel           # root package (exports, workspace-level aliases)
config/               # public workspace policy provider (@rules_dx//config)
quality/              # aspects, QualitySourcesInfo, result protocol, runners
generation/           # first-party Gazelle extensions + manifest transport
env/ codegen/         # environment and codegen projections (//dx:env etc.)
rust/ python/ js/     # language foundations (wrappers, toolchains, providers)
dx/                   # dx CLI + shared Rust runners (src/bin/dx, src/runners)
tools/                # internal acquisition, adapters, metadata (not public)
tests/                # focused contract + fixture suites per domain
examples/             # consumer-facing minimal workspaces
private/              # internal glue with no public load labels
docs/                 # authoritative contracts (this tree)
```

M00 seed subset: `MODULE.bazel`, `.bazelrc`, `.bazelversion`, root `BUILD.bazel`,
minimal `rust/` binary/test fixture, minimal CI skeleton, and coverage-gate
support. `tools/`, `dx/`, `config/`, `quality/`, `generation/`, and `env/`
arrive with their implementing milestones (M02–M11). No `lint/` umbrella or
per-tool top-level packages: individual adapters live under `tools/` or their
owning `quality/` subpackage until a cross-package caller justifies extraction.

### Public And Private APIs

Public APIs are narrow, typed, and loaded from capability packages. They include conventional
language wrappers, workspace configuration, aspect entry points, and the provider boundaries needed
by custom source-owning rules and environment integrations. Their exact symbols and fields are
defined by the owning domain documents and accepted decisions, not by this overview.

Public capability surfaces live in their top-level domain packages. The repository's `tools/`
package is reserved for internal acquisition and adapters rather than acting as a public umbrella.

Internal tool adapters, individual tool aspects, runner protocols, generated metadata, and upstream
compatibility glue are not public APIs. Consumers needing advanced upstream behavior may depend on
and load the upstream ruleset directly; `rules_dx` does not re-export its complete surface.

The source-provider bridge carries direct, classified, main-workspace, non-generated sources. It
does not carry dependencies, tools, configuration, generated context, or transitive source closure.
Project wrappers emit that shape, and tested adapters may normalize authoritative upstream providers
into it. See [Quality Sources and Applicability](../quality/quality-sources.md).

### Upstream Authorities

Language foundations wrap pinned upstream rulesets for build, test, toolchain, provider, and IDE
semantics. Ecosystem manifests, lockfiles, package-manager outputs, and public Bazel metadata remain
authoritative for dependency and target policy. Generation translates those facts into BUILD graph
edges but does not resolve versions, install packages, or invent missing ecosystem policy.

Tool acquisition and adapter boundaries are likewise centralized rather than embedded in language
wrappers. See [Tools](../tools/) and the
[tool integration model](../decisions/0007-tool-integration-model.md).

## Activation And Laziness

Each release makes supported foundations and quality families available at
tested defaults with no language enable list. Availability alone performs no
analysis or execution: operational behavior activates only when analyzed
targets and providers exist, and an unused foundation creates no configured
target, action, environment projection, toolchain payload, runtime download, or
dependency fetch. Quality policy is independent of foundation activation, and
environment/codegen selections stay independent workflows. See
[Environments](../environments/) for projection and commit boundaries.

## Selection Model

Quality selection starts from Bazel targets, not filesystem paths: the CLI
normalizes input to target requests, analysis determines ownership and
configuration, and aspects attach work to applicable source-owning targets. Only
direct, main-workspace, non-generated sources are mutation subjects. Action
granularity stays provisional under
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

The delivery order and delivery gates are maintained in the
[implementation plan](../implementation-plan.md). Foundation constraints are recorded in
[ADR 0010](../decisions/0010-python-foundation.md),
[ADR 0013](../decisions/0013-rust-javascript-typescript-foundations.md), and
[ADR 0015](../decisions/0015-first-party-gazelle-extensions.md).

## Inspectability

Every user-visible operation has one inspectable chain from CLI request to
Bazel selection, declared actions, normalized results, and optional mutation,
without exposing unstable subprocess command lines as API. CI checks run
directly through Bazel; ownership is inspectable through query interfaces.
Command, output, and report behavior is authoritative under [CLI](../cli/);
quality evidence under [Quality](../quality/). Unresolved choices stay in
[Open Decisions](../open-decisions.md).
