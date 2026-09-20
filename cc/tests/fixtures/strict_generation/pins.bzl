"""Strict generation pins.

Contract: `docs/native-toolchains.md#coverage-generation-and-ide-gaps`,
`docs/native-toolchains.md#qualification-questions-and-delivery`,
`docs/generation/common.md#resolution`, `docs/generation/common.md#ownership-and-naming`.
Fixture: `cc/tests/fixtures/strict_generation/` via
`bazel run //tools/ci:strict_generation_qualification`.
"""

# Inspected generation identity: gazelle_cc v0.6.0 at the pinned commit is
# the reuse candidate, not a drop-in implementation of the accepted contracts.
GAZELLE_CC_VERSION = "v0.6.0"
GAZELLE_CC_COMMIT = "50dbcbcfd9199c19a50522695c568b9380caabe5"
GAZELLE_CC_NOTE = "Strict resolution and ownership conformance remain incomplete by configuration alone"

# Quoted includes: every `#include "path/to/header.h"` contributes its
# basename (`cc/tests/fixtures/strict_generation/strict.h` -> `strict.h`),
# which matches the owning library's indexed header basename. Quoted
# identities resolve strictly or fail generation.
QUOTED_INCLUDE_NOTE = "every quoted include contributes its basename and resolves strictly or fails generation"
QUOTED_INCLUDE_EXAMPLE = "cc/tests/fixtures/strict_generation/strict.h -> strict.h"

# Angle includes: `#include <vector>` is toolchain-provided and never
# produces an edge; the pinned rules_cc toolchain stays authoritative at
# execution time, so no standard-library manifest is needed.
ANGLE_INCLUDE_NOTE = "angle includes are toolchain-provided and never produce an edge"
ANGLE_INCLUDE_EXAMPLES = ["<string>", "<vector>"]

# Ambiguous includes: two libraries owning the same header basename are
# ambiguous and fail resolution; owners add an exact `# gazelle:resolve`
# mapping or rename. First-candidate selection stays rejected.
AMBIGUOUS_NOTE = "two libraries owning the same header basename are ambiguous and fail resolution"
AMBIGUOUS_REJECTED = "ambiguity can select a first candidate"

# Macro includes: `#include HDR` after `#define HDR "foo.h"` carries no
# literal identity, so it contributes no edge and synthesizes no ignore.
# Generation never evaluates the expression, guesses an edge, or emits a
# notice solely because the identity is computed. Callers keep a
# user-authored Bazel dependency behind `# keep` where a computed form is
# recognized.
MACRO_INCLUDE_NOTE = "macro include contributes no edge and synthesizes no ignore"
MACRO_INCLUDE_EXAMPLE = '#define HDR "foo.h" plus #include HDR contributes no edge'

# Comment and literal inertness: text that looks like an include inside
# comments, string literals, character literals, or raw strings never
# produces a fact. Backslash-newline continuations are joined before
# matching so a split include still counts.
INERT_NOTE = "comments, string literals, character literals, and raw strings are inert"
CONTINUATION_NOTE = "backslash-newline continuations are joined before matching"

# Authoritative dependency metadata: for C/C++ there is no ecosystem
# lockfile. Resolution order is exact `# gazelle:resolve` mapping, then the
# local rule index, then the exact `# gazelle:dx_ignore_import` exception,
# else fail. The gazelle_cc module index is not the consumer's resolved
# graph.
RESOLUTION_ORDER = [
    "exact # gazelle:resolve mapping",
    "local rule index",
    "exact # gazelle:dx_ignore_import exception",
]
RESOLUTION_NOTE = "authoritative metadata is the local rule index plus exact mappings, never the module index"
MODULE_INDEX_NOTE = "its module index is not the consumer's resolved graph"

# Generated headers: headers produced by Bazel actions have no checked-in
# owner, so a literal reference to a generated header basename with no local
# owner fails strict resolution unless an exact `# gazelle:resolve` mapping
# supplies the producing label. Generation never infers a generated producer.
GENERATED_HEADER_NOTE = "generated headers have no checked-in owner and require an exact resolve mapping"
GENERATED_HEADER_REJECTED = "inferred generated producer"

# Test grouping: `*_test.*` files are test-owned and never library sources.
# Library imports index only non-test headers; sources contribute no
# identity. Test-only references ride the handwritten `cc_test`, never the
# production library.
TEST_GROUPING_NOTE = "test-owned sources never enter the generated library"
TEST_SUFFIX = "_test"

# Assembly dialects: `.s`/`.S`/`.asm` plus other non-`.c`/`.cc`/`.cpp`/`.cxx`
# sources are undiscovered and stay handwritten. Assembly tool routing needs
# declared inputs; discovery alone is not an assembly proof.
ASSEMBLY_NOTE = "assembly sources are undiscovered and stay handwritten"
ASSEMBLY_EXTS = [".s", ".S", ".asm"]
SOURCE_EXTS = [".c", ".cc", ".cpp", ".cxx"]
HEADER_EXTS = [".h", ".hh", ".hpp", ".hxx"]

# Named modules and PCH: `import foo;` statements, `.cppm`/`.ixx` module
# interfaces, header units, and precompiled headers have no complete
# generation route. They stay handwritten and explicitly assessed before
# admission; compiler flags alone are not complete support.
MODULE_PCH_NOTE = "named C++ modules, header units and PCH have no complete generation route and stay handwritten"
MODULE_PCH_REJECTED = "compiler flags as complete support"

# Union of literal includes: a literal reference contributes an ordinary
# unconditional edge even under `#ifdef`, platform checks, or exception
# handling. The extension collects the deduplicated union per target and
# derives no `select()`, feature semantics, or platform selection. The
# preprocessor-derived platform selections are not the approved Go-only
# generation exception.
UNION_NOTE = "generated dependencies are the deduplicated union of literal identities"
UNION_REJECTED = "preprocessor-derived platform selections"

# Names: one directory holds one reusable `cc_library` named after the
# directory basename, normalized (ASCII letters/digits plus internal
# underscores, runs of other characters collapse to one underscore, edge
# underscores trimmed, empty rejected). Same-package normalized-name
# collisions fail with every claimant; no affix is invented.
NAMING_NOTE = "basename-derived names normalize deterministically and collisions fail with every claimant"

# Merge and lifecycle: the extension uses desired plus empty-rule APIs with
# explicit owned attributes, matches conservatively, preserves `# keep` and
# unfamiliar user content, leaves BUILD syntax to Gazelle, emits a matching
# empty rule when the last owning source disappears, and never deletes a
# BUILD file. A rename is guarded stale-target removal plus generation.
LIFECYCLE_NOTE = "stale generated targets clean through normal merge with keep protection"
LIFECYCLE_REJECTED = "deleting a BUILD file"

# Bounded strictness: unresolved imports fail with an actionable diagnostic,
# ambiguous imports fail with every resolver, stale ignores fail as obsolete
# detection, mapping-plus-ignore conflicts fail, kind mismatches fail, and
# `main`-defining library sources fail instead of inferring a thin binary.
# Reuse is parser plus generation capability with output adaptation only:
# log parsing plus a replacement C++ preprocessor stay rejected.
STRICT_NOTE = "every unknown or ambiguous literal reference fails with actionable context"
STRICT_REJECTED = [
    "unknown angle includes may disappear",
    "missing module mappings can warn and omit edges",
    "log parsing",
    "a replacement C++ preprocessor",
]

# Rejected substitutes per the issue alternatives: loose generation.
REJECTED_ALTERNATIVES = [
    "loose generation",
    "unresolved-dependency errors cover quoted includes only by configuration alone",
    "ambiguity can select a first candidate",
    "missing module mappings can warn and omit edges",
    "unknown angle includes may disappear",
    "log parsing",
    "a replacement C++ preprocessor",
    "compiler flags as complete support",
    "inferred generated producer",
    "preprocessor-derived platform selections",
    "deleting a BUILD file",
]

# Live proof labels: the strict lib plus test below prove the buildable
# strict shape on the seed host; the Go unit suites prove the negative
# strict shapes hermetically without checking in failing BUILD graphs.
STRICT_GENERATION_FIXTURE_LIB = "//cc/tests/fixtures/strict_generation:strict"
STRICT_GENERATION_FIXTURE_TEST = "//cc/tests/fixtures/strict_generation:strict_test"
STRICT_GENERATION_FIXTURE_CORPUS = "//cc/tests/fixtures/strict_generation:corpus_starlark"
STRICT_GENERATION_CC_SUITE = "//gazelle/cc:cc_test"
STRICT_GENERATION_GOLDEN_SUITE = "//gazelle/cc:generation_test"
