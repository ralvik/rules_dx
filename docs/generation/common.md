# Common Generation Contract

This contract applies to every first-party language extension. See
[ADR 0015](../decisions/0015-first-party-gazelle-extensions.md) for rationale and the
[generation test matrix](../testing/generation.md#generate-command-and-gazelle-extensions) for
required evidence.

## Implementation Boundary

First-party extensions use Gazelle's standard language, configuration, indexing, resolution,
rule-generation, and merge APIs. They emit desired and empty rules, identify exactly the attributes
they own, match conservatively, preserve `# keep` and unfamiliar user content, and leave BUILD syntax
and writes to Gazelle. They do not implement a BUILD parser, writer, ownership database, compiler,
build rule, or package-manager resolver.

Rust is the first concrete implementation. It must not extract shared language helpers in advance;
helpers are extracted only after later language implementations prove concrete reuse. There is no
universal language model or runtime plugin framework.

Extensions may parse checked-in sources and user-authored project configuration. Authoritative
manifests, lockfiles, package-manager outputs, and public upstream Bazel metadata retain ownership of
dependency identities, versions, features, platforms, and build semantics. Generation may translate
that metadata into imports and labels, but never selects versions, installs packages, invokes an
ambient package manager, reconstructs a lockfile, or edits manifests or lockfiles.

## Resolution

Every recognized dependency reference with a literal identity resolves in this order:

1. Language standard library.
2. Effective `# gazelle:resolve` mapping.
3. Gazelle's local rule index.
4. Dependency active in the target's authoritative locked ecosystem scope.
5. Effective exact `# gazelle:dx_ignore_import` exception.

Unknown or ambiguous references, missing required lock metadata, and dependencies present in a lock
but inactive for the target fail with actionable context. Generation never guesses or omits a label,
emits an unresolved marker, or defers a known graph error to the build. Source-only code with only
standard-library and local references does not require an ecosystem manifest or lockfile.

Production targets use their selected production scope. Tests may additionally use only their
assigned test or development scope. Optional dependencies, extras, groups, features, importers, and
platform variants are admissible only when authoritative configuration already activates them for
that target. Source imports never activate or move them, and IDE/environment-only groups do not
become build scope.

Recognized runtime-load forms follow the same rule. Literal identities are resolved normally;
computed identities require user-authored Bazel dependencies protected by `# keep`. Generation does
not evaluate the expression, guess an edge, synthesize an ignore, or emit a notice solely because an
identity is computed. Arbitrary calls, reflection APIs, and strings are dependencies only when the
language recognizer defines them as such.

A literal reference contributes an ordinary unconditional edge even under a source condition,
platform check, feature gate, or exception handler. The extension collects the union of literal
identities per generated target and does not derive `select()` or feature semantics from source flow.

Go is a narrow exception for declared Go build constraints and platform source selection:
preserve upstream Go/Gazelle platform-aware source and dependency semantics. This does not
permit interpreting arbitrary runtime conditions, inferring resources, or weakening strict resolution.
Exact recognizers and provider mappings remain subject to O46 review.

The ignore directive has the exact form
`# gazelle:dx_ignore_import <source-language> [<import-language>] <import-string>`.

Gazelle directory inheritance and directive precedence determine the effective policy. Matching is
exact, not prefix- or regular-expression-based. Conflicting equally effective mappings, a mapping and
ignore for the same reference, or an ignore that matches no literal reference in its effective
subtree fails generation. This is deliberate obsolete detection: removing the last usage of an
ignored import requires deleting its directive in the same change. Each accepted ignore produces a deterministic `ignored_import` notice but
does not make an otherwise successful command fail. Extensions never generate mappings or ignores.

## Ownership And Naming

Dependency ownership follows source ownership. A reference contributes only to the generated target
that owns its source. Test-only references do not widen a production target; any conventional
test-to-production edge is separate. Ambiguous ownership fails rather than duplicating a source or
attaching it to an unrelated target. Language contracts define whether the ownership unit is a file,
crate, or framework container.

Every recognized test source receives its own independently runnable target; generation does not
replace these with an implicit package, crate, or runner aggregate. Go instead preserves native
package-level test targets, including shared test helpers and `TestMain`; exact internal/external
test-package mappings remain subject to O46 review.

Basename-derived names preserve ASCII letters, digits, and internal underscores, replace each run of
all other characters with one underscore, trim edge underscores, and reject an empty result.
Same-package normalized-name collisions fail with every claimant. Generation does not invent a
language affix or another suffix.

## Merge And Lifecycle

When the last owning source or authoritative manifest declaration disappears, the extension emits a
matching empty rule. Gazelle removes the stale generated target only when normal matching and merge
semantics permit it. `# keep`, unfamiliar user-owned content, and unrelated handwritten rules remain
protected. An edited stale rule that must be preserved is a successful conservative merge, not a
generation error or a request to strip only recognized fields.

A rename is guarded stale-target removal plus generation of the new target. Generation never deletes
an existing `BUILD` or `BUILD.bazel` file, even when it becomes empty.

Normal consumer trees receive no generated integration sidecar such as an import map, dependency
index, provider inventory, or `gazelle_python.yaml`. Legacy sidecars are inert: generation does not
read, validate, update, delete, or warn about them. Derived metadata, when required, is a versioned
declared Bazel artifact outside the source tree and is supplied to the canonical Gazelle workflow.

## Resources

All test and production `data` and semantic resource attributes are user-owned. Extensions do not
interpret file-loading calls, path literals, static-inclusion constructs, framework loaders,
ecosystem resource declarations, or directory conventions to infer, validate, remove, or relocate
resources. They do not resolve producer labels or diagnose missing paths.

Users maintain upstream resource attributes and protect values with normal `# keep` comments where
needed. Resource access alone produces no generation notice, diagnostic, or status change.

## Executable Entries

A checked-in source that is both importable and a recognized executable entry has one reusable
library owner plus one thin binary that depends on it. The library alone owns the source and its
source-derived dependencies; the binary carries only tested entry-point or launcher metadata. If the
pinned upstream rules cannot preserve execution semantics in this shape, support remains blocked
rather than duplicating ownership or falling back to a binary-only owner.

Every recognized entry receives a thin binary over the reusable owner. An authoritative manifest
mapping supplies its name when present; otherwise the language's tested package-relative source-path
normalization applies. Duplicate or ambiguous manifest mappings and normalized-name collisions fail
with all claimants. Generation never chooses a default entry, drops one, or invents a suffix.
`dx run` file scope resolves to this owning library plus thin wrapper with strict single-target execution.
