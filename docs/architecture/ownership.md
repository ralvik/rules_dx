# Ownership

Capability packages, layout, and public boundaries.
The [Architecture index](README.md) stays short; this document owns the ownership record.

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
cc/ go/ java/ javascript/ typescript/ python/ rust/ kotlin/ scala/ csharp/ fsharp/ astro/ mdx/ svelte/ vue/ ruby/ powershell/  # language and file-family foundations (wrappers, toolchains, providers)
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

