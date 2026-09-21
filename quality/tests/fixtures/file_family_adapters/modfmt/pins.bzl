"""modfmt check plus fix wiring.

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`.
"""

MODFMT_VERSION = "v0.4.0"
MODFMT_ARTIFACT = "standalone checksummed release artifact from github.com/joshdk/modfmt v0.4.0"
MODFMT_CHECK = "modfmt -d"
MODFMT_FIX = "modfmt -w"
FILE_FAMILY_PROOF = "bazel build //quality/tests/fixtures/file_family_adapters/modfmt:corpus_starlark"
MODFMT_REJECTED = "ambient discovery rejected; auto-supplied preset rejected"
