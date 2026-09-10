# Target Resolution

## Goal

Users may provide Bazel labels, target patterns, files, or directories. `dx`
normalizes those inputs into source-owning Bazel targets and then applies the
configured workflow aspects through Bazel. It never parses BUILD files or
constructs source lists.

## Input Classification

- Main-workspace labels and target patterns are passed to Bazel after syntax and
  workspace validation. Workflow commands reject external-repository scopes; internal
  external dependencies may still participate through Bazel's analyzed graph without
  becoming user-facing scope.
- Existing file paths are normalized relative to the workspace, but ownership is
  resolved by Bazel query.
- Directory paths become recursive Bazel target patterns. For example, `src/` maps
  to `//src/...`, including nested packages even when `src/` is not itself a Bazel
  package. Recursion is performed by Bazel, not filesystem traversal. Users may
  pass an explicit pattern such as `//src:all` when they want one exact package.
- Missing or ambiguous inputs fail with actionable diagnostics.

No-scope lint, typecheck, format, and audit commands use the explicit target pattern
`//...`. This is independent of the current directory and does not authorize
filesystem source discovery; aspects and providers determine applicable work.

## File Ownership

Selected strategy ([O44](../open-decisions.md), qualified by M08 WP0 prototypes
against fixture targets): unconfigured `bazel query` only — no `cquery`, no
purpose-built aspect. Ownership of one file is
`kind('rule', rdeps(//..., <file-label>, 1))` at depth exactly 1 over the
main-workspace `//...` universe, one invocation per input file. The file
label uses the nearest enclosing package: `dx` walks from the file's
directory up to the workspace root for the first `BUILD.bazel`/`BUILD`
marker (existence only; contents are never read), so
`pkg/src/deep/a.py` queries as `//pkg:src/deep/a.py`. Every depth-1
rule referrer (including filegroups) is a direct owner; results are
canonicalized, deduplicated, and bytewise sorted, never lexically
disambiguated. Non-package directories fail as not-a-package, workspace-missing
paths as not-found, and query-visible non-source paths as generated-excluded.
Test and coverage mapping is
`kind('.*_test rule', rdeps(//..., set(<owners>)))` with empty mappings as
explicit errors. `select()` over-selection stays conservative.

The implementation milestone prototyped ownership against fixture targets
before this expression became normative. In particular, tests cover source
files named directly in `srcs`, files reached through `filegroup`, generated
sources, aliases, and files with multiple owners.

## Ambiguity Policy

Multiple owners are valid Bazel graph facts. `dx` must not choose one based on
lexical order or naming convention.

For linting, formatting, and their check modes, all relevant direct source owners
may be selected. Bazel deduplicates aspect applications to the same configured
target. Typecheck and source audit use the same all-direct-owners policy so every
configured source context is analyzed. Build selects all direct source owners.
Test and coverage map every direct owner to all transitive reverse-dependent test
targets in the main-workspace `//...` universe. Conceptually, the query is
`tests(rdeps(//..., set(<owner labels>)))`. Returned tests are canonicalized,
deduplicated, and all are selected; no distance limit or package-location heuristic
is applied. A source-owning library is never passed directly to `bazel test` or
`bazel coverage` merely because it owns the file.

This mapping intentionally uses Bazel's unconfigured query graph. It may
conservatively include tests reachable only through inactive `select()` branches,
but it must not prune graph-visible reverse dependencies using CLI heuristics.
Configuration-aware reverse-dependency mapping is deferred unless measured
over-selection justifies its additional complexity.

Resolution queries load the workspace `.bazelrc` and receive only user options the
specific query command natively supports. Other configuration-affecting build
options apply to the final workflow only. Their presence neither changes the query
into a configured resolver nor makes path scope invalid.

Explicit test labels and target patterns bypass source-to-test inference and go
directly to Bazel. A file with no direct owner is an ownership error. A file whose
owners have no reverse-dependent tests is a distinct empty-mapping error that
suggests an explicit test label or pattern; it is not silent success.

No owner is a hard error. Diagnostics should suggest adding the file to an
appropriate Bazel target or supplying an explicit target label.

## From Owner to Workflow

The CLI maps each verb to a repository-configured aspect and requests the common
`dx_results` output group. Check and mutating modes request equivalent supported
replacements in the same unified results. Check mode validates and exposes them without
writing; mutating mode applies them.

The stable entry points are `@rules_dx//lint:defs.bzl%lint_aspect`,
`@rules_dx//typecheck:defs.bzl%typecheck_aspect`,
`@rules_dx//format:defs.bzl%format_aspect`, and
`@rules_dx//audit:defs.bzl%audit_aspect`. Tool-specific aspects remain internal.

The target tags `no-lint`, `no-typecheck`, `no-format`, and `no-audit` opt a target out of the
corresponding capability. The corresponding aspect creates no action for that target; other
capabilities remain independent. There is no combined quality opt-out tag. Tags do not select
individual tools, change native-config resolution, or carry source ownership. `dx_results`
is the sole stable public output group exposed by the capability aspects.

There are no positive quality-classification tags. Conventional `rules_dx` wrappers, custom rules,
and explicitly supported upstream adapters expose source facts through the authoritative
[Quality Sources](../quality/quality-sources.md) contract. Target resolution consumes those analyzed
facts; it does not redefine provider fields, inspect arbitrary attributes, infer classes from raw
extensions, or treat `DefaultInfo` outputs as writable sources.

## Determinism

Query expressions use repository-relative normalized paths. Returned labels are
canonicalized, deduplicated, and bytewise sorted before command construction.
User-supplied Bazel arguments retain their original order. Empty selection is
reported explicitly and is not silently treated as success unless a command
defines that behavior.

## Query Safety

The CLI constructs query expressions as argument vectors and escapes label values
according to Bazel query syntax. It does not interpolate raw user text into a
shell. Large scope sets use one bounded query per input file; any batching
strategy must preserve deterministic output and remain inspectable.

## Acceptance Cases

- Resolve a file with exactly one direct Python owner.
- Resolve a directory containing multiple Bazel packages without filesystem
  source enumeration.
- Map the workspace root to `//...` and a nested directory to its recursive
  `//path/...` pattern, independent of whether the directory is itself a package.
- Preserve explicit labels and recursive target patterns.
- Reject explicit external-repository labels and patterns for workflow scopes without
  exposing Bazel canonical repository names in structured output.
- Diagnose zero and multiple owners.
- Select all direct owners for lint, typecheck, format, and source audit, and
  preserve each distinct configured-target context.
- Build every direct owner of a file without arbitrary disambiguation.
- Prove command-specific behavior for building an owner and selecting tests for a
  path owned by a non-test library.
- Prove test and coverage map all owners to all transitive reverse-dependent tests
  in `//...`, deduplicate results, and reject empty mappings rather than treating a
  source owner as a test.
- Cover configurable dependencies and preserve the documented conservative
  over-selection of ordinary query.
- Handle spaces and query-significant characters safely where the platform and
  Bazel permit them.
- Exclude generated artifacts as direct quality subjects and mutation destinations
  while allowing typecheck/compiler actions to consume them as read-only context.
- Apply configured workflow aspects without checker-specific CLI logic.
- Automatically expose semantic file classes from supported wrappers and normalize tested
  upstream provider shapes without generic attribute inspection.
- Accept any number of valid `QualitySourcesInfo` classes, keep one target/capability pipeline,
  and pass each adapter only its effective source subset.
- Reject unsupported source extraction, unknown or inadmissible classes, generated direct
  subjects, and duplicate class ownership while preserving exactly the four capability opt-outs.
- Preserve different valid classifications from multiple owners of one path and apply the normal
  final-byte consensus rule before mutation.
