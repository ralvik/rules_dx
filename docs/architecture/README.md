# Architecture

Status: implemented for the quality-core plus Rust, Python, JavaScript, and TypeScript foundations on their delivered verification layers only, excluding Docs and Env/codegen which stay `Open` for every language. See the [support matrix](../product/support-matrix.md#unqualified-platforms) for qualified hosts and the [verification matrix](../testing/verification-matrix.md#layers) for layer status.
Implementation lives in `cli/cli/src/main.rs`, `quality/result.proto`,
`generation/result.proto`, and `dx/BUILD.bazel:24-33`; shipped surfaces are
tracked in [support matrix](../product/support-matrix.md).

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

Only capability packages with a concrete consumer are created. Domain-specific shared code
stays with its domain. General runner, protocol, or apply code is extracted only when there is a
concrete cross-package caller.

Consumer repositories own a conventional `//dx` package for workspace policy and canonical
environment, code-generation, and generation workflows. Quality checks remain aspects over selected
source targets rather than a parallel tree of generated workflow targets. The exact consumer
surface belongs to the [CLI](../cli/), [quality](../quality/),
[generation](../generation/), and [environment](../environments/) contracts.

### Physical Layout

Logical capability packages map to these top-level paths. Only packages with a concrete
consumer are created; the repository started from a seed subset.

```text
MODULE.bazel          # Bzlmod module, version, dependency pins
.bazelrc              # shared build/test/coverage flags (no local-only flags)
.bazelversion         # Bazelisk pin
BUILD.bazel           # root package (exports, workspace-level aliases)
config/               # public workspace policy provider (@rules_dx//config)
quality/              # aspects, QualitySourcesInfo, result protocol, runners
generation/           # first-party Gazelle extensions + manifest transport
env/                  # environment projections (//dx:env etc.; codegen collector lives in generation/ plus cli/codegen behind //dx:codegen)
cc/ go/ java/ javascript/ typescript/ python/ rust/ kotlin/ scala/ csharp/ fsharp/ astro/ mdx/ svelte/ vue/  # language and file-family foundations (wrappers, toolchains, providers)
cli/                  # dx CLI Rust implementation (libs visible to //cli only; tools //cli/cli:dx, //cli/env:env)
dx/                   # consumer facade, Starlark-only (//dx:generate, //dx:env, //dx:codegen, //dx:config)
tools/                # internal acquisition, adapters, metadata (not public)
libs/                 # shared internal Starlark and test libraries (not public)
gazelle/              # first-party Gazelle extension hosts per language
deploy/               # deploy provider surface
third_party/          # upstream pins and vendored metadata
examples/             # consumer-facing minimal workspaces
docs/                 # authoritative contracts (this tree)
```

Seed subset: `MODULE.bazel`, `.bazelrc`, `.bazelversion`, root `BUILD.bazel`,
minimal `rust/` binary/test fixture, minimal CI skeleton, and coverage-gate
support. `tools/`, `dx/`, `config/`, `quality/`, `generation/`, and `env/`
arrived as their consumers landed. No `lint/` umbrella or
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

### Facade Twins (Post-Reorg Mapping)

`cli/` holds the Rust implementation (34 crates: 27 under `cli/` plus 5
under `quality/` plus 2 under `generation/`, `dx_*` crate names stable) and
`dx/` is the Starlark-only consumer-policy facade (`//dx:config`, `//dx:env`,
`//dx:codegen`, `//dx:generate`), landed as the facade reorganization under issue #76 with visibility
decisions recorded alongside. Only `//dx:generate` and `//dx:env` are
executable workflows (Gazelle binary, installer binary); `//dx:codegen` and
`//dx:config` are Accepted empty reservations (CLI selection identity,
workspace-flag default) with a qualification guard, not executable targets. The move
fixed the two true collisions (`dx/qual` → `cli/qualification`,
`dx/docs` → `cli/docgen`); the facade labels below are unchanged.
The reorganization is complete with no successor: `dx/` Rust to `cli/`, documentation IR to
`docs/ir`, and the mixed/hello fixture to `examples/mixed` all landed under issue #76
(recorded in ADR 0004); Rust library extraction decided internal-only under
issue #469 (see [ADR 0023](../decisions/0023-rust-libraries-internal.md)).
The e2e suite naming resolved
hermetically under issue #466: no `e2e/` tree, CLI-contract pins run under
`bazel test //...` (see the [verification matrix](../testing/verification-matrix.md)).

| Facade label | Actual owner | Status |
| --- | --- | --- |
| `//dx:generate` / `//dx:generate_check` | Composed multi-language Gazelle wiring (`//gazelle/dispatch:gazelle` with every first-party extension plus the dispatch witness last, `mode=diff` only on the check twin, `dx/BUILD.bazel`). See [`dx generate`](../cli/commands/generate.md). | Accepted (repo-wide promise in [scope](../product/scope.md) delivered) |
| `//dx:env` | Alias to `//cli/env:env` installer binary (`dx/BUILD.bazel`) | Accepted |
| `//dx:codegen` | Empty filegroup reserving the CLI selection identity; real plan collector is the `dx_codegen_plan_aspect` plus `dx_codegen_plans` output group (frozen), effective roots on the frozen `//...` baseline (closed #506; successors closed #787 and #788) | Accepted (issue #423; guard `//tools/ci:dx_facade_qualification`) |
| `//dx:config` | Empty filegroup default for the `//config:workspace` label flag, failing fast until a consumer binds its typed workspace policy; typed per-family sections frozen in `//quality:policy.bzl` (issue #916) and `//quality:sources.bzl` (issue #916) | Accepted (issue #423; guard `//tools/ci:dx_facade_qualification`) |
| `//tools/coverage:coverage_gate` | Single crate after the unused `:coverage` wrapper removal | Accepted |
| `real_source_target(name="corpus_*")` splits | Per content type per package, Gazelle-owned via `dx generate`, shared `tags = ["corpus"]` (issue #15) | Accepted |

### Rust Library Boundary (Issue #469)

Rust libraries stay internal with a binaries-only public boundary: only
`//cli/cli:dx` and `//cli/env:env` are public tool entry points, and
consumers use those binaries plus the `//dx` Starlark facade, never the
`cli/*`, `quality/*`, or `generation/*` libraries directly. `libs/` is
Starlark-only with no Rust crates. The closed 34-crate inventory, boundary,
and wont-extract rationale live in
[ADR 0023](../decisions/0023-rust-libraries-internal.md); guard
`//tools/ci:rust_library_qualification` pins them.

### Upstream Authorities

Language foundations wrap pinned upstream rulesets for build, test, toolchain, provider, and IDE
semantics. Wrapper macros use bare conventional names (`python_library`, not `dx_py_library`)
loaded from `//<language>/rules:defs.bzl`; the load label is the namespace, per
[ADR 0004](../decisions/0004-naming.md). A file that also needs the same-named upstream symbol
aliases the upstream load (`_cc_library = "cc_library"`), never the wrapper. Ecosystem manifests, lockfiles, package-manager outputs, and public Bazel metadata remain
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
