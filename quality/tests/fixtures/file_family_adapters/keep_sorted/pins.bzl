"""keep_sorted check plus fix wiring.

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`.
"""

KEEP_SORTED_VERSION = "v0.10.0"
KEEP_SORTED_ARTIFACT = "standalone checksummed release artifact; check-only"
KEEP_SORTED_CHECK = "keep-sorted text"
KEEP_SORTED_FIX = "check-only"
FILE_FAMILY_PROOF = "bazel build //quality/tests/fixtures/file_family_adapters/keep_sorted:corpus_starlark"
KEEP_SORTED_REJECTED = "ambient discovery rejected; auto-supplied preset rejected"
