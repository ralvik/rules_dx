# M24 Completion Report: Parity Closure

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
this milestone reconciles the v1 inventory and proves closure properties; it
implements no new adapter and promotes no support claim. O32 stays open for
its owner; this report supplies the milestone-specific evidence (reconciled
matrix, cross-cohort suite results, interaction/stage-order measurement
state, deferred-vs-blocker accounting).

## WP1: Reconciled Baseline Rows, Classes, Capabilities, Routes

The reconciliation is machine-checked, not hand-kept: `quality/
parity_tests.bzl` (`//quality:parity_unit`) pins the complete classified
inventory — 11 adapter-backed classes and 24 deferred classes — and fails
closed on any silent change (unclassified adapter class, undispositioned
class, double claim, ownerless/routeless deferral, or drift in either
pinned set). Deferred adapter implementation stays owned by O32; the gate
owns the inventory, not the adapters.

Closed required cells (adapter-tested with passing capability evidence in
this tree): `rust` (rustfmt, Clippy, rustc typecheck), `python` plus
`python_stub` (Ruff format/lint, Ty typecheck, pydoclint/flake8/pylint lint
opt-ins), `javascript`/`jsx`/`typescript`/`tsx`/`json` (Biome default
lint/format, Prettier JSON-format plus JS/TS-format alternative, ESLint
JS/JSX opt-in, target-coupled `tsc` typecheck), `starlark` (Buildifier
format/lint), `toml` (Taplo format/lint), `markdown` (repo-owned link/
structure check plus Vale prose lint as two ordered stages).

Deferred cells with evidence-backed dispositions (all recorded in the gate
with owner plus frozen route, none silent): Go/C/C++/CUDA/cue/go-module/
jsonnet/pkl/protobuf/qml/shell/terraform/yaml standalone and toolchain
cohorts, Java/Kotlin/Scala-managed/C#/F# managed cohorts, Ruby and
PowerShell tool cohorts (foundations deferred beyond v1 by ADR 0019; tool
cohorts stay in force under O31/O32), and the Vue/Svelte/Astro/MDX
framework regions (owned by O29/O40/O41/O42 plus O32). Swift and Bandit are
evidence-backed v1 exclusions (ADR 0019), not pending cells.

WP1 finding: the CSS/Less/SCSS, HTML, GraphQL, SQL, XML, and Gherkin
baseline rows have no dedicated semantic class in `REAL_CLASS_TO_FAMILY`
(they are served, where served, through the M17 private Node plugin closure
without adapter wiring). Class creation is owned by the O15 registry
review; adapter mapping by O32. This is recorded here, not silently
dropped.

## WP2: Cross-Cohort Suites, Dedup, Exposure, Update

- Convergence and pipeline suites: `//quality:all` passes (synthetic plus
  real pipeline construction, policy, sources registry, native config),
  proving exact class-to-tool mapping, stage composition, and omission of
  unsupported classes/capabilities across the closed cohorts.
- Acquisition deduplication: one shared Maven lock for the JVM cohort, one
  shared Paket lock for the .NET cohort, one private `npm_tools` hub and
  one private `pypi` hub for the Node/Python tool graphs — a tool appearing
  in several baseline rows resolves to one runtime/lock identity by
  construction; the parity gate pins the class side of that identity.
- Environment exposure: all admitted-foundation env plans (`go`, `cc`,
  `java`, `kotlin`, `scala`, `csharp`, `fsharp` plus the M14/M16 plans)
  carry pinned plan tests passing in the full run.
- Update: the interim Renovate `bazel`-manager loop over `.bazelversion`
  (M05 WP4) remains the only qualified update path; ecosystem updater
  mappings stay pending under O12/M26 and are not claimed here.
- Interaction and stage-order measurement: stages order by sorted tool ID
  (the provisional O19 rule); the O19 benchmark corpus, thresholds, and
  per-set interaction measurements are still pending and are carried below
  as a blocking conflict, not treated as proven.

## WP3: Blocking Conflicts, Not Silent Weakening

Two blocking scope conflicts are reported; neither weakens v1 silently:

1. O32 adapter implementation backlog: 24 deferred classes have frozen
   routes but no implemented adapter, and no milestone after M24 owns new
   adapter implementation. Carried to M28 WP1 as a release-blocking
   provisional decision: either schedule the adapter work with owning
   milestones or take evidence-backed admission-policy dispositions before
   any support claim. Required-core obligations are unaffected: every
   required-core cell closed above links to passing evidence.
2. O19 measurement gap: global stage-order and interaction benchmarks are
   unmeasured, so multi-tool convergence relies on the provisional lexical
   order. Carried to M28 WP1 alongside O32.

A foundation deferral removes no baseline tool: every named quality cell
above stays in force under its owning decision.

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (624 targets).
- `bazel test //...`: 143/143 pass, including `//quality:parity_unit` and
  `//quality:all` (7/7).
- `bazel run //dx:generate_check`: clean.
- Coverage inventory needs no change (Starlark-only addition; the
  inventory reconciles Rust/Go sources only);
  `//tools/coverage:coverage_test` passes in the full run.

## Changed Components

- `quality/parity_tests.bzl` (new fail-closed parity gate),
  `quality/BUILD.bazel` (`parity_unit` target, wired into `:all`).
- `docs/open-decisions.md` (O32 reconciliation state, M28 carry).
- `docs/milestones/README.md` (M24 report link, Ready).
- This report.

## Open Items

- O32 stays open: deferred adapter implementation (backlog above), version
  pins, config mappings, and common-runner exceptions; carried to M28 WP1.
- O19 stays open: stage-order/interaction measurement; carried to M28 WP1.
- O15 registry review owns the six class-less baseline rows (CSS/HTML/
  GraphQL/SQL/XML/Gherkin).
- Non-Linux hosts, remote execution, and clean external-consumer
  evidence are unproven (same gap class as M00/M12/M14/M16/M17/M22/M23).
- No supported claim; every deferred cell keeps its owning decision.
