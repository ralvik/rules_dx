# ADR 0010: Python Foundation

## Status

Accepted.

## Context

The Python developer workflow needs lockfile-authoritative dependencies, hermetic interpreters,
conventional imports, Bazel-native build and test behavior, and an IDE-usable virtual environment.
Stable `rules_python` workflows did not provide the required combined uv and developer-environment
experience when this decision was accepted.

Implementing interpreters, wheel handling, dependency resolution, virtual environments, launchers,
native-extension behavior, and platform support in `rules_dx` would create a parallel Python rules
engine. That would duplicate a mature upstream ecosystem and make `rules_dx` responsible for
Python execution semantics rather than for a focused developer experience.

`aspect_rules_py` 2.x targets this combined workflow, although the available 2.x releases at the
time of acceptance were prereleases. Python follows the Rust application foundation rather than
establishing the first language or environment model.

## Decision

### Foundation And Public Boundary

`rules_dx` uses an exact pinned `aspect_rules_py` 2.x release as the preferred authority for Python
interpreters, execution, dependency resolution, wheels, virtual environments, and providers. This
selection remains unchanged. Focused upstream rules patches and small missing integrations are
permitted under the [core-language remediation strategy](0013-rust-javascript-typescript-foundations.md#core-language-completion-and-remediation);
`rules_dx` does not rewrite a whole ruleset or replace a language engine, dependency solver,
compiler, or runtime.

Using a 2.x prerelease is an explicit exception to
[ADR 0008](0008-dependency-currency.md) until a compatible stable 2.x release exists. The exact
prerelease identity is not pinned by this record; it is pending O25 qualification and the
required consumer and platform evidence. The selected
release must pass the required consumer and platform evidence before shipping. A compatible stable
release replaces the prerelease after passing the same evidence. Upstream gaps require remediation
toward complete core Python support on every required platform, not permanent scope reduction.
The affected capability continues to fail closed and cannot ship as supported until conformance
passes. An alternative upstream requires explicit contract/API review and approval before adoption;
failure never silently selects another foundation.

The project-facing API consists of narrow, conventional `rules_dx` wrappers and setup. The wrappers
preserve the verified upstream provider and execution semantics while owning opinionated defaults
and compatibility across upstream changes. They do not re-export the complete upstream API or turn
unstable upstream symbols into `rules_dx` compatibility promises. Advanced consumers may depend on
and load upstream APIs directly outside that wrapper contract.

Exact wrapper, provider, pytest, coverage, interpreter, and platform mappings remain evidence-gated
in [Open Decisions](../open-decisions.md). The detailed verification contract is under
[Testing](../testing/).

### Generation And Ownership

Python BUILD generation is first-party `rules_dx` generation over Gazelle, not an alternate Python
rules engine. It emits the narrow wrappers while leaving execution and package semantics with the
pinned upstream stack. The authoritative source, ownership, test, entry-point, naming, merge,
resource, import, uv-scope, and paired-stub contract is
[Python Generation](../generation/python.md).

The durable ownership model is one reusable library owner per supported non-test runtime source,
with tests and executable entries represented without duplicating that source. A paired stub is type
metadata on its runtime owner; a stub-only ownership model requires a separate accepted decision.
This ownership model prevents importer count, tests, or executability from moving or multiplying
production source ownership.

Project metadata, `uv.lock`, selected dependency groups, and verified public upstream metadata are
the strict authority for external dependencies. Generation may translate those facts into Bazel
edges, but it does not resolve versions, activate optional groups or extras, install packages, guess
unknown imports, or maintain a consumer-side derived dependency index. Source-only local and
standard-library graphs remain possible without ecosystem metadata.

The shared strict-resolution, literal/computed reference, resource, executable-entry, and Gazelle
merge rules are owned by the [generation contracts](../generation/). Exact recognizers and upstream
mappings that have not yet passed fixtures remain unresolved in
[Open Decisions](../open-decisions.md), not accepted implementation detail in this record.

### Environment Direction

Python development uses the language-neutral `dx env` workflow to project analyzed Bazel state into
a conventional persistent `.venv`. Bazel and the pinned Python rules remain authoritative for the
interpreter, wheel closure, source/import roots, and execution behavior. The environment is an IDE
and interactive-development projection, not a second resolver, installation authority, or build
model.

Ordinary build, test, lint, typecheck, and audit actions do not mutate the workspace environment.
Environment and generated-code projections remain separate explicit workflows and compose through
the shared managed-state model. Exact-target environments derive from the selected target graph;
repository defaults derive from authoritative configured graph facts rather than checkout scans.

The canonical layout, graph projection, dependency materialization, conflict handling, identity,
atomic selection, retention, and safety contracts are owned by
[Environments](../environments/). Exact Python provider and `.venv` mappings, including typed IDE
group configuration, remain unresolved in [Open Decisions](../open-decisions.md).

Python is a later concrete environment projection. It does not define a generic environment API for
Rust, Node, or future languages, and `rules_dx` does not create parallel per-language environment
engines when the shared managed-state contract applies.

### Quality And Availability

The curated Python quality direction is Ruff for formatting and lint, pydoclint for documentation
lint, and Ty for type checking. Ruff is the sole default Python
formatter. Bandit was excluded from v1 by
[ADR 0019](0019-first-release-additional-foundations.md); initial
source-audit tool selection is owned by O11. Capability ownership, source selection, action behavior, fix semantics, and default or
opt-in policy are authoritative under [Quality](../quality/); acquisition and the complete tool
baseline are authoritative under [Tools](../tools/).

Quality adapters consume verified public wrapper/provider contracts. Under the selected foundation,
they do not infer Python semantics from `rules_python` target kinds or unverified provider fields,
and they do not create a second Python dependency graph.

Python is automatically available with the tested release-default interpreter, subject to the
language-version policy in [ADR 0012](0012-language-toolchain-versions.md). Availability remains
lazy: an unused Python foundation must not activate operational payloads, resolve application
dependencies, create configured application targets or actions, project an environment, or download
an interpreter or package closure. Evidence and support claims follow [Testing](../testing/), not the
mere presence of this accepted decision.

## Consequences

- Consumers receive one opinionated Python setup while upstream Python rules retain execution and
  package semantics.
- `rules_dx` owns a smaller wrapper and generation compatibility surface rather than a Python rules
  ecosystem.
- The prerelease exception creates an explicit release gate and an upgrade obligation when a stable
  compatible 2.x release becomes available.
- Strict dependency authority prevents successful generation of guessed or incomplete external
  dependency graphs.
- Stable single ownership supports reusable libraries, isolated tests, thin executable entries, and
  paired type metadata without duplicate runtime owners.
- IDEs receive a conventional graph-derived `.venv` without environment sibling targets or
  build-time workspace mutation.
- Python quality defaults remain capability-oriented and independently lazy rather than expanding
  the application wrapper API.
- Upstream API gaps require focused, tested, pinned patches, bounded integration work, or an
  explicitly reviewed and approved alternative upstream. Unresolved gaps block conformance and
  support claims, not remediation; they do not justify private provider coupling or a project-owned
  replacement Python engine.

## Rejected Alternatives

- Initially selecting stable `rules_python` as the direct developer-facing foundation because its
  workflows did not meet the accepted combined uv and environment requirements at acceptance. This
  preserves the preference for `aspect_rules_py`, not a permanent prohibition on a future direct
  `rules_python` alternative that passes explicit contract/API review, approval, and conformance.
- Re-exporting the complete `aspect_rules_py` API as stable `rules_dx` API.
- Replacing Python interpreters, dependency resolution, or the upstream Python rules engine inside
  `rules_dx`, rather than applying focused upstream rules patches or owning bounded integration.
- Making `python_test` a generic test-driver multiplexer rather than the conventional pytest path.
- Resolving dependencies or populating IDE environments from ambient installers, checkout scans, a
  second manifest, or every optional dependency group.
- Creating or changing `.venv` during ordinary build, test, or quality actions.
- Treating Python's `.venv` shape as the universal language-environment architecture.
