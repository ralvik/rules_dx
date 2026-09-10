# Tool Integrations

## Direction

The first release uses tool-list parity with
[`aspect_rules_lint` v2.8.0](https://github.com/aspect-build/rules_lint/releases/tag/v2.8.0)
as a minimum baseline, not a scope ceiling or an API or behavior compatibility promise.
Curated expansion is mandatory where upstream implementations or rules permit hermetic thin
integration under [First-Release Admission](../product/scope.md#first-release-admission) and
O46 in the [open-decision register](../open-decisions.md), without building replacement stacks.
Curated default tool selections are owned by the [tool baseline](../tools/tool-baseline.md);
this document owns adapter mechanics and does not re-pin defaults.
Swift and SwiftFormat are excluded from v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md); see
[Swift Feasibility](../tools/tool-baseline.md#swift-feasibility). Reconsidering Swift after v1
requires a new scope decision.

The project does not depend on or fork `aspect_rules_lint`. Project-owned
Starlark aspects declare actions, and project-owned Rust runners normalize tool
execution, reports, formatted outputs, and fixes. Upstream implementations and
documentation are design references under their Apache-2.0 license; copied code,
if any, must retain required notices.

Tool-specific public rule families for Ruff, Ty, and similar adapters are not
planned by default. Public APIs are capability-oriented and conventionally named,
such as `lint_aspect`, `typecheck_aspect`, `format_aspect`, and `audit_aspect`. Tool adapters remain
internal unless a concrete external API is needed.

`QualitySourcesInfo` is the accepted public cross-rule integration concept for custom source-owning
rules. Its exact API remains provisional under O15. The authoritative conceptual boundary and
candidate shape are in [Quality Sources and Applicability](quality-sources.md); this document does
not redefine its fields.

## Common Adapter Contract

Each adapter defines only what differs for its tool:

- Capability-specific supported semantic file-class IDs and any authoritative upstream
  provider requirements.
- Tool executable and runtime inputs.
- Configuration inputs and arguments.
- Check, format, or fix invocation.
- Diagnostic parser and optional fix-output parser.
- Supported platforms and known hermeticity limits.

Capability-owned typed workspace policy selects adapters. Adapters combine that selection with the
target's semantic source classes and its explicitly bound local native configuration. A selected
adapter creates one stage only when this intersection contains direct sources, and the stage receives
only those sources.

Checked-in native-policy ownership, exact config closures and action inputs, Gazelle target and
`aspect_hints` binding, stale merge behavior, dedicated Ruff config rules, exclusions, config cycles,
and config mutation are defined in [Native Configuration](native-configuration.md). Adapters consume
that contract; they do not search the workspace or invent a second configuration mechanism.

Standalone quality adapters are available through independently configured policy families and
do not require application-foundation activation. Target-coupled adapters are available only
when declared targets and authoritative compiler/toolchain providers activate their application
foundation. Gazelle may create an initial target from source discovery, but that discovery does
not itself activate or supply the context for a target-coupled tool.
Provider class matching and capability-specific adapter metadata determine whether an adapter
creates a stage for a selected target. Raw extension matching is registry validation, not an
adapter-selection API. Tool artifacts remain lazy Bazel
inputs and are not fetched merely because unrelated targets are built.

Each quality policy family defines curated default lint/audit adapters and a default formatter
set over its registered semantic file classes. Workspace overrides may add or remove supported tools
and select multiple formatters.
Adapter manifests supply supported classes; user configuration does not repeat extension lists.
Unknown tools and target-coupled tools without required authoritative application context fail
during configuration or analysis.
Every built-in family is available with lazy curated defaults; there is no separate quality-
family enable list. A family with no effective target sources creates no stage or fetch.
Selecting a broad adapter authorizes only the classes belonging to that policy family. If
several families select the same adapter, one target/capability stage receives the union of
their effective subsets; the adapter is not executed or acquired once per family. Family
configuration selects IDs only. It cannot specialize adapter flags, version, runtime, executable,
native config, or instance identity.

Normal overrides are policy-family lists using stable built-in tool identifiers. They cannot supply
native config labels. Capability tags and custom-rule participation are defined in
[Quality Sources and Applicability](quality-sources.md#tags); adapters never infer custom-rule sources
from arbitrary attributes, rule names, raw extensions, or `DefaultInfo` outputs.
This keeps the internal adapter record, executable, runtime, package closure, and acquisition
metadata out of the consumer API without creating a global config registry.

Omitted selections use curated tool-selection defaults. Empty lint/typecheck/audit/formatter lists
explicitly disable those capabilities for the configured policy family. Separate enable flags
are not exposed, and disabled tools create no actions or downloads.

If a valid selected scope has no active applicable adapter for the invoked quality
capability, the command succeeds silently as a configured no-op. It emits no notice or
tool-result event and fetches no tool. Invalid selection or configuration does not use this
path.

Under the [default lifecycle policy](../tools/tool-baseline.md#default-lifecycle-direction),
only curated lint/audit additions are eligible for a minor release after compatibility
qualification, release notes, and a tested preservation override. Default removals,
default formatter-set changes, and otherwise breaking consumer workflows require a major release.
Normal tool-version updates follow the exact-pin
currency policy and may still change native diagnostics or formatting behavior.

The shared Rust runner owns process execution, deterministic environment,
response-file handling, normalized diagnostics, source digest recording, and the
internal Quality Result Protocol. The CLI owns projection into the public Output
Protocol. Starlark owns action inputs, toolchain selection, action registration, the
`dx_results` output group, and cache boundaries.

The runner also owns edit derivation. When a tool mutates writable source copies,
the runner compares each final copy with its original bytes and emits the normalized
sorted, non-overlapping byte-range edit set. When a tool emits native edits, the
runner validates and normalizes them to the same contract. Adapters specify tool
invocation and diagnostic/native-output parsing but do not implement independent
diff or merge algorithms.

## Action And Result Boundaries

Adapters participate in the target-scoped capability pipelines defined by the
[Quality Action Model](action-model.md). They do not choose action granularity, stage ordering,
convergence, aggregation, or CLI apply behavior.

The exact Protobuf schema, source and diagnostic identity, replacement semantics, execution status,
evaluator behavior, and compatibility rules are defined in
[Quality Result Protocol](quality-result-protocol.md). The public NDJSON and standard-report
projections are separately defined in [Output Protocol](../cli/output-protocol.md).

## Initial Adapter Qualification

The Vale config-required and Taplo text-parser exceptions below are approved. Exact O20 mappings remain untested, and no new public API or
support claim follows from these selections. Acquisition evidence
lives in [Tool Acquisition](../tools/tool-acquisition.md#initial-artifact-research).

- **Buildifier:** qualify `--mode=check --format=json` with `--lint=off` for formatting and
  `--lint=warn` for linting, plus explicit `--config` or `--config=off`. The
  [v8.5.1 CLI](https://github.com/bazel-contrib/buildtools/blob/v8.5.1/buildifier/buildifier.go)
  resets the exit status in JSON mode; a read failure can omit an input from the report. Require
  complete expected-input coverage and classify stderr failures, not just exit zero and valid JSON.
  Native `--lint=fix` also formats: qualify that coupling on virtual copies and derive edits through
  the common runner. Native fix-capability hints do not satisfy the protocol's fixability guarantee.
  Research notes (unproven mappings): `--format` accepts only `text|json` with `--mode=check`
  per [validation](https://github.com/bazel-contrib/buildtools/blob/v8.5.1/buildifier/config/validation.go);
  `--lint=fix` requires `--mode=fix`; exit codes are `0` ok, `1` syntax, `2` usage, `3` IO/internal,
  `4` check-fail. JSON shape is `Diagnostics{success, files[{filename, formatted, valid, warnings[]}]}` with
  `warning{start/end line/column, category, actionable, autoFixable, message, url}` per
  [diagnostics](https://github.com/bazel-contrib/buildtools/blob/v8.5.1/buildifier/utils/diagnostics.go).
  Config precedence is `--config` flag, then `BUILDIFIER_CONFIG`, then upward `.buildifier.json`
  search; `--config=example` prints an example and exits `0`, `off` disables. Versions are
  observations, not pins; recheck latest stable at implementation.
- **Taplo:** the [0.10.0 CLI](https://github.com/tamasfe/taplo/blob/0.10.0/crates/taplo-cli/src/args.rs)
  exposes text lint diagnostics, not JSON/SARIF; findings and operational errors share exit 1.
  A version-qualified, fail-closed text parser is permitted if conformance fixtures prove complete
  diagnostic parsing and reliable distinction between completed findings and operational failure.
  Qualify deterministic color/locale settings, source coordinates, and every supported output form.
  Unrecognized, malformed, truncated, or ambiguously classified output fails the action, never
  becomes an empty successful result. Requalify the parser on every tool update. If these proofs
  fail, stop the integration and propose upstream structured output or a reviewed narrow patch;
  do not fall back to exit-code-only classification. This exception is Taplo-specific and preserves
  the common structured result protocol. [Config loading](https://github.com/tamasfe/taplo/blob/0.10.0/crates/taplo-cli/src/lib.rs)
  can warn and continue with defaults, so invalid-config fallback must fail the action.
  Qualify escaped exact-file glob arguments, local schema closures and transitive references;
  `--no-schema` alone is not proof of network isolation.
  Research notes (unproven mappings): `lint` aliases `check`/`validate`, `format` alias `fmt`;
  global `--colors auto|always|never` plus `--config/-c` (`TAPLO_CONFIG` env) and `--no-auto-config`;
  `lint` accepts `--schema`, `--schema-catalog`, `--default-schema-catalogs`, `--no-schema`;
  `format` accepts `--check`, `--diff`, `--force`, `--option key=value`; diagnostics render through
  `codespan-reporting` to stderr. Schema transitive `$ref`/catalog recursion and `NO_COLOR`/`LANG`
  handling live outside the inspected CLI files and remain unproven.
- **Vale:** qualify explicit `--config`, `--no-global`, and `--output=JSON`. The
  [v3.20.0 CLI](https://github.com/vale-cli/vale/blob/v3.20.0/cmd/vale/main.go) returns 0 for
  warning-only findings, 1 for error findings, and 2 for operational failure; the shared evaluator
  must still enforce its warning threshold. Preserve native severity and checked-in styles, including
  inherited rules, dictionaries, scripts, and pipeline files; never run `vale sync` in a consumer
  action. Its hidden native fix command is only a qualification candidate, not an approved fix API.
  Vale has [no default configuration](https://github.com/vale-cli/vale/blob/v3.20.0/internal/core/config.go),
  so the approved [config-required exception](native-configuration.md#authority) applies. Missing
  required config is an actionable failure, not a hidden preset or a silently skipped stage.
  Research notes (unproven mappings): JSON default is `map[path][]Alert` with
  `Alert{Check, Message, Severity suggestion|warning|error, Span[begin,end], Line, Match, Link,
  Description, Action}`; `--counts` yields `{files, counts}`; line form is
  `path:line:col:check:message`. Missing config reports `E100`; global `xdg .vale.ini` loads in
  addition unless `--no-global`. `StylesPath` plus `.vale-config/*.ini` pipeline files, vocabularies,
  dictionaries, templates, scripts, and filters form the closure; `sync` downloads packages and must
  never run in consumer actions.

Candidate native filenames are `.buildifier.json`, `.taplo.toml`/`taplo.toml`, and `.vale.ini`.
Freeze them in [Native Configuration](native-configuration.md#discovery) only with exact binding,
co-location, and complete config-closure tests. Explicit config flags do not themselves prove absence
of parent/home discovery; qualify source inspection with sandbox and hostile-home fixtures.

Clippy's native filename is frozen as `clippy.toml`. The pinned binary discovers only that basename
upward from the working directory: a direct probe shows a `clippy_test.toml` in the working
directory is silently ignored while `clippy.toml` applies. Runner wiring pins a hinted run's working
directory to the mirrored config's parent directory and leaves unhinted runs at the empty scratch
root. `clippy_cfg` bound to `fixture_real_rust_hinted` proves the binding end to end: its lint
result carries `clippy::too_many_arguments` where the unhinted fixture stays silent.

- **Repository-owned Markdown checks:** link and structure validation is repository-owned and
  distinct from Vale (M04 WP3, O20). The checker is the Rust crate `//quality/markdown`: it parses
  one Markdown source plus its declared sibling-file closure and reports structured findings for
  dangling relative file targets, missing same-file or resolved-file anchors, heading-hierarchy
  violations (exactly one H1, no skipped levels), and fenced code blocks without a language tag.
  Remote URLs are recorded but never fetched. Undeclared link targets fail closed as findings, never
  as silent passes. The `//quality/markdown:quality_markdown` binary checks `--source WS_PATH=EXEC_PATH` files
  against the union `--source`/`--sibling WS_PATH=EXEC_PATH` closure and prints one JSON
  `{"path","line","kind","message"}` object per finding; exit `0` when checked, `2` on bad
  arguments, unreadable files, or non-UTF-8 input.
  Runner-backend wiring (M04 WP3) follows the M04 real-adapter pattern: tool ID `markdown_check`
  (`REAL_ADAPTERS` lint `markdown`, `real_markdown_family` lint alongside `vale`, aspect binary
  `//quality/markdown:quality_markdown`), one `--source WS=ABS` mapping per stage file with no
  config and scratch-root cwd (`commands::markdown_check`), NDJSON parsing keyed by workspace path
  with re-rooting onto scratch-absolute paths before placement (`parsers::parse_markdown_findings`;
  findings exist only on exit 0, any other exit is an action failure), check-only `apply_fix`
  returning its input, and diagnostics carrying the kebab-case kind as `rule_id` at `Error`
  severity. Sibling plumbing (M05 WP1): source targets declare unclassified link-resolution files
  via `markdown_siblings` on `real_source_target` (for example a `LICENSE` file); the aspect passes
  one `--sibling WS=ABS` mapping per sibling alongside the action inputs, only when a
  `markdown_check` stage runs, and the runner mirrors siblings into the scratch tree through
  `run_real_pipeline_with_siblings` without linting them or entering snapshots. A sibling
  shadowing a checked source is dropped at the aspect (the source wins); a `--sibling` colliding
  with a `--source` at the runner CLI is a `DuplicateFile` action failure. Direct-Bazel dogfood
  (executing the wired stage on fixtures) follows in M05; this
  boundary freezes the checker shape and its pipeline integration, not its execution evidence.

## First-Release Tool Baseline

The frozen parity matrix, curated tool selections, additions, and feasibility assessments are maintained
in [First-Release Tool Baseline](../tools/tool-baseline.md). This document defines how those
tools integrate with the common adapter and result contracts.

## Versions and Updates

All managed tools are supplied by `rules_dx` as pinned, hermetic, ready-to-run Bazel
dependencies. Compiler-coupled tools may come from authoritative selected toolchains,
standalone tools from checksummed artifacts, interpreted tools from private
`rules_dx`-owned package locks, and exceptional closures from assembled release bundles.
Consumers see one interface and never configure quality-tool acquisition or install a
tool. Typed behavior policy and declared [native configuration](native-configuration.md) remain
available. Exact delivery, platform, pinning, update, and proof policy is defined in
[Tool Acquisition](../tools/tool-acquisition.md).

## First-Release Integration Process

Adding or upgrading a first-release integration must satisfy
[Tool Acquisition](../tools/tool-acquisition.md), the
[Native Configuration](native-configuration.md) contract when the tool has native policy, and
include:

- A deterministic delivery-class and artifact/runtime identity for each required
  execution platform.
- A representative source target and native tool configuration.
- Passing and failing diagnostics tests.
- Fix or formatted-output tests when supported.
- Action-input, deterministic-argument, sandbox, cache, and remote tests.
- Same-file formatter convergence tests for shared fixed points, cycles, iteration limits, and
  stable stage ordering.
- Passing focused acquisition and performance proofs for its delivery class.
- Documentation of any intentional difference from the native tool or v2.8.0
  baseline.

Tool-specific architecture documents are unnecessary unless an adapter cannot fit
the common contract or needs a new public API.
