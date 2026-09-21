# Product Scope

## History

Early design started from a near-empty scaffold and recorded its starting
inventory and greenfield risks here. That material now lives only in git
history; the sections below describe the product as built. See
`../../CHANGELOG.md` for release status.

## Product Boundary

`dx` is a transparent user interface over Bazel. It may discover a workspace,
normalize user input, ask Bazel for graph facts, select known workflow targets,
compose a small number of Bazel invocations, and report diagnostics. It must not
own source or dependency graph semantics.

Bazel remains authoritative for dependencies, source ownership, actions, tests,
non-mutating CI checks, type checking, coverage, caching, sandboxing, and local
execution. Remote execution and remote cache remain unqualified: pipeline plus
evaluator actions are local-only until remote is qualified (see
[Remote Tests](../testing/README.md#remote-tests) and the
[remote boundary](../quality/action-model.md#outputs-remote-cache-and-execution)).
Determinism claimed here is local per-cell determinism only, with no cross-cell
union and no remote claim.

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
targets and `dx setup` prepares matching development environments. Unused foundations remain lazy
per [Activation And Laziness](../architecture/README.md#activation-and-laziness): the consumer
surface is one dependency while the internal module graph and `MODULE.bazel.lock` size are
version-resolution cost, not payload.
This does not eliminate the documented Bazel/bootstrap prerequisites or invent application policy.
Symlink creation privilege (Developer Mode on Windows) is such a prerequisite, not a host-SDK
exception: see [Managed State](../environments/managed-state.md#installation-and-ownership).

Windows native toolchains retain hermetic acquisition and MSVC compatibility under
[ADR 0014](../decisions/0014-tested-platform-release-stack.md#decision). No separately installed
Build Tools or host SDK exception is approved.

Additional complete foundations belong in v1 when existing upstream rules permit low-cost hermetic
integration. When completion requires substantial missing infrastructure, an additional foundation
may be explicitly deferred beyond v1 with evidence and a recorded decision rather than delaying the core
indefinitely. This exception does not apply to the required core/framework set or reduce the broad
quality-tool baseline. A language's foundation deferral is not permission to omit its required tools.

The [support matrix](support-matrix.md) is the minimum candidate inventory. Its
[minimal core](support-matrix.md#minimal-required-core) records the
required set, provisional upstreams, owning trackers, and roles.
Review the wider upstream ecosystem and record each named capability's
rules/tools, acquisition route, public providers, dependency authority, applicable platforms,
integration effort, contract compatibility, evidence, and delivery owner, tracked in
qualified seed-only under closed #476-#484 and open work under #796-#800
(successors to closed #510; framework adapters in
open work under #796-#800). Apply the same review to
existing feature exclusions, additional test runners, framework adapters, plugins, audit/update
ecosystems, and codegen pairs. Apply the foundation admission rule above separately from tool and
workflow obligations; an unreviewed entry is not an exclusion. Freeze a reviewed release inventory
rather than claim support for an unbounded set of
unnamed tools. Newly identified candidates require an explicit disposition, not automatic deferral.

Each candidate must become required v1 scope, a recorded additional-foundation deferral, or an
evidence-backed recorded exclusion. No individual foundation is deferred by this policy alone.
There is no post-v1 bucket for workflow scope: delivered work under closed #462,
delivered work under closed #463, and
open work under #787 and #788 (successors to closed #506) are v1 scope per the sole repository maintainer decision in [roadmap](../roadmap.md).
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

Candidate-specific effort, patch/build provenance, upgrade strategy, and
delivery ownership are tracked per candidate under
[remediation bounds](../native-toolchains.md#qualification-questions-and-delivery)
(closed #505): each defect records estimate, actual owner, patch plus upstream issue
plus upgrade tracking, and complete-workflow evidence in
`cc/tests/fixtures/remediation_bounds/` via
`bazel run //tools/ci:remediation_bounds_qualification`. Per-foundation effort
and ownership follow the [minimal core](support-matrix.md#minimal-required-core)
(sole maintainer owns every row until delegated; effort estimated from
qualification evidence as each tracked item lands) with foundation mappings
qualified seed-only under closed #470-#489 and adapters open
under #796-#800. Patch/build provenance follows
[delivery classes](../tools/tool-acquisition.md#delivery-classes) (pinned
reproducible upstream source builds, checked-in digests, SBOM/provenance per
closed #612); upgrade strategy follows
[ADR 0008](../decisions/0008-dependency-currency.md) (exact latest-stable pins,
native bump loop sole updater per closed #461) with per-defect upgrade tracking
under closed #505.
Exact upstream switches and new public APIs remain unapproved until their
domain contracts and work packages are accepted. General permission for these maintenance routes
does not prove any candidate feasible or authorize an unbounded fork.

The project also requires first-party implementation coverage per [testing](../testing/README.md#coverage):
the single exact per-cell gate with zero uncovered lines over the versioned cell
inventories (one gate per required cell, no cross-cell union; the pinned
`dx coverage --min-coverage 97` flag over `//...` is the user-facing configurable
threshold surfaced by the CLI, informational only for the repo verdict, never a
second repo gate).
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
and upstream analysis for each language, tracked in
open work under #796-#800 (successors to closed #510); this does not require executing every target
platform's binaries on the checking host or weakening selected-target build isolation.

The usage test also reports incorrect declaration categories as errors where the ecosystem
distinguishes production, test/development, and build dependencies. For example, usage only by tests
does not justify a production declaration. Evaluate categories across the declared supported
configurations using qualified upstream semantics, not one universal category model. Legitimate
multi-category usage must remain valid. Tests report miscategorization without moving declarations
or rewriting locks; exact ecosystem category and checker mappings are tracked in
open work under #796-#800 (successors to closed #510).

Generate both checks as normal Bazel test targets for the applicable dependency-owning scopes.
They participate in `bazel test //...` and bare `dx test` by default, without a separate opt-in or
default `manual` exclusion. They remain independently runnable through their generated labels.
Exact naming, rules/providers, and generation mappings require qualification tracked in
open work under #796-#800 (successors to closed #510).

Declared-dependency usage permits narrow, explicit exceptions for legitimate uses the checker
cannot recognize, such as dynamic plugins or tools invoked by scripts. Prefer upstream-native
checker configuration and require an explanatory reason for each excepted dependency in its owning
scope. An exception does not disable the usage test, suppress unrelated unused dependencies, or
waive lockfile consistency. Obsolete exceptions are errors: an exception naming a removed dependency
or no longer suppressing a finding in the owning scope must be removed, including when upstream
improvements recognize the legitimate usage. Assess this against the same declared supported
configurations as the usage test, not only the active host configuration. Validation reports the
obsolete exception without deleting it automatically. The selected exception
registry is the owning-scope `depcheck_exceptions.toml` (no separate global
registry or public API): each `[[exception]]` carries its owning dependency
plus an explanatory reason, missing reasons fail, and obsolete entries (removed
dependency or now-recognized usage, including after a checker upgrade
recognizes the usage) fail without auto-deletion, qualified in
`tools/depcheck/` (closed #22) per the
[dependency-check contract](../quality/quality-testing.md#tool-parity) and pinned by
`bazel test //tools/depcheck/...` plus
`bazel run //tools/ci:depcheck_contract`. Upstream-native checker configuration
remains a per-language preference: where a qualified upstream config exists it
is preferred, otherwise the owning-scope file is authoritative; exact
per-language native-configuration choices stay open under #796-#800.

Gazelle automatically maintains generated Bazel dependency edges under the
[generation and merge contract](../generation/common.md#merge-and-lifecycle). Do not add a separate
unused-Bazel-dependency test duplicating that responsibility. Normal `# keep` and handwritten-content
protections remain; this does not authorize pruning user-owned edges. Existing generation conformance
tests remain required.

The two-test requirement and explained usage exceptions are accepted, with required-core
(Rust, Python, JavaScript, TypeScript) plus admitted (Go, Java, Kotlin, Scala, C#, F#, C/C++)
lock/usage scopes, non-import recognition, native exception/reason-validation mappings, and focused
failing/passing fixtures qualified in `tools/depcheck/` (#22; remaining opens under
#796-#800, successors to closed #510). Remaining admitted
quality-adapter mappings stay open under #796-#800 (successors to closed #307 and closed #416-#420); foundation mappings qualified seed-only under closed #476-#484.
Existing strict generation alone is not proof of lockfile consistency or declared-dependency
usage. This is dependency hygiene, not an expansion of `dx audit`.

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
| `dx run` | retain | Build and run runnable targets sequentially (explicit labels/patterns multirun; file/dir scopes require exactly one runnable); see [dx run](../cli/commands/build-test-coverage.md#dx-run) |
| `dx deploy` | retain | Build and run a single deployable target; see [dx deploy](../cli/commands/build-test-coverage.md#dx-deploy) |
| `dx update` | retain | Update dependencies through their authoritative resolvers and Bazel integration |
| `dx bump` | retain, mutating without confirmation | Widen exactly one declared requirement explicitly, then chain the resolver-owned refresh; see [dx bump](../cli/commands/audit-update-bazel.md#dx-bump) |
| `dx migrate` | retain, mutating by default | Upgrade-only breaking-change rewrites over the generation edit-manifest pattern (`--from`/`--to`); live execution fails closed until the first manifest lands; see [dx migrate](../cli/commands/migrate.md) |
| `dx generate` | retain, mutating by default | Default repository-wide Gazelle discovery with explicit v1 path/label/pattern scope; supported sources can create initial targets without manifests or prior targets, and `--check` validates freshness without writes |
| `dx codegen` | retain, mutating | Build and select repository-wide or exact-target generated-source projections for LSPs |
| `dx env` | retain, mutating | Prepare repository defaults or exact per-language projections for one target from Bazel providers |
| `dx setup` | retain, mutating | Atomically prepare and select matching codegen and environment projections without running Gazelle |
| `dx coverage` | retain | Run Bazel coverage and expose its outputs |
| `dx check` | retain, non-mutating | Sequential `format`, `lint`, `typecheck`, then `generate` freshness in `--check` mode; see [check/fix/clean](../cli/commands/check-fix-clean.md) and [ADR 0018](../decisions/0018-umbrella-check-fix-cleanup-clean.md) |
| `dx fix` | retain, mutating by default | Same sequence in default mutating mode with per-file atomic apply; no post-apply rerun (run `check` again); see [check/fix/clean](../cli/commands/check-fix-clean.md) and [ADR 0018](../decisions/0018-umbrella-check-fix-cleanup-clean.md) |
| `dx clean` | retain, mutating managed state only | Prune validated unselected `.dx` generations only (never Bazel outputs unless `--bazel`); see [check/fix/clean](../cli/commands/check-fix-clean.md) and [ADR 0018](../decisions/0018-umbrella-check-fix-cleanup-clean.md) |
| `dx init` | retain, mutating by default | Absent-only scaffolding into a foreign tree; see [dx init](../cli/commands/hooks.md#dx-init) |
| `dx new` | retain, mutating by default | Absent-only per-language scaffolding; see [dx new](../cli/commands/new-upgrade.md#dx-new) |
| `dx upgrade` | retain, mutating by default | One-shot pin plus migrate plus setup composition with recovery pointer; see [dx upgrade](../cli/commands/new-upgrade.md#dx-upgrade) |
| `dx hooks` | retain, mutating by default | Hermetic git-hook runner install/uninstall/status/run; see [dx hooks](../cli/commands/hooks.md#dx-hooks) |
| `dx status` | retain | Consolidated diagnostics surface (toolchain/platform/tools/pin); see [dx status](../cli/commands/status-version.md#dx-status) |
| `dx version` | retain | Single-version pin/launcher with rollback and startup skew gate; see [dx version](../cli/commands/status-version.md#dx-version) |
| `dx watch` | retain, local-only | Thin local-only loop over `build/test/run/lint/typecheck/format/check/fix`; see [dx watch](../cli/commands/watch.md) and [ADR 0017](../decisions/0017-dx-watch.md) |
| `dx owners` | retain | Thin `bazel query`/`cquery` wrapper reusing target resolution; see [inspect wrappers](../cli/commands/inspect.md) |
| `dx deps` | retain | Thin `bazel query`/`cquery` wrapper reusing target resolution; see [inspect wrappers](../cli/commands/inspect.md) |
| `dx why` | retain | Explain one dependency path via `somepath`; see [inspect wrappers](../cli/commands/inspect.md) |
| `dx completion` | retain | Generated static shell scripts from the single command-definition source; see [dx completion](../cli/commands/completion.md) |
| `dx bazel` | retain | Exact-forwarding escape hatch through the selected repository launcher |
| `dx docs` | retain, non-mutating (delivered under #786, successor to closed #581, live successor to closed #421) | Build, check, and serve the unified documentation site over the Bazel-cached extract to aggregate to render chain; `--check` validates without rendering, `--serve` previews the last build locally; see [`dx docs`](../cli/commands/docs.md) |

`dx doctor` and `dx configure` are not commands. Help stays
flag-only (`dx --help`, `dx <cmd> --help`) with no `help` verb.
`dx check` and `dx fix` are thin sequential umbrellas, not a general CI
scheduler. `generate` is preferred to `configure` because Gazelle generates
repository metadata rather than configuring developer preferences.

Thin fit (2026-09-21): the 32-verb surface stays thin because each verb
names one explicit behavior with fail-closed misuse handling. `dx fix`
applies once in phase order with no post-apply rerun (run `dx check`
again); `dx clean` prunes only validated unselected `.dx` state by default
(`--bazel` forwards `bazel clean`, distinct from `--configured` and
`dx bazel`); `dx status` is the unscoped diagnostics surface that always
prints; `dx completion` only prints scripts with no `--check` mode.

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

History: greenfield risks and the fixture-first validation approach that
retired them live only in git history.

## Related issues

Tracking lives in the [roadmap](../roadmap.md). Depcheck: #22. Codegen: #787, #788. Adapters: #796-#800. Docs reintroduction: #786.
