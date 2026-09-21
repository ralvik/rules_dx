"""yamllint check plus fix wiring.

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`.
"""

YAMLLINT_VERSION = "1.38.0"
YAMLLINT_ARTIFACT = "private wheel-only Python graph member"
YAMLLINT_CHECK = "yamllint text"
YAMLLINT_FIX = "check-only"
FILE_FAMILY_PROOF = "bazel build //quality/tests/fixtures/file_family_adapters/yamllint:corpus_starlark"
YAMLLINT_REJECTED = "ambient discovery rejected; auto-supplied preset rejected"
