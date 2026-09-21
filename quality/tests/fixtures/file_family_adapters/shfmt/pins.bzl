"""shfmt check plus fix wiring.

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`.
"""

SHFMT_VERSION = "v3.12.0"
SHFMT_ARTIFACT = "standalone checksummed release artifact"
SHFMT_CHECK = "shfmt -d"
SHFMT_FIX = "shfmt -w"
FILE_FAMILY_PROOF = "bazel build //quality/tests/fixtures/file_family_adapters/shfmt:corpus_starlark"
SHFMT_REJECTED = "ambient discovery rejected; auto-supplied preset rejected"
