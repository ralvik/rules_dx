# ADR 0006: CLI Command Surface

## Status

Date: 2026-09-09.

Superseded for the `dx check` / `dx fix` / `dx clean` surface by
[ADR 0018](0018-umbrella-check-fix-cleanup-clean.md). Remaining invocation,
scope, output, and workflow-composition constraints stand.

## Context

The CLI needs a small command set with direct, predictable verbs. General umbrella
commands make mutation and CI behavior unclear. Gazelle generates repository
metadata rather than configuring developer preferences. Type checking has distinct
cost, diagnostics, and fix behavior from linting.

Normal usage should expose underlying commands while allowing users to suppress
wrapper chatter without hiding Bazel or tool diagnostics.

## Decision

The command surface is:

- `dx audit` for non-mutating `security` and `license` audits (default both).
- `dx lint` for linters, with mutating fixes when available and non-mutating
  `--check`.
- `dx typecheck` for type checkers, with mutating fixes when available and
  non-mutating `--check`.
- `dx test` for Bazel-owned tests.
- `dx format` for mutating formatting, with non-mutating `--check`.
- `dx build` for Bazel builds.
- `dx docs` for Bazel-owned documentation extraction, validation, and rendering; `--check`
  performs extraction and validation without rendering, and `--serve` previews built outputs.
  Both build and check are source-non-mutating. Exact scope, invocation combinations, and protocol
  mechanics remain under [O54](../open-decisions.md) in the [docs contract](../cli/commands/docs.md).
- `dx update` for authoritative dependency-update workflows.
- `dx generate` for the mutating repository-defined Gazelle workflow, with non-mutating
  freshness validation through `--check`.
- `dx codegen` for repository-wide or exact-target generated-source projections.
- `dx env` for repository and exact-target persistent developer environments.
- `dx setup` for atomic matching codegen and environment selection.
- `dx coverage` for Bazel-owned coverage.
- `dx bazel` for unchanged forwarding to the selected Bazel launcher.
- `dx init` for new-repository scaffolding: module and `//dx` target wiring, workspace
  config, CI caller template, hermetic hook installation, devcontainer, and the `dx`
  version pin. It may bootstrap without an existing `MODULE.bazel` and writes only
  absent files, without Git-based tracked-file inspection. Bootstrap destination
  mechanics and force syntax/managed-replacement behavior remain pending under
  [O49](../open-decisions.md); unqualified force behavior is blocked.
- `dx hooks` for managing the custom hermetic git-hook runner in an existing repository
  (`install`, `uninstall`, `status`). Hook management and staged-file selection are narrow
  Git exceptions: all product Git operations for hooks use hermetic managed Git, never
  ambient Git. Refuse unmanaged existing hooks; force cannot authorize arbitrary hook
  overwrite. Exact installation and snapshot mechanics remain pending under O49; see the
  [hooks contract](../cli/commands/hooks.md).

The approved bootstrap/hook exception allows module creation and staged-path hook selection without
general Git status inspection, clean-worktree requirements, or weakening ordinary workspace
discovery. This amends the design-only scaffold in place; no shipped compatibility behavior
or new unrestricted force API is established.

Historical note: the original decision rejected `dx check` and `dx fix` as
general umbrellas. That rejection is superseded by
[ADR 0018](0018-umbrella-check-fix-cleanup-clean.md), which accepts them as thin
sequential umbrellas over `format`, `lint`, `typecheck`, and `generate --check`.
There remains no `dx doctor`, `dx configure`, or general-purpose
audit umbrella. `generate` is the accepted name for Gazelle-oriented workflows.

Normal text operation prints concise workflow summaries. `--quiet` suppresses wrapper
planning output while Bazel and tool diagnostics remain visible. `--dry-run` and
machine-readable output expose safe operation summaries, never subprocess argv or
forwarded option values.

Operation summaries retain compact canonical Bazel scopes: repository `//...`, directory
recursive patterns, and explicit labels/patterns are not expanded. File scopes list the
deterministically sorted owner or mapped test/coverage labels that resolution selects.
Internal tool, aspect, and implementation dependency labels are not exposed.

Structured workflow scopes contain main-workspace labels only. External repository,
aspect, toolchain, compiler, and dependency labels remain internal. Failures originating
in external repositories use generic stable error codes in NDJSON while detailed Bazel
diagnostics remain on stderr. External scope support requires a later explicit contract.

`--output` accepts `text`, `diff`, or `json`. Text is the concise default and preserves native
subprocess stream routing. Diff owns stdout as a complete unified patch, suppresses `dx`
summaries and normalized diagnostics, and leaves raw child output and operational errors on
stderr; it is accepted only by lint, typecheck, format, and generate. JSON owns stdout
exclusively as one versioned NDJSON event per line and routes raw
subprocess stdout and stderr to stderr so they cannot corrupt the event stream. Explicit JSON
output applies this routing to `dx bazel` while preserving its argv and exit behavior.

Diff and JSON derive from the same validated exact edit sets. They do not rerun tools,
Gazelle, or a comparison workflow. JSON emits exact `change` events in check and default
modes; default mode then emits terminal `mutation` outcomes. Diff renders complete changes
without truncation, includes changes whose later mutation fails, and contains no prose or
mutation annotations on stdout. It represents the complete validated intended patch rather
than final workspace state.

The stable machine API contains only durable dx events; raw BEP and Bazel progress remain
private or passthrough. NDJSON uses additive major/minor evolution and authoritative line
order without redundant correlation fields. Standard reports use SARIF 2.1.0 for lint,
typecheck, and audit, JUnit XML for test, and LCOV for coverage, with file and stdout
destinations. The event schema, error codes, ordering, and stream interaction are defined
in [Output Protocol](../cli/output-protocol.md); exact format profiles and partial-document
rules are defined in [Standard Reports](../cli/standard-reports.md). Exact event schemas and
report profiles are provisional in those contracts until M06/M10 qualification; this record
freezes only the durable surface (verbs, modes, stream ownership, additive evolution).

`--quiet` and `--dry-run` are stable global option names. A non-Gazelle `generate`
backend requires a new decision.

A lint, typecheck, format, or audit invocation whose valid resolved scope has no active
applicable tools succeeds silently as a configured no-op. Text mode prints nothing for the
no-op; machine mode emits only its required lifecycle events and zero diagnostic counts,
without a notice, change, mutation, or report event. Explicit capability disablement is not an
error. Configuration, scope, provider, and analysis failures remain errors.

Ordinary multi-process plans stop on the first required subprocess failure, start no dependent
operation or mutation, and return that process's exit code. Already-running independent
work may only settle or be interrupted for safe cleanup and cannot replace the selected
failure. Bazel-internal quality `--keep_going` remains one subprocess under this rule.

The approved update failure policy makes `dx update` the narrow exception: continue independent
selected dependency sets after a set failure, skip operations dependent on that failure,
preserve successful changes, and return overall failure. This preserves independent progress
without a repository-wide rollback; the [update contract](../cli/commands/audit-update-bazel.md#dx-update)
owns set independence. Exact aggregate exit-code selection, backend mapping, and report
qualification remain pending under [O12](../open-decisions.md). Ordinary plans, including
the `dx check` and `dx fix` umbrellas, retain their first-failure behavior.

Dry-run may execute read-only Bazel queries needed to resolve the plan but never
executes the final workflow or mutation. `dx generate` defaults to repository-wide
operation and additionally accepts an explicit v1 scope: zero or more paths, labels, or
target patterns selecting the Gazelle subtree to refresh. It exposes no Gazelle application
arguments, modes, or index controls. Arguments after `--` are Bazel
command options placed before canonical `//dx:generate`, never Gazelle arguments. Advanced
users may invoke the Bazel target or Gazelle directly outside the `dx generate` contract.
Exact scoped-selection syntax, empty-scope handling, and freshness semantics for scoped
`--check` are qualified under [O48](../open-decisions.md).
The command does not require a recognized manifest or existing application target: Gazelle
discovers supported sources and creates initial target declarations. Manifests remain
authoritative for project and dependency metadata, and generation does not analyze or execute the
new targets.

`dx generate --check` runs the check mode of the same canonical Gazelle workflow used by
default-mode `dx generate`. It accepts the same default repository-wide scope plus the
explicit v1 scoped selection above and no Gazelle
application arguments, writes no workspace file, and exits nonzero with one file-level
exact JSON `change` event for each stale or missing Gazelle-maintained `BUILD` or `BUILD.bazel`
file. The event contains
the original digest and byte-range replacements or complete new-file content. Default mode
emits the same `change` plus file-level mutation shape as lint and format for each terminal
create or modification attempt. Diff mode renders the manifest as a complete unified patch.
The Gazelle integration produces one exact result manifest during the canonical execution,
including exact edits and ADR 0015 ignored-import audit records; no second run, source scan,
before/after comparison, or Git inspection is required.
Any private Bazel helper used to implement the mode is not a second public workflow. Rust does
not parse BUILD files, calculate edits, or run mutating generation in a temporary copy.
First-party extensions may consume Bazel-owned derived metadata, but generation writes no import
map, dependency index, provider inventory, or integration YAML/JSON/TOML sidecar into the consumer
source tree. The private result manifest also remains outside that tree.

`dx generate` does not inspect file-ownership markers or decide which BUILD files Gazelle
may edit. It neither skips files nor implements a separate conflict policy. Gazelle's
directives, merge semantics, and exit status are authoritative.

`dx codegen` with no label invokes canonical `//dx:codegen`, whose implementation uses
the approved Bazel repository-root mechanism and includes every registered production,
test, example, and development projection. `dx codegen <target>` accepts exactly
one explicit compatible target label and derives generated artifacts and language
roots from its configured transitive provider closure. A registered bare schema target
selects every currently declared generated-language projection found through Bazel
reverse-dependency query because aspects cannot traverse backward to wrappers. It
accepts no path, directory, pattern,
multiple labels, profile, or language selector. Codegen changes only managed generated-
source projection state; it does not update BUILD metadata or prepare language
environments.

`dx env` with no label invokes the repository-owned `//dx:env` target and may
materialize configured IDE/development artifacts, including the broad root Python
environment. `dx env <target>` accepts exactly one explicit compatible target label
and materializes the exact persistent environment projections for each supported application
integration represented in that target's analyzed transitive closure, without requiring
sibling environment rules. Previously selected integrations absent from the closure retain their
currently selected projections. It accepts no path, directory, or target-pattern
scope and does not launch an interactive shell. Language integrations own their
native environment contents and facades; Rust owns target resolution, Bazel
invocation, diagnostics, and the all-or-nothing commit of represented language
facades through one shared atomic selection pointer after every integration prepares
successfully.
`bazel run //dx:env` remains the no-label bootstrap before `dx` is available.
`dx env` and `dx codegen` never invoke each other. Either may report a non-fatal scope
mismatch and identify the matching command and scope kind without rendering argv.

`dx setup` with no label prepares both repository-wide workflows; `dx setup <target>`
prepares both exact-target workflows. It accepts no path, directory, target pattern,
multiple labels, profile, or language selector. Environment and codegen preparation may
run as separate local materializers, but one Bazel request selects the union of required
roots, applies both provider aspects, requests both output groups, and emits one BEP
stream. Neither becomes selected unless both validate and one shared
`.dx/setups/current` pointer replacement commits their immutable setup selection. It
  never invokes `dx generate` or mutates Gazelle-maintained BUILD files.

An exact setup carries forward a currently selected side when the target has no
corresponding capability. In particular, a registered bare schema updates all of its
codegen projections without replacing the environment. With no prior selection, the
absent side uses a versioned managed empty generation.

## Consequences

- CI uses non-mutating check modes or directly executable Bazel targets.
- `audit` cannot silently grow to include lint, formatting, tests, or builds.
- Type checking is separate from lint and may apply genuine fixes; suppression
  generation is not part of default fixing.
- `update`, `generate`, `codegen`, and `env` delegate semantics to authoritative
  Bazel-owned resolvers, generators, providers, and environment rules instead of
  implementing them in the CLI.
- Exact target environments need no generated or handwritten language-environment
  target.
- Exact target codegen needs no generated projection sibling for the consumer; Gazelle
  maintains wrappers only where authoritative Bazel language rules require them and
  maps bare schemas to all registered projections.
- `dx bazel` remains the exact-forwarding escape hatch for unsupported workflows.
- A failed or interrupted `dx setup` cannot expose mismatched new/old environment and
  codegen selections.
- Help text and tests must identify which commands mutate by default.

## Rejected Alternatives

- A general `dx check` command composing unrelated workflows (original
  rejection; superseded for the frozen `format → lint → typecheck → generate`
  sequence by [ADR 0018](0018-umbrella-check-fix-cleanup-clean.md)).
- Using `audit` as a synonym for all CI checks.
- `dx configure` as the Gazelle command.
- Requiring `dx fix` or `dx lint --fix` for the default lint mutation.
- Combining lint and type checking into one command.
- A `dx new` app/service/component template generator: out of v1 (rejected 2026-09-09); new-repository
  scaffolding stays in `dx init`, BUILD maintenance stays in `dx generate`, and breaking-change
  rewrites stay open under [O57](../open-decisions.md). No `dx new` command is selected.
- Language/profile selectors, path/pattern scope, or an interactive shell under
  `dx env`.
- Making `env` or `codegen` implicitly run the other instead of using explicit
  `dx setup` composition.
- Printing subprocess argv in normal, dry-run, or machine output, including through a
  `--show-command` option.
