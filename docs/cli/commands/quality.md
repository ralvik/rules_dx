# Quality Commands

Explicit scope positionals accept Bazel labels and patterns (`//pkg:target`,
`//pkg/...`); they resolve through Bazel unchanged. Workspace-relative file
paths resolve to every direct source owner through Bazel query, and
directories become recursive patterns, per
[Target Resolution](../target-resolution.md).

## `dx lint`

`dx lint` is explicitly mutating by default. It resolves scope through Bazel,
requests `dx_results` from each target lint pipeline, and atomically applies each agreed stable
file candidate using the Rust apply engine. Selected linters converge over action-local copies;
it does not format or type-check files. Oscillating, iteration-limited, owner-disagreeing, or
stale files remain unchanged, valid unrelated files apply, and any rejected file fails the
command. Incomplete or globally invalid result collection prevents all mutation. Terminal
findings still fail according to `--fail-on`.

With no scope, lint selects `//...`. The aspect still creates actions only for
compatible direct main-workspace non-generated sources. Each stage receives only the intersection of target semantic
file classes, adapter-supported lint classes, and workspace class policy. Ruff receives its
effective Python subset and the target's Gazelle-generated local config
set and applies its native per-file config and exclusion resolution in one target-level
pipeline while forcing `--no-respect-gitignore`. `rules_dx` adds no global ignore layer,
and `dx` does not scan the workspace to construct a source or config list.

All lint, typecheck, format, and source-audit adapters disable Git and other VCS ignore
discovery. Bazel direct ownership, semantic-class applicability, workspace policy, capability,
and declared native-config exclusions are the complete file-selection policy for quality
commands. Native configuration can exclude files from a stage but cannot add undeclared files.

When lint or format changes a native config file, its action may converge virtually under the
currently declared policy, but the CLI does not rerun Bazel or invoke generate after applying
it. The next explicit command observes the new policy and Bazel invalidates consumers.

Lint supports SARIF 2.1.0 reports through the shared `--report` contract. SARIF is an
export of normalized findings, not the live lifecycle protocol. Partial
collection marks the SARIF invocation unsuccessful while retaining validated findings.
Check reports contain all findings; default mutating reports omit successfully fixed findings
and retain remaining or not-applied findings under the common SARIF current-result contract.

`dx lint --check` is non-mutating. It applies the configured lint aspects to the
selected source targets and requests the same cacheable proposed fixes as default mode but
does not apply them. It shows every original finding, including fixable findings, and marks a
finding fixable only when its exact same-file change is guaranteed to resolve it. JSON output
emits one `change` event per changed file with its original
BLAKE3-256 digest, exact half-open UTF-8 byte ranges, and replacement text. Any proposed
change fails check mode independently of remaining diagnostics and `--fail-on`. The CLI
understands the workflow class, not tool internals. CI uses this mode or invokes the same
aspects and output group directly through Bazel.

Check text shows only initial findings. Check JSON preserves initial and stable terminal
findings with explicit snapshot identity. Mutating text shows terminal findings for applied
files and initial findings for rejected files. Diff output contains only the intended patch on
stdout; `dx` emits no `dx`-owned normalized diagnostic or operation summary alongside it.
Raw subprocess passthrough and operational errors on stderr follow the authoritative
[Output Protocol](../output-protocol.md#stream-ownership).

In default JSON mode, lint emits those same exact changes before their terminal mutation
outcomes and emits every original diagnostic with `fixed`, `remaining`, or `not_applied`
resolution. Default human output hides `fixed` diagnostics, shows `remaining` and
`not_applied` diagnostics, and summarizes applied fixes. Diff output renders complete unified diffs
from the same validated edit set in either mode without rerunning lint. A mutating diff
includes every intended change even when a later per-file mutation fails; stderr and process
status report the failure.

## `dx typecheck`

`dx typecheck` is explicitly mutating by default. It resolves scope through Bazel,
requests `dx_results` from each target typecheck pipeline and applies agreed stable candidates
independently per file through the same digest-validated engine as lint. Selected fix-capable
type checkers converge over virtual copies; rejected files remain unchanged while valid
unrelated files apply. Type checkers without fixes contribute diagnostics in each analysis
round but never mutate the virtual source.

With no scope, typecheck selects `//...`. Provider semantic classes, adapter-supported
typecheck classes, and workspace policy determine nonempty stages. Each adapter then applies
exclusions from its explicitly bound native configuration within that subset.

Typecheck pipelines report initial and stable terminal findings with strict binary fixability. After the
per-file apply phase, `dx` derives fixed, remaining, and not-applied resolution from guaranteed
fixability and the file outcome; it does not automatically rerun Ty, `tsc`, or any other
selected type checker. The command fails if any action reports unresolved findings at or
above `--fail-on`, a guaranteed fix is not applied, or replacement validation/application
fails; otherwise it succeeds. Users and CI
may run `dx typecheck --check` separately to validate the resulting workspace in a
new Bazel request.

Ty initially runs per compatible Python target, propagates over dependencies, and
uses provider-derived transitive sources and import roots for resolution. Its
`--fix` behavior produces cacheable proposed replacements. Suppression insertion,
including Ty's `--add-ignore`, is not a normal fix and is excluded. Final Ty action
granularity remains benchmark-gated.

`dx typecheck --check` is non-mutating and requests the same genuine proposed fixes as
default mode. It shows all original findings and marks only guaranteed fixes as fixable. It
emits exact JSON `change` events and fails when any change is proposed or
when remaining diagnostics cross `--fail-on`. CI uses this mode or invokes the same
typecheck aspects and output group directly through Bazel.
Default JSON mode includes the exact attempted changes before mutation outcomes; diff mode
renders all those intended changes as unified diffs regardless of later mutation outcomes.

Typecheck supports SARIF 2.1.0 reports through the shared `--report` contract. Partial
collection marks the SARIF invocation unsuccessful while retaining validated findings.
Check reports contain all findings; default mutating reports omit successfully fixed findings
and retain remaining or not-applied findings.

Lint, typecheck, format, and audit invocations explicitly select canonical
`//dx:config` through
`--@rules_dx//config:workspace=//dx:config`. This matches the committed consumer
`.bazelrc` default while remaining independent from ignored `user.bazelrc`.

After valid scope and workspace-policy resolution, zero active applicable tools is a
silent successful no-op for lint, typecheck, format, and audit. No analyzer process
launches, so no [operation summary](../cli-contract.md#operation-display) is emitted.
Text mode prints nothing for the no-op. NDJSON emits no notice, diagnostic, change, mutation, or report event; required
lifecycle events end with complete zero diagnostic and, for check mode, change counts and
exit code `0`. Invalid tool identifiers, config labels, scopes, providers, or analysis remain
failures.

Here and throughout the quality commands, an applicable tool means a selected adapter with at
least one effective direct source after semantic-class intersection. An adapter with no
effective sources creates no stage, action of its own, or tool fetch.

## `dx format`

`dx format` is explicitly mutating by default. It resolves Bazel-owned source targets, requests
converged format-pipeline results, validates recorded source digests, and atomically applies each
agreed stable candidate per file. It does not apply non-formatting lint fixes. Pipeline order,
fixed source subsets, convergence, and rejection behavior are defined by the
[Quality Action Model](../../quality/action-model.md); this command consumes those results rather
than reimplementing the algorithm. Incomplete or globally invalid result collection writes nothing.

With no scope, format selects `//...`; only checked-in Bazel-owned direct sources included in at
least one selected formatter's effective semantic-file-class subset are eligible.

`dx format --check` applies configured format aspects and requests `dx_results`
with the same replacement edits as default mode, but never writes them. Any proposed change
fails check mode. JSON output contains the exact shared `change` event shape. CI uses this
mode or invokes the same aspects and output group directly through Bazel.
Default JSON mode includes exact formatting changes before mutation outcomes. Diff mode emits
complete unified diffs from those same changes in either mode, without filtering failed
mutation attempts.

Lint, typecheck, format, and source-audit commands collect aspect outputs from the
Bazel Build Event Protocol. They do not discover artifacts by scanning `bazel-out`.
Bazel remains responsible for materializing requested remote outputs.
