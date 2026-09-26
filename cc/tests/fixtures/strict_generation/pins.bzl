GAZELLE_CC_VERSION = "v0.6.0"
GAZELLE_CC_COMMIT = "50dbcbcfd9199c19a50522695c568b9380caabe5"
GAZELLE_CC_NOTE = "Strict resolution and ownership conformance remain incomplete by configuration alone"

QUOTED_INCLUDE_NOTE = "every quoted include contributes its basename and resolves strictly or fails generation"
QUOTED_INCLUDE_EXAMPLE = "cc/tests/fixtures/strict_generation/strict.h -> strict.h"

ANGLE_INCLUDE_NOTE = "angle includes are toolchain-provided and never produce an edge"
ANGLE_INCLUDE_EXAMPLES = ["<string>", "<vector>"]

AMBIGUOUS_NOTE = "two libraries owning the same header basename are ambiguous and fail resolution"
AMBIGUOUS_REJECTED = "ambiguity can select a first candidate"

MACRO_INCLUDE_NOTE = "macro include contributes no edge and synthesizes no ignore"
MACRO_INCLUDE_EXAMPLE = '#define HDR "foo.h" plus #include HDR contributes no edge'

INERT_NOTE = "comments, string literals, character literals, and raw strings are inert"
CONTINUATION_NOTE = "backslash-newline continuations are joined before matching"

RESOLUTION_ORDER = [
    "exact # gazelle:resolve mapping",
    "local rule index",
    "exact # gazelle:dx_ignore_import exception",
]
RESOLUTION_NOTE = "authoritative metadata is the local rule index plus exact mappings, never the module index"
MODULE_INDEX_NOTE = "its module index is not the consumer's resolved graph"

GENERATED_HEADER_NOTE = "generated headers have no checked-in owner and require an exact resolve mapping"
GENERATED_HEADER_REJECTED = "inferred generated producer"

TEST_GROUPING_NOTE = "test-owned sources never enter the generated library"
TEST_SUFFIX = "_test"

ASSEMBLY_NOTE = "assembly sources are undiscovered and stay handwritten"
ASSEMBLY_EXTS = [".s", ".S", ".asm"]
SOURCE_EXTS = [".c", ".cc", ".cpp", ".cxx"]
HEADER_EXTS = [".h", ".hh", ".hpp", ".hxx"]

MODULE_PCH_NOTE = "named C++ modules, header units and PCH have no complete generation route and stay handwritten"
MODULE_PCH_REJECTED = "compiler flags as complete support"

UNION_NOTE = "generated dependencies are the deduplicated union of literal identities"
UNION_REJECTED = "preprocessor-derived platform selections"

NAMING_NOTE = "basename-derived names normalize deterministically and collisions fail with every claimant"

LIFECYCLE_NOTE = "stale generated targets clean through normal merge with keep protection"
LIFECYCLE_REJECTED = "deleting a BUILD file"

STRICT_NOTE = "every unknown or ambiguous literal reference fails with actionable context"
STRICT_REJECTED = [
    "unknown angle includes may disappear",
    "missing module mappings can warn and omit edges",
    "log parsing",
    "a replacement C++ preprocessor",
]

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

STRICT_GENERATION_FIXTURE_LIB = "//cc/tests/fixtures/strict_generation:strict"
STRICT_GENERATION_FIXTURE_TEST = "//cc/tests/fixtures/strict_generation:strict_test"
STRICT_GENERATION_FIXTURE_CORPUS = "//cc/tests/fixtures/strict_generation:corpus_starlark"
STRICT_GENERATION_CC_SUITE = "//gazelle/cc:cc_test"
STRICT_GENERATION_GOLDEN_SUITE = "//gazelle/cc:generation_test"
