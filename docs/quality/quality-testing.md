# Quality Workflow Testing

## Result Protocol

The normative message and compatibility semantics are defined in
[Quality Result Protocol](quality-result-protocol.md). This section defines the required
test evidence.

Unified-result analysis and integration tests verify that every applicable
lint/typecheck/format target/capability pipeline and audit target/tool action contributes
exactly one result message to `dx_results`, target pipelines remain independently cacheable,
and no invocation-wide aggregate result action is created.

Protocol tests verify binary Protobuf round trips, malformed and truncated input
rejection, unknown-field compatibility, required semantic validation, and bounded
per-action fixture sizes. Compatibility fixtures reject unknown schema majors,
accept newer minors containing ignorable fields, and reject newer-minor mutation
results when required replacement semantics are absent or unsupported.

Location fixtures cover ASCII and multibyte UTF-8, zero-width ranges, end-of-file
ranges, malformed boundaries, out-of-bounds offsets, and conversion from each
adapter's native coordinate system. Path fixtures accept normalized
workspace-relative UTF-8 paths and reject absolute paths, backslashes, empty paths,
`.` or `..` components, and paths escaping the workspace root.

Generated-source fixtures prove generated artifacts may be read-only typecheck
context but never direct quality subjects or replacements. Generated-context
diagnostics remain visible and fail typecheck; they are locationless unless a valid
checked-in workspace source range is available.

Digest fixtures use published BLAKE3 vectors, require exactly 32 raw bytes, detect
single-byte source changes, and reject missing or incorrectly sized digests before
mutation. Runner fixtures prove fixed-copy and native-edit tools produce the same
normalized edit contract, final bytes reconstruct exactly, output ordering is
deterministic, and malformed or overlapping native edits are rejected.
Pipeline fixtures validate each native stage edit before virtual application and prove only
the original-to-stable candidate leaves the action.

## Diagnostics And Evaluation

Diagnostic fixtures require a valid severity enum, exercise each threshold, and
prove expected findings exit codes are interpreted through parsed diagnostics.
Unexpected exits, signals, launch failures, malformed output, and protocol failures
must fail at every threshold. Default-mode tests prove warnings and errors fail and
informational diagnostics pass. CLI parsing rejects `--fail-on never`.

Fixability fixtures require a boolean on every diagnostic and accept `true` only when an exact
same-file candidate is present and the adapter proves it resolves that finding. They reject or
downgrade rule-level capability hints, pathless claims, missing edits, and cross-file
associations. Duplicate finding merge is conservative: one false contributor makes the public
finding unfixable. Check output retains every finding and marks only guaranteed fixes.

JSON fixtures retain initial and stable terminal findings with explicit snapshot identity.
Default mode derives initial `fixed`, `remaining`, or `not_applied` resolution from convergence
and the terminal file outcome; terminal diagnostics are always unfixable. Text shows initial
findings in check mode, terminal findings for applied files, and initial findings for rejected
files. Diff mode renders no normalized diagnostics or summaries on stderr. Event-order fixtures
prove mutating diagnostics appear only after the outcomes needed for truthful resolution.

SARIF fixtures prove check mode describes initial bytes, while mutating mode uses terminal
findings for applied candidates and initial findings for rejected files. An all-fixed tool run
has an empty results array rather than disappearing. Fixtures reject same-run use of
`baselineState`, emit no v1 SARIF fixes, preserve stable result-matching fields, and mark
partial reports non-authoritative for scanner upload.

Action-status fixtures prove ordinary findings write complete result messages and leave
their Bazel actions successful. Multi-action fixtures prove one finding does not
prevent other results from materializing. Direct Bazel evaluator tests and `dx`
tests must produce identical policy outcomes from the same manifest set.

Evaluator `aquery` and cache tests prove validation creates one consumer per result,
creates no aggregate action, and a threshold-only change invalidates evaluators but
not analyzers. Direct Bazel failure tests cover default `warning`, all three
thresholds, replacement-present results at every threshold, validation disabled, and
`--keep_going` collection behavior.

## Initial Adapter Evidence

The O20 [adapter qualification contract](tool-integrations.md#initial-adapter-qualification) requires:

- Buildifier JSON with an omitted unreadable input, syntax errors, config/table closure, and
  lint-fix/format coupling. Exit zero must not hide incomplete analysis. Prove the `success` field
  plus complete expected-input coverage, unreadable-file exit `3` with sibling omission from JSON,
  `lint=fix` with format-clean exit `0`, and `BUILDIFIER_CONFIG` versus `.buildifier.json` precedence
  from the [research notes](tool-integrations.md#initial-adapter-qualification).
- Taplo malformed/unreadable-config fallback, text diagnostic classification, paths containing glob
  characters, document schema directives and remote transitive references. Fail closed on incomplete
  parsing and undeclared network inputs. Qualify the permitted text parser against each exact tool
  version, including clean success, findings, operational failure, unknown/truncated output,
  deterministic color/locale settings, and Unicode/CRLF coordinates. Ambiguous output must fail
  at every severity threshold; upgrades must requalify the parser before adoption. Prove corrupt
  `.taplo.toml` warn-and-fallback rejection, `--no-schema` suppression of schema errors, nested
  `$ref`/catalog offline-or-fail-closed behavior, bare exact paths without glob metacharacters, and
  a `--colors=never` plus `NO_COLOR`/UTF-8 locale output-byte matrix.
- Vale warning-only exit zero, missing config, global/home contamination, inherited styles and
  script closure, and native fix ambiguity. Selected applicable Vale without a declared config must
  fail with guidance to supply and bind native policy; prove no generated policy, hidden style, or
  silent omission. Contrast valid declared config with explicit disablement and empty source subsets,
  which retain their normal lazy no-op behavior. Prove the `0/1/2` matrix with `--output=JSON` and
  `--no-exit`, missing-config `E100`, JSON/line transcripts against the `Alert` shape, hermetic
  `StylesPath` plus `Packages` plus styles/rules/dictionaries/scripts with `.vale-config` pipeline
  files, mocked `vale sync` network behavior, and hidden `fix --apply` round trips.

- Repo-owned Markdown checker (`//quality/markdown:quality_markdown`) exit matrix and closure rule:
  exit `0` with NDJSON findings and exit `0` with empty stdout when clean; exit `2` on bad
  arguments, unreadable files, or non-UTF-8 input with no partial findings. Finding `path` keys
  are the `--source` workspace paths over the union `--source`/`--sibling` closure; remote URLs
  are recorded-never-fetched. The checker performs no config discovery and takes no config, so
  the backend runs it from the scratch root; it is check-only (`apply_fix` returns its input).
  Prove the matrix and the workspace-keyed grammar through the crate unit tests, the backend
  `markdown_check` tests, and direct-Bazel execution of the wired stage on
  `//quality/testdata:fixture_real_markdown` with the evaluator passing `--fail_on info`.

## Configured Policy

The normative ownership and binding behavior exercised by the native-config fixtures is defined in
[Native Configuration](native-configuration.md).

Default-policy tests assert the exact curated adapter and formatter set for every quality policy
family and semantic file class against the
[curated baseline](../tools/tool-baseline.md#curated-differences), not every named integration.
Override tests cover additions, removals, generated local
native-config bindings, formatter-set replacement, unknown tools, target-coupled tools without
authoritative providers, and edit conflicts. Actions
for configured tools remain absent until applicable targets are analyzed.

Selection-API tests cover stable policy-family tool identifiers, deterministic ordered lists
for every capability, and class-aware applicability with no workspace native-config map or
user-authored extension lists. They reject unknown or duplicate identifiers, use under the
wrong family/capability, executable or acquisition overrides, and consumer-constructed
adapter objects.
Cross-family selection tests prove JavaScript-only Prettier selection enables JavaScript and JSX
but not TypeScript, TSX, JSON, Markdown, or CSS formatting. TypeScript policy independently
controls TypeScript and TSX. Selecting the same Prettier adapter in JavaScript and Markdown
policy creates one stage over the authorized source union and one acquisition identity;
incompatible adapter identities fail configuration. Registry tests require every class to have
exactly one policy family.
Selection-schema tests reject family-specific flags, presets, versions, executable/runtime
labels, native-config labels, and separately named adapter instances. Native-config fixtures
prove one deduplicated broad-tool stage still resolves declared behavior per effective source.

Defaulting tests distinguish omission from explicit disablement: omitted selections use
curated tool-selection defaults and empty lint/typecheck/audit/formatter lists create no corresponding
actions. Disabled tools contribute no environment entry and trigger no artifact or runtime
fetch. No separate enable flag is accepted.
Family-activation tests omit all family configuration and prove matching semantic classes receive
their curated defaults. Families with no matching selected source remain absent from actions and
fetches. Explicit family blocks override or disable defaults but do not act as enable switches.
Opt-in fixtures include applicable sources and native configs for flake8, pylint, and ESLint:
omitting tool selections must leave these tools unselected, with no actions, environment entries,
or artifact/runtime fetches. Explicit selection activates only applicable quality actions and
the selected environment tools; unrelated opt-ins remain lazy.

Quality/application separation tests expose TypeScript through a custom `QualitySourcesInfo`
target with no authoritative TypeScript application context. Selected standalone formatting/linting adapters
run without Node dependencies, `tsc`, application Gazelle plugins, or a TypeScript environment.
Target-coupled `tsc` remains inapplicable and unfetched until authoritative TypeScript providers
and targets activate the application foundation.

No-applicable-tool tests cover an explicitly disabled capability, a valid mixed-class target
with no matching adapter, and a valid target whose classes are unsupported by selected tools.
Each exits `0`, produces no text output, emits no NDJSON notice/tool-result/mutation/report
event, ends machine output with complete zero diagnostic counts, creates no tool action, and
fetches no tool artifact. Control cases prove invalid identifiers, config labels, scopes,
providers, and analysis still fail rather than becoming no-ops.

Native-config tests prove pinned upstream behavioral defaults require no repository file for tools
permitting config-free operation, while applicable Vale requires declared config and fails without it.
Explicit primary and transitive config labels are complete action inputs, and changing one invalidates every
and only consuming action. Tool-specific Gazelle tests add, move, nest, and remove native
configs and prove generation changes bindings only for targets whose relevant config set
changes. Unrelated target analysis edges and action keys remain byte-for-byte equivalent;
no registry, discovery index, or unrelated config enters an action. Undeclared
includes/imports, missing labels, action-time upward discovery, home-directory
configuration, and conventional filenames absent from the generated graph cannot affect
execution and fail clearly where the tool requests them. Exclusion tests first prove provider
classes, adapter-supported classes, workspace policy, and capability produce the exact stage
subset. They then prove only native configuration and upstream built-in exclusions can remove
files from that subset, apart from mandatory VCS-ignore isolation, and `rules_dx` does not apply
a second user-authored ignore policy.

Package-boundary tests add recognized configs to source and config-only directories with no
existing BUILD file. Gazelle creates a local `BUILD.bazel` and typed config target in every
case, splits parent globs at the new subpackage, and regenerates valid source ownership under
that boundary. Tests permit labels, ownership, and action keys to change in the affected
subtree while proving packages outside it remain identical. Explicit hand-written ownership
that cannot survive the new boundary fails through Gazelle's normal conflict behavior rather
than receiving a hidden ancestor-package fallback.

Selection-gated generation tests place valid, invalid, nested, and config-only native files
for supported but unselected tools throughout the repository. They cause no package creation,
validation, config target, hint, action, or tool fetch. Selecting the tool and regenerating
activates those files; deselecting it removes managed targets and hints while retaining BUILD
files and package boundaries according to standard Gazelle behavior.

Filename-recognition tests place unknown, custom, case-variant, and heuristically similar
config names beside supported files. Only exact documented names for selected tools trigger
parsing, package creation, targets, or hints. Unsupported names are silently inert, and no
directive for designating one is accepted in v1.

Naming tests require each built-in config rule kind and generated target to use
`<tool>_config`, including `ruff_config`, `eslint_config`, and `checkstyle_config`. Compatible
existing rules merge under standard Gazelle semantics. Same-name different-kind collisions
fail generation, supported config-filename changes retain the target label, and numeric,
hashed, private-prefix, or filename-derived fallback names are never generated.

Visibility tests prove generated config targets explicitly use
`//<package>:__subpackages__`, with `//:__subpackages__` at the root, regardless of package
default visibility. Declaring and descendant packages may consume the target; siblings and
external repositories fail visibility checks. Gazelle owns the attribute while standard
attribute-level `# keep` behavior remains effective.

Config-target selection tests prove generated targets have no `manual` or `rules_dx`-specific
tag, appear in normal package and wildcard expansion, remain lightweight to build, and expose
only their typed config provider and checked-in declared files.

Binding-shape tests require each applicable source rule's `aspect_hints` to contain its
relevant `<tool>_config` labels directly. Generation creates no `quality_config` aggregator,
package wrapper, or equivalent intermediate provider, and repeated direct labels remain
deterministically ordered and idempotent.

Hint-merge tests follow upstream Gazelle list behavior: surviving existing entries and
comments retain their position, value-level `# keep` survives, stale `rules_dx` entries are
removed, and missing generated entries append in canonical-label order. Third-party values
remain present and keep their relative order; generation never sorts the complete shared
`aspect_hints` list.

Removal tests follow upstream Gazelle behavior: the extension emits a narrowly matching empty
stub, stale generated config rules disappear only when no configured non-empty attribute or
`# keep` protection remains, and unrelated or structurally unfamiliar user rules survive.
Fixtures cover `# keep` on rules, attributes, and individual `aspect_hints` values. Kept stale
or invalid bindings are preserved by generation but fail ordinary analysis validation. The
BUILD file remains after its last generated rule is removed, including when empty, so the
package boundary does not collapse automatically. Repeated generation is idempotent.

Source-policy tests accept checked-in primary and supporting config files and reject generated
artifacts, generator-rule outputs, and generated config constructors in typed config targets
or `aspect_hints` before any quality action is registered. Rejection identifies the offending
target and explains that native quality config must be checked in for editor/tool parity.

Resolution-conformance tests compare each supported tool's native nearest-config behavior
with its Gazelle-generated local sets and sandbox layout, including multiple configs relevant
to one target, filename precedence, explicit inheritance, relative path semantics, and
removal fallback. Ruff runs once for such a target and chooses the expected config for each
direct source. A new native file is intentionally inert before regeneration. Quality
commands never invoke Gazelle or reject stale generated metadata; generation freshness is
tested independently.

Ruff config-format tests recognize `ruff.toml` and `.ruff.toml` independently and reject both
in one directory instead of applying native precedence. They reject `[tool.ruff]` in any
`pyproject.toml` during generation with guidance to move it to a dedicated file. With no Ruff
config present, upstream tool defaults apply: `rules_dx` merges no behavioral preset, per the
no-config golden tests below. Editing
Python packaging metadata in a `pyproject.toml` without `[tool.ruff]` leaves all Ruff analysis
edges and action keys unchanged. Equivalent dedicated-file policies for other tools require
fixtures proving the dedicated format has the same semantics as its shared-manifest form.

Ruff `extend` tests cover single and transitive relative chains, an extended config that is
not directly selected by any source, and narrow action invalidation after editing each
member. Generation rejects missing files, cycles, absolute paths, external-repository
references, and normalized paths outside the main repository. No user-authored
config-dependency list is accepted or required.

Every linter, formatter, type checker, and source-audit integration has file-selection tests
proving its supported isolation mechanism disables Git and other VCS ignore discovery.
Workspace and nested ignore files, repository-information excludes, global excludes,
tracked/untracked state, and changes to any of them cannot alter action keys, selected direct
sources, or results. Declared native excludes and Bazel ownership remain effective. Ruff
fixtures additionally prove `--no-respect-gitignore` is forced. An adapter without a passing
isolation fixture cannot enter the supported baseline.

Config-as-source tests prove generated TOML, JavaScript, XML, and other representative
`<tool>_config` targets both provide policy and directly create applicable file-class quality
actions for their checked-in config source without another wrapper target. They expose the same
canonical semantic-class source facts as ordinary wrappers. Multiple-owner
fixtures add a second ordinary owner and prove all configured contexts execute, identical
stable candidates deduplicate, and differing final candidates reject only that file while unrelated
valid files apply.

Cross-tool binding tests prove config targets receive the same applicable native-config hints
as ordinary targets of their file class, including Ruff-under-Taplo and JavaScript-config
cases. Cycle fixtures cover direct self-reference, two-tool cycles, and transitive cycles
through imported config dependencies; analysis reports the complete label cycle and creates
no quality action.

Config-mutation tests format and lint representative native configs, converge virtually under
the currently declared policy, apply each valid stable file once against its original snapshot,
and prove the command invokes neither Gazelle nor a post-apply quality pass. The next explicit
command observes the updated bytes and reruns exactly the normal Bazel consumers.

Generation-failure tests inject invalid configs and ownership conflicts after earlier
directories produce valid edits. Expected files, diagnostics, and status match a direct
pinned Gazelle invocation exactly. `dx` performs no repository-wide staging, rollback, or
Rust-side prevalidation and does not claim atomic generation.

Gazelle merge tests cover generated and hand-written source rules with empty, existing
`rules_dx`, third-party, and mixed `aspect_hints`. Generation adds, replaces, or removes only
the entries owned by `rules_dx` and preserves unrelated attributes, comments and ordering
where Gazelle guarantees them, and third-party hints. Source-only fixtures begin without a
manifest or BUILD file and prove first generation creates the same source-rule shape before
quality hints are merged, without inventing project/dependency metadata or activating a quality
action. There is no config-binding opt-out.
The config-binding component uses the same shared Gazelle merge/index infrastructure as the
first-party language extensions; fixtures reject a parallel BUILD parser, ownership database, or
consumer-side generated helper manifest.
Capability-tag tests prove an opted-out target creates no corresponding action while its
generated config hints remain valid and other capabilities continue independently. They
cover exactly `no-lint`, `no-typecheck`, `no-format`, and `no-audit`, prove combined or
tool-specific lookalike tags have no `rules_dx` effect, and prove each recognized tag affects
only its named capability.

Quality-source fixtures prove Python wrappers need no tag and custom rules integrate solely by
returning `QualitySourcesInfo`. Generic aspects do not inspect arbitrary source/dependency
attributes, rule names, raw extensions, or `DefaultInfo` outputs. Names such as `lint-python`
and `format-javascript` do not change applicability. Exactly `no-lint`, `no-typecheck`,
`no-format`, and `no-audit` retain their capability opt-out behavior.

`QualitySourcesInfo` fixtures cover custom-rule construction, wrapper emission, tested upstream
normalization, zero, one, and many semantic file classes, and deterministic map/depset order.
Negative cases reject unknown class IDs, non-depset values, empty class entries, directories,
generated/external files, registry-inadmissible class/file combinations, duplicate paths across
classes, and attempts to carry transitive/config/tool/capability data.

Owner-context fixtures put one `.h` path in a C owner and a C++ owner. They prove both valid
classifications and applicable pipelines are preserved, identical final candidates deduplicate,
different final candidates reject the shared path, and `no-lint` on one owner suppresses only
that owner's lint context.

Mixed-class fixtures expose JavaScript, TypeScript, JSON, and CSS sources in one target. They
prove format creates one target/capability pipeline, Prettier and Biome receive their exact
possibly overlapping subsets, ESLint and Stylelint receive their distinct lint subsets, and an
adapter with no effective source creates no stage, process, result, or tool fetch. A stage's
class membership and source paths remain fixed throughout convergence.
Source-audit fixtures give each qualified source auditor a mixed-class target and prove its
independent target/tool action receives only its effective subset under the
[audit applicability contract](../cli/commands/audit-update-bazel.md#dx-audit), without assuming
Python-only applicability or a Python runtime. An empty source-audit intersection creates no
action or tool fetch, and native exclusions operate only within the precomputed subset.

Golden tests compare a declared native config under `rules_dx` with a direct hermetic tool
invocation using the same config. They prove no `rules_dx` behavioral preset is merged and
that adapter-added transport flags affect only result encoding, explicit paths,
determinism, and check/fix operation. Any unavoidable tool-specific semantic difference is
documented and tested.

No-config golden tests compare each adapter with the pinned tool's upstream config-free
behavior while holding transport settings equal, including Vale's config-required failure rather
than an assumed successful default run. They prove `rules_dx` adds no rules, style, severity, or
behavioral preset; only documented hermetic transport controls and actionable error normalization may differ.

Environment-parity tests prove every selected quality adapter exports the exact executable,
runtime, and package closure used by its Bazel actions through `.dx/bin`, including a selected
tool with no applicable source target. Disabled and unselected tools are absent. Managed
commands plus checked-in native configs match action versions; ambient editor installations
receive no parity guarantee.

Quality-command naming tests require conventional upstream names and cover case-sensitive and
case-insensitive host normalization. Every collision reports all claimant tools and fails
configuration; no prefix, suffix, declaration-order winner, or silent renaming is accepted.

Raw-tool tests compare `.dx/bin` invocations with the same managed upstream executable outside
the environment projection. Arguments, working directory, native config discovery, streams,
signals, and exit status pass through unchanged. Bazel-only transport flags, VCS-ignore
isolation, machine-result normalization, and network policy are absent from direct commands.

Python policy tests follow the [curated baseline](../tools/tool-baseline.md#curated-differences):
Ruff and pydoclint under lint, Ty under typecheck, and Ruff as the sole formatter.
Audit selection and qualification remain separate under the
[audit contract](../cli/commands/audit-update-bazel.md#dx-audit), not a prerequisite for the
[Python acquisition/cohort proof](../testing/tools.md#managed-tool-acquisition).
Flake8 and pylint remain absent unless explicitly added.
Ty fix tests verify converged `--fix` replacements and prove
`--add-ignore` is never used by the default workflow. Convergence orchestration tests verify
that applied stable fixes with no terminal findings succeed, unresolved
findings fail, report-only `tsc` findings fail, apply validation failures apply
nothing to the rejected path while valid unrelated paths apply, and no automatic post-apply
typecheck invocation occurs. Incomplete collection or global result-envelope failure writes
nothing anywhere.

JavaScript/TypeScript policy tests require Biome as the default linter and
formatter, with Prettier formatter and ESLint linter support available by override.
Biome/Prettier composition tests cover identical output, compatible non-overlapping edits,
one formatter enabling a later formatter change, convergence to a common fixed point, direct
and longer oscillations, iteration limits, and deterministic measured ruleset stage order
independent of user declaration or result arrival order. Permutation benchmarks select order
from terminal correctness/convergence first, then rounds, process starts, and wall time.
Fixtures prove the fixed ten-round cross-tool bound, no eleventh round after a changing tenth,
and earlier repeated-state termination.

Release tests diff curated policy manifests against the
[default lifecycle policy](../tools/tool-baseline.md#default-lifecycle-direction).
Minor-release lint/audit additions require compatibility qualification, explicit release notes,
and a tested override reproducing the prior lint/audit set. Negative fixtures reject additions
missing any of that evidence, default removals (including replacements presented as additions),
default formatter-set or formatter-ownership changes, and otherwise breaking consumer workflows
in a minor release, even when an override exists. Default removals and default formatter-set changes
require a major release. Normal tool-version behavior drift within the pinned set follows the exact-pin
currency policy owned by [Tool Acquisition](../tools/tool-acquisition.md).

## Cache Correctness

For each check kind, capture action graphs and execution records before and after
controlled changes:

| Change | Ruff expectation | Ty expectation |
| --- | --- | --- |
| One direct source | Every owning lint/format pipeline containing the source misses | Every typecheck boundary containing the source misses |
| Transitive Python dependency | Unaffected unless it is a direct Ruff input | Every consuming boundary misses |
| Shared Ruff config | All consuming lint/format pipelines miss | No effect unless shared with Ty |
| Ty config | No effect | All consuming Ty boundaries miss |
| Tool version/platform | All pipelines using that tool/platform miss | All pipelines using that tool/platform miss |
| Unrelated file | No miss | No miss |
| Aggregate membership only | Existing check action keys unchanged | Existing check action keys unchanged |

Equivalent cache cases apply to every adapter. Pipeline tests additionally verify that changing
one selected tool, native config, stage-order policy, or runner invalidates the complete
affected target/capability action but no unrelated target/capability action. Formatter tests
verify that changing the active formatter set invalidates only affected pipelines and that the
apply step itself never changes Bazel action keys.
Changing one target's effective class membership, an applicable adapter's supported-class
manifest, or the exact stage source subset invalidates that target/capability pipeline. Adding
a class unsupported by every selected adapter for that capability, or changing an unselected
adapter, leaves its pipeline key unchanged.

Verification uses `aquery` to inspect declared inputs/action shape and Bazel
execution logs or a controlled remote cache to distinguish executed actions from
cache hits. Tests compare clean output bases and, where available, separate machines
or containers. A warm local no-op alone is not a cache test.

## Determinism

- Reorder equivalent source and dependency declarations and compare action arguments
  and keys where semantic order is irrelevant.
- Run from different absolute checkout paths and compare normalized outputs where
  Bazel permits.
- Randomize query result order in CLI tests and expect identical Bazel argv.
- Declare one source in multiple owners and verify every relevant Ruff action, and no
  unrelated action, misses after an edit.
- Verify locale, timezone, home directory, and PATH do not alter check actions.
- Randomize normalized report/replacement ordering and require identical manifests.
- Reorder user tool-selection lists and randomize result arrival; require identical ruleset-
  ordered stages, convergence state, diagnostics, and original-to-final replacements.
- Randomize `QualitySourcesInfo` map insertion and depset traversal order; require identical
  canonical class maps, per-stage class/source lists, action keys, and results.
- Compare viable permutations for each selected multi-tool set. Reject incorrect, divergent,
  oscillating, and unjustifiably different terminal results; rank equivalent correct orders by
  non-convergence count, rounds, process starts, then measured wall time.
- Exercise two-stage and longer cycles, including a full round whose end bytes equal its start
  after intermediate changes; require oscillation rather than false stability.

## Apply Safety

- Validate source digests before writing and reject stale outputs.
- Validate each stage's byte-range edits against its current virtual snapshot before applying
  them; reject malformed, overlapping, digest-mismatched, unsorted, or invalid stage output.
- Require a complete no-change round, detect every repeated changed state, enforce the fixed
  ten-round limit, and emit only one original-to-stable candidate. Prove a changing tenth round
  fails without an eleventh invocation and native tool-internal passes count as one stage.
- Require identical final bytes across owner/configuration pipeline results; do not merge
  otherwise compatible edits from separate contexts.
- Reject invalid UTF-8 source/replacement bytes, same-offset edits, insertions inside replaced
  ranges, no-op edits/candidates, and quality-originated file creates.
- Apply each selected file atomically and independently after complete result-envelope
  validation; one rejected path does not block valid unrelated paths.
- Cover insertion, deletion, adjacent edits, multibyte source boundaries, and a
  formatter's full-file edit.
- Preserve file modes and documented newline behavior.
- Verify interruption cannot leave a partially written file and may leave only complete
  earlier path commits according to deterministic path order.
- Do not rerun Bazel after lint, typecheck, or format application; terminal pipeline findings
  and per-file apply failures determine the current invocation status.
- Reject incomplete collection or invalid result envelopes before any path mutation begins.
- In JSON check and default modes, emit one deterministic exact `change` event per valid
  candidate path. Check mode performs no writes; default mode emits the change before its
  terminal mutation. Reconstruct each candidate from its digest, UTF-8 byte ranges, and
  replacement strings and require byte-for-byte equality with default mode's planned input.
- Render complete deterministic diff-mode patches from that same edit set in check and default
  modes without rerunning tools or truncating replacement content. A rejected default-mode
  mutation remains in the intended patch while stderr and exit status report rejection.
- Fail check mode on any proposed change independently of diagnostic severity, and prove
  direct Bazel evaluators enforce the same replacement-presence rule.
- Emit `applied` and `not_applied` outcomes together for mixed per-file results and fail the
  command when any path is rejected.
- Run mutation fixtures with tracked, modified, staged, and untracked inputs and verify
  behavior depends on current bytes and source digests rather than Git status.

## Tool Parity

For the [dependency-check requirement](../product/scope.md#unused-dependencies), each supported
language needs separate consumer fixtures for lockfile consistency and declared-dependency usage.
A stale lockfile must fail consistency even when every declaration is used; a consistent lockfile
with an unused declaration must pass consistency but fail usage. A consistent lockfile with all
declarations used must pass both, even when newer compatible releases exist. Verify missing/stale
entries against upstream lock semantics and that neither test mutates manifests or locks.
Run lockfile consistency with network access denied after declared inputs are provisioned, without
an undeclared package-manager cache. Both valid and stale-lock cases must produce the expected
result offline; missing required metadata must fail actionably, not skip validation or pass. Verify
the test does not query live registries for newer releases. Qualify exact offline routes under O46.
Run the generated tests through both `bazel test //...` and bare `dx test`, proving each check is
included without opt-in, propagates failures, and is independently runnable by label. Verify no
default `manual` exclusion hides either check and unrelated foundations remain inactive.
Include necessary transitive packages and shared-workspace usage to prevent target-local false
positives. Verify a legitimate non-import use can pass through an explicit dependency-scoped
exception with an explanatory reason, while an unrelated unused declaration still fails and the
exception does not waive lockfile consistency. Missing reasons must fail validation. Qualify native
configuration, reason validation, non-import recognition, and ecosystem scopes under O46.
Verify exceptions for removed dependencies and exceptions that no longer suppress a finding fail
as obsolete, including after a checker upgrade recognizes legitimate usage. A still-needed explained
exception must continue to pass. Use the same supported-configuration scope as usage analysis and
verify validation reports obsolete entries without deleting them.
Include dependencies used only by another declared supported platform or an inactive optional
feature: they must pass usage without exceptions when checked on the current host/default
configuration. An unused optional or platform-specific declaration must still fail. Qualify the
configuration inputs and analysis route without requiring foreign-platform binary execution.
Where the ecosystem distinguishes dependency categories, a consistent lock with a production
declaration used only by tests or build tooling must fail the usage test with a category error.
Correctly categorized and legitimate multi-category usage must pass. Include category-relevant
usage on another supported configuration and verify diagnostics do not mutate declarations or locks.
Do not add a duplicate unused-Bazel-edge test; generated edge
maintenance remains covered by the existing [generation tests](../testing/generation.md).

Every required entry in [First-Release Tool Baseline](../tools/tool-baseline.md), including mandatory
curated expansion under [First-Release Admission](../product/scope.md#first-release-admission)
and O46 in the [open-decision register](../open-decisions.md), requires a fixture using
the tool's native configuration, passing and failing diagnostics, supported platform
coverage, and applicable fix/format output. A generated parity manifest must fail CI
when a required entry lacks its tests. Swift and SwiftFormat are excluded from v1 by user
scope decision and are not parity requirements; see
[Swift Feasibility](../tools/tool-baseline.md#swift-feasibility).

Tool-update tests regenerate versions and checksums, reject missing platform
artifacts, and run the affected adapter suite. Scheduled automation proposes changes
but does not bypass review or tests.

Equivalent update checks cover every repository-managed Bazel module, Rust crate and
toolchain, and bootstrap dependency. CI rejects floating versions and pending stable
updates without an approved compatibility exception.

Cross-cutting fixture, remote, and acceptance-evidence requirements remain in
[Testing Strategy](../testing/README.md). Acquisition, laziness, and platform requirements
are in [Tool And Platform Testing](../testing/tools.md).
