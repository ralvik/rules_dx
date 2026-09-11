# `dx generate`

## Invocation And Scope

`dx generate` runs the canonical `//dx:generate` Gazelle workflow. It defaults to
repository-wide operation and additionally accepts an explicit v1 scope of zero or more
paths, labels, or target patterns selecting the Gazelle subtree to refresh. With no scope
it refreshes the repository. It may create or modify Gazelle-maintained `BUILD` and `BUILD.bazel` files
within the selected scope. Exact scope syntax and scoped freshness semantics are frozen
under [O48](../../open-decisions.md).

V1 scope resolution reuses
[target resolution](../target-resolution.md) verbatim. Empty scope refreshes `//...`;
labels and patterns pass through after syntax and workspace validation with
external-repository scopes rejected; files resolve to depth-1 owners via unconfigured
`bazel query` with no-owner, generated-excluded, not-found, and not-a-package errors;
directories map to recursive `//path/...` patterns. Scoped `--check` runs the same
non-mutating workflow over the selected subtree with the freshness boundary at the
scope edge: a stale or missing Gazelle-maintained file in scope fails; staleness
outside the selected scope never fails the check. The versioned run manifest records the resolved run
scope, and each edit/write-outcome/completion/ignored-import record carries its owning
scope element, so scoped refreshes produce scope-attributed records consumable without
rereading the workspace. Gazelle traverses and merges only packages selected by the
resolved patterns: it preserves every package outside that set and all hand-authored or
kept content inside it under the common merge contract. A directory pattern includes
that package and its descendants; an exact label or file-owner scope includes its owning
package only. Scoped-selection fixtures
(executable against the resolver mapping): empty scope resolves `//...`; label and
pattern scopes pass through after validation with external-repository rejection; file
scopes resolve to depth-1 owners with the no-owner/generated-excluded/not-found/
not-a-package errors; directory scopes map to `//path/...`; freshness fixtures prove
in-scope staleness fails and out-of-scope staleness passes.

The command exposes no Gazelle application arguments. Arguments after `--` are Bazel command
options placed before `//dx:generate`, not arguments passed to Gazelle. Advanced users may invoke
the target or Gazelle directly, but that is outside the `dx generate` contract.

Generation does not require an existing application target or ecosystem manifest when supported
checked-in sources contain only local or standard-library dependencies. Manifests and lockfiles
remain authoritative when project or external dependency metadata is needed. Generation updates
the declared graph; it does not build, test, analyze, resolve package versions for, or execute the
generated targets.

## Generation Contracts

The command delegates BUILD semantics to Gazelle. The authoritative generation contracts are:

- [Common generation](../../generation/common.md): resolution, exceptions, ownership, naming,
  merge and lifecycle behavior, resources, executable entries, and sidecars.
- [Rust](../../generation/rust.md): crates, Cargo metadata, tests, examples, benchmarks, and build
  scripts.
- [Python](../../generation/python.md): sources, stubs, tests, and uv dependency scope.
- [JavaScript and TypeScript](../../generation/javascript-typescript.md): core sources, pnpm scope,
  runtime loads, and tests.
- [Framework adapters](../../generation/framework-adapters.md): Vue, Svelte, Astro, and MDX
  boundaries and support gates.

Native quality configuration can create Bazel package boundaries and generated config bindings as
defined by [Native Configuration](../../quality/native-configuration.md). Removing the final
generated rule does not delete the existing BUILD file or collapse its package boundary.

## Modes

Default mode runs the mutating canonical workflow with Gazelle's native merge and write behavior.

`dx generate --check` runs the non-mutating mode of that same workflow. It writes no workspace file
and succeeds only when default mode would make no change. A stale or missing Gazelle-maintained BUILD
file is therefore a check failure.

`--output text` reports concise affected paths and applicable ignored-import notices. Bazel and
Gazelle diagnostics retain the common stream behavior.

`--output diff` reserves stdout for the validated intended unified patch. It suppresses summaries,
notices, and normalized diagnostics, does not alter whether the selected mode writes, and does not
claim that the patch is the final workspace state after a failed default-mode run.

`--output json` reserves stdout for NDJSON. In check mode, each calculated BUILD-file create or
modification is one exact `change` event and no `mutation` event is emitted. In default mode, the
same exact changes are followed by terminal file-level mutation records: `applied` confirms a
completed write and `not_applied` a failed attempt. New files carry complete content; modifications
carry the original digest and exact byte-range replacements.

The complete event schema, ordering, stream ownership, aggregate counts, exit behavior, and
diff construction are defined by the [Output Protocol](../output-protocol.md).

## Notices

An effective `# gazelle:dx_ignore_import` exception produces one `ignored_import` notice for each
distinct source path, language, and exact ignored literal dependency. Notices are shown in text and
JSON modes, do not by themselves fail the command, and are suppressed in diff mode. Directive
syntax, validation, precedence, and resolution behavior belong to the
[common generation contract](../../generation/common.md#resolution).

## Result And Failure Semantics

The canonical execution produces one private declared versioned result manifest artifact
containing exact edits, write outcomes in default mode, completion state, and ignored-import
audit records, per [O13](../../open-decisions.md) and scoped selection per
[O48](../../open-decisions.md). The manifest schema, dispatch, and canonical-target wiring remain
pending O13. Affected-path,
change, mutation, diff, and ignored-import output comes from that manifest; rendering does not rerun
Gazelle.

A complete check-mode manifest is required before any change event or patch is emitted. Incomplete
check-mode generation emits no changes or mutations and no patch.

Default generation preserves truthful partial reporting. A structurally valid manifest that ends
after a late workflow failure may report only its validated attempted prefix: its `change` events,
terminal `applied` or `not_applied` mutation events, and diff entries remain reportable before the
operational error. `results_complete` is false because this prefix is not a repository-wide result.
A malformed or contradictory manifest fails closed and emits no change or mutation events, even if
Gazelle changed workspace files before manifest validation failed.

`dx generate` does not promise repository-wide atomic BUILD edits. Gazelle writes sequentially using
its native behavior; the CLI does not stage, prevalidate, or roll back the complete edit set. If a
later generation or write fails, earlier confirmed writes and their `applied` events remain, and the
command returns the Gazelle failure. Diff output can include an attempted change whose write later
failed.

## CLI Boundary

The CLI does not parse or write BUILD syntax, infer source ownership, scan ownership markers,
classify externally generated files, preflight individual Gazelle edits, or override Gazelle
directives and merge decisions. It does not create a temporary mutating tree, compare before and
after trees, derive structured changes from human diffs, or inspect Git state. Gazelle and the
language extensions own those decisions and failures.

The consumer source tree receives no generated import map, dependency index, provider inventory,
helper YAML/JSON/TOML file, or public result-manifest sidecar. The result manifest and any derived
indexes remain private Bazel/CLI transport. Pre-existing legacy integration sidecars are ignored:
the command does not read, validate, update, delete, or warn about them, and their bytes do not
affect output or exit status.
