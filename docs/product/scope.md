# Product Scope

## Repository Audit

At design start the repository was an initial scaffold. The only tracked files then were
`.gitignore` and `AGENTS.md`. The table below records that starting point; it is historical
and does not describe the current checkout, which now holds this design, `docs/`, and examples.

| Area | Evidence | Design effect |
| --- | --- | --- |
| Bazel | `AGENTS.md` says the repository uses Bazel; no `MODULE.bazel`, lockfile, BUILD files, `.bazelrc`, or version pin exists | Bootstrap choices remain open; no existing target model can be migrated or validated yet |
| Custom rules | No Starlark, rules, macros, providers, aspects, toolchains, or generators exist | Every public Starlark API in this design is proposed |
| Languages | No source language or language configuration exists | Rust is selected for `dx` and shared runners; Starlark declares Bazel actions |
| Developer tools | No formatter, linter, type checker, test framework, coverage tool, or Gazelle setup exists | The frozen tool list is a minimum inventory; first-release admission below governs feasible expansion |
| Wrappers | No scripts, CLI, task runner, Bazel wrapper, or Bazelisk launcher exists | `dx` has no compatibility obligation; the accepted launcher policy uses `.bazelversion` with Bazelisk |
| CI and release | No CI, tags, releases, version source, or packaging exists | CI and distribution cannot be assumed by early milestones |
| Documentation | At design start no README, `docs/`, ADR convention, or examples existed | `docs/` establishes the first architecture convention |
| Local configuration | `.gitignore` excludes Bazel outputs and `user.bazelrc` | Shared behavior must not rely on untracked local flags |

`AGENTS.md` requires Bazel, warnings as errors, focused tests and documentation,
formatter and linter execution after code changes, no direct edits to generated
files, concise README files, and substantial examples under `examples/`. There is
currently no command capable of satisfying its formatter and linter requirement.

No existing documentation was overwritten. The repository has no established
architecture location, so the design lives under `docs/`. The requested
`decisions/` name is retained rather than introducing a competing `adr/`
convention.

## Product Boundary

`dx` is a transparent user interface over Bazel. It may discover a workspace,
normalize user input, ask Bazel for graph facts, select known workflow targets,
compose a small number of Bazel invocations, and report diagnostics. It must not
own source or dependency graph semantics.

Bazel remains authoritative for dependencies, source ownership, actions, tests,
non-mutating CI checks, type checking, coverage, caching, sandboxing, and remote
execution.

Lint, typecheck, format, and source-audit integrations are implemented by project-
owned aspects and Rust runners. The project does not depend
on or promise API compatibility with `aspect_rules_lint`; its frozen v2.8.0 supported-tool list,
plus explicitly documented curated additions, is the minimum first-release tool baseline, not a
ceiling on scope. See
[First-Release Tool Baseline](../tools/tool-baseline.md).

First-class language build/test foundations implement Rust first. Python and JavaScript/TypeScript
independently follow the Rust dogfood closure, with broader foundations promoted only after their
support-matrix evidence passes. Every release makes supported foundations automatically available at one tested default
stack while retaining strict unused-foundation laziness. This scope is distinct from tool parity
and delegates language semantics to selected upstream Bazel rules.

## First-Release Admission

V1 requires complete Rust, Python, JavaScript, and TypeScript foundations, plus the already-required
Vue, Svelte, Astro, and MDX integrations, on every
[required platform](../decisions/0014-tested-platform-release-stack.md#required-platforms).
Build, test, dependencies, generation, quality, coverage, environments, and IDE integration must work
together for the documented scope. An incomplete required foundation cannot be replaced by a
build-only or quality-only support claim. Rust-first dogfooding remains the delivery order.

The out-of-the-box contract is one `rules_dx` dependency with tested defaults and no language enable
lists or separately installed language runtimes and quality tools. Bazel acquires declared pinned
dependencies; application manifests and locks remain authoritative. `dx generate` creates supported
targets and `dx setup` prepares matching development environments. Unused foundations remain lazy.
This does not eliminate the documented Bazel/bootstrap prerequisites or invent application policy.

Windows native toolchains retain hermetic acquisition and MSVC compatibility under
[ADR 0014](../decisions/0014-tested-platform-release-stack.md#decision). No separately installed
Build Tools or host SDK exception is approved.

Additional complete foundations belong in v1 when existing upstream rules permit low-cost hermetic
integration. When completion requires substantial missing infrastructure, an additional foundation
may be explicitly deferred beyond v1 with evidence and a recorded decision rather than delaying the core
indefinitely. This exception does not apply to the required core/framework set or reduce the broad
quality-tool baseline. A language's foundation deferral is not permission to omit its required tools.

The [support matrix](support-matrix.md) is the minimum candidate inventory. Its
[minimal core freeze](support-matrix.md#minimal-required-core-freeze-o46-pre-m00) records the
pre-M00 required set, provisional upstreams, owning decisions, delivery milestones, and roles.
Under [O46](../open-decisions.md), review the wider upstream ecosystem and record each named capability's
rules/tools, acquisition route, public providers, dependency authority, applicable platforms,
integration effort, contract compatibility, evidence, and delivery owner. Apply the same review to
existing feature exclusions, additional test runners, framework adapters, plugins, audit/update
ecosystems, and codegen pairs. Apply the foundation admission rule above separately from tool and
workflow obligations; an unreviewed entry is not an exclusion. Freeze a reviewed release inventory
rather than claim support for an unbounded set of
unnamed tools. Newly identified candidates require an explicit disposition, not automatic deferral.

Each candidate must become required v1 scope, a recorded additional-foundation deferral, or an
evidence-backed recorded exclusion. No individual foundation is deferred by this policy alone.
Record missing upstream rules, non-hermetic acquisition, unsupported platform requirements, or
substantial integration effort precisely, including concrete gaps and ongoing maintenance ownership.
Scheduling preference and omission from the original plan are not feasibility failures.
Unresolved required cells block qualification; do not silently downgrade
them to optional or post-v1. Existing platform, laziness, generation, and public API contracts remain
in force. If a viable upstream integration conflicts with one, stop that slice and present the
evidence, alternatives, compatibility impact, and blocked work for a contract decision.

### Language Integration Maintenance

Prefer existing upstream rules and ready-made artifacts. Language integration may include narrow
wrappers, provider/Gazelle adapters, pinned artifact packaging, assembled upstream runtime closures,
and reproducible release-CI builds from pinned upstream source when necessary for missing platforms.
Consumers acquire the resulting qualified artifacts, not a requirement to build their development
tool stack locally. Retain the [acquisition and provenance requirements](../tools/tool-acquisition.md).

Focused, tested, pinned ruleset patches are generally allowed whenever they make the
integration better or easier; prefer upstreaming them and track their maintenance until an
upstream release replaces them. For example, first reproduce a Python Windows gap, then patch
the narrow failing path and prove the complete Windows workflow. A Windows gap is not yet
established here, and no particular patch is approved by this example.

If remediation is substantial, compare established upstream alternatives before implementing
replacement rules. Keep the current upstream choices unless an explicit contract/API review approves
a switch. A small missing integration may be project-owned with a clear boundary, tests, and
maintenance owner. Do not rewrite an entire language ruleset or replace compilers, interpreters,
dependency resolvers, runtimes, or framework engines. Reproducibly building upstream source is not
reimplementing its semantics.

O46 must still establish candidate-specific effort, patch/build provenance, upgrade strategy, and
delivery ownership. Exact upstream switches and new public APIs remain unapproved until their
domain contracts and work packages are accepted. General permission for these maintenance routes
does not prove any candidate feasible or authorize an unbounded fork.

The project also requires [100% first-party implementation coverage](../testing/README.md#coverage).
That gate is separate from consumer language-coverage support and feature-matrix completeness.

## Unused Dependencies

Every supported language foundation must participate in two distinct, non-mutating Bazel-owned
dependency checks/tests:

- Lockfile consistency: verify that the authoritative lockfile is up to date with current manifest
  requirements and resolver configuration, including missing or stale entries under upstream
  semantics. This is not a requirement to select newly published versions; upgrading valid locked
  versions remains the separate `dx update` workflow.
- Declared-dependency usage: verify that declared ecosystem dependencies are actually used in their
  owning project/workspace scope. An unused declaration is an error even when the lockfile is
  consistent with that declaration. Do not require every transitive locked package to be directly
  referenced by first-party source.

Failures are errors, not informational findings or warnings. The tests do not rewrite manifests or
lockfiles. Assess usage across the declaration's owning scope, not just one target that shares it.
Use qualified upstream language tooling and authoritative dependency metadata, not a second graph
or a universal source-import heuristic in the CLI. Unused foundations remain lazy; this requirement
does not introduce a language enable list.

Lockfile-consistency test execution must work without network access once its declared tools and
inputs are available. Validate the existing resolution without consulting live registries for newer
releases or relying on an undeclared package-manager cache. Any required resolver metadata must be
supplied as declared inputs through the qualified integration. Missing required inputs fail with an
actionable error; an unsupported offline validation route blocks qualification, not a skipped or
passing test. Normal Bazel acquisition of declared inputs remains distinct from test execution.

Usage is assessed across the project's declared supported configurations, not only the host or
active default configuration. A dependency used by a supported platform or optional feature counts
as used without an exception merely for being inactive in the current run. Declaration as optional
or platform-specific is not itself proof of usage. Qualify the authoritative configuration inputs
and upstream analysis for each language under O46; this does not require executing every target
platform's binaries on the checking host or weakening selected-target build isolation.

The usage test also reports incorrect declaration categories as errors where the ecosystem
distinguishes production, test/development, and build dependencies. For example, usage only by tests
does not justify a production declaration. Evaluate categories across the declared supported
configurations using qualified upstream semantics, not one universal category model. Legitimate
multi-category usage must remain valid. Tests report miscategorization without moving declarations
or rewriting locks; exact ecosystem category and checker mappings remain under O46.

Generate both checks as normal Bazel test targets for the applicable dependency-owning scopes.
They participate in `bazel test //...` and bare `dx test` by default, without a separate opt-in or
default `manual` exclusion. They remain independently runnable through their generated labels.
Exact naming, rules/providers, and generation mappings require qualification under O46.

Declared-dependency usage permits narrow, explicit exceptions for legitimate uses the checker
cannot recognize, such as dynamic plugins or tools invoked by scripts. Prefer upstream-native
checker configuration and require an explanatory reason for each excepted dependency in its owning
scope. An exception does not disable the usage test, suppress unrelated unused dependencies, or
waive lockfile consistency. Obsolete exceptions are errors: an exception naming a removed dependency
or no longer suppressing a finding in the owning scope must be removed, including when upstream
improvements recognize the legitimate usage. Assess this against the same declared supported
configurations as the usage test, not only the active host configuration. Validation reports the
obsolete exception without deleting it automatically. Exact native configuration, reason validation,
and obsolete-exception detection mappings require
qualification under O46; no separate exception registry or public API is selected here.

Gazelle automatically maintains generated Bazel dependency edges under the
[generation and merge contract](../generation/common.md#merge-and-lifecycle). Do not add a separate
unused-Bazel-dependency test duplicating that responsibility. Normal `# keep` and handwritten-content
protections remain; this does not authorize pruning user-owned edges. Existing generation conformance
tests remain required.

The two-test requirement and explained usage exceptions are accepted, but ecosystem-specific
lock/usage scopes, non-import recognition, native exception/reason-validation mappings, and exact
public test/capability API remain
open under [O46](../open-decisions.md). Qualify each supported language's mapping and focused
failing/passing consumer fixtures before implementation. Existing strict generation alone is not
proof of lockfile consistency or declared-dependency usage. This is dependency hygiene, not an
expansion of `dx audit`.

## Automatic Workflows

Prefer automatic behavior over extra explicit user steps when the selected upstream tools support
it and it is the simplest conforming integration. This preference applies across languages and
workflows, not only editors. Do not build a custom watcher, resolver, or refresh engine merely to
make a workflow automatic. Use an explicit workflow when upstream automation is unavailable or
cannot preserve the required semantics without greater integration complexity; document the concrete
limitation rather than treating manual refresh as inherently preferable.

Automation preserves Bazel ownership, authoritative manifests and locks, hermetic declared inputs,
unused-foundation laziness, selected-target isolation, and mutation/ownership safety. It does not
accept licenses on the user's behalf, choose new application dependency versions, or authorize
destructive cleanup. User-facing automatic triggers and their possible build/download work must be
documented and tested. Explicit commands remain useful for recovery and controlled invocation even
when routine refresh is automatic. The accepted exception is the thin local-only
`dx watch` loop defined by [ADR 0017](../decisions/0017-dx-watch.md), which reuses
existing command resolution and Bazel execution without a daemon, cache, or graph.

Review existing explicit-only workflows against this preference as their upstream mappings are
qualified; update their owning contracts and tests together before changing behavior. This policy
does not silently compose existing CLI commands, change environment selections, or introduce new
public APIs. The approved Rust and Go editor behavior is defined in
[Ownership And Refresh](../environments/environment.md#ownership-and-refresh).

## Initial Command Evaluation

| Command | Initial status | Reason |
| --- | --- | --- |
| `dx audit` | retain | Run `security` and `license` families (default both); not a general quality umbrella |
| `dx lint` | retain, mutating by default | Run linters, apply available fixes, and fail on remaining findings; `--check` is non-mutating |
| `dx typecheck` | retain, mutating by default | Run type checkers and atomically apply genuine fixes when available; `--check` is non-mutating |
| `dx test` | retain | Thin mapping to Bazel-owned tests |
| `dx format` | retain, mutating by default | Format selected sources; `--check` is non-mutating and separate from lint |
| `dx build` | retain | Thin mapping to `bazel build` |
| `dx update` | retain | Update dependencies through their authoritative resolvers and Bazel integration |
| `dx generate` | retain, mutating by default | Default repository-wide Gazelle discovery with explicit v1 path/label/pattern scope; supported sources can create initial targets without manifests or prior targets, and `--check` validates freshness without writes |
| `dx codegen` | retain, mutating | Build and select repository-wide or exact-target generated-source projections for LSPs |
| `dx env` | retain, mutating | Prepare repository defaults or exact per-language projections for one target from Bazel providers |
| `dx setup` | retain, mutating | Atomically prepare and select matching codegen and environment projections without running Gazelle |
| `dx coverage` | retain | Run Bazel coverage and expose its outputs |
| `dx check` | retain, non-mutating | Sequential `format`, `lint`, `typecheck`, then `generate` freshness in `--check` mode; see [ADR 0018](../decisions/0018-umbrella-check-fix-cleanup-clean.md) |
| `dx fix` | retain, mutating by default | Same sequence in default mutating mode with per-file atomic apply; no post-apply rerun |
| `dx clean` | retain, mutating managed state only | Prune validated unselected `.dx` generations; explicit `--bazel` also forwards `bazel clean` |
| `dx bazel` | retain | Exact-forwarding escape hatch through the selected repository launcher |
| `dx docs` | retain, provisional contract | Build, check, and serve the unified documentation site; `--check` is non-mutating |

`dx doctor` and `dx configure` are not commands. `dx check` and `dx fix`
are thin sequential umbrellas, not a general CI scheduler. `generate`
is preferred to `configure` because Gazelle generates repository metadata rather
than configuring developer preferences.

Normal CLI operation prints concise workflow summaries without subprocess argv or
forwarded option values. `--quiet` suppresses wrapper planning output while preserving
Bazel/tool diagnostics. `--dry-run` exposes safe structured operation summaries; machine
output follows the complete [Output Protocol](../cli/output-protocol.md).

## Constraints

The accepted constraints are recorded in [Decision Records](../decisions/). In
particular, the CLI does not parse BUILD files, scan recursively to construct
lint inputs, execute Ruff or Ty directly for CI checks, or introduce independent
resolution, caching, lockfile, action graph, BUILD syntax, or plugin mechanisms.
Repository-managed dependencies use exact latest-stable pins under
[ADR 0008](../decisions/0008-dependency-currency.md).

## Greenfield Risks

- The proposed target and action model cannot yet be measured against real source
  packages.
- Tool acquisition and platform support have no repository precedent.
- There is no consumer workspace proving the `aspect_rules_py` 2.x wrappers,
  providers, uv workflow, venvs, or required platform matrix.
- There is no CI or remote cache/execution environment in which to validate
  hermeticity and cache behavior.
- First-release parity across many ecosystems is a large compatibility and test
  commitment even though adapters share one architecture.

These risks justify a fixture-first validation milestone before freezing public
rules or CLI selection behavior.
