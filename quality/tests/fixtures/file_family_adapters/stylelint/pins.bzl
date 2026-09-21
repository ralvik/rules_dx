"""stylelint check plus fix wiring.

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`.
"""

STYLELINT_VERSION = "17.14.1"
STYLELINT_ARTIFACT = "private pure-JavaScript graph member"
STYLELINT_CHECK = "stylelint --formatter json"
STYLELINT_FIX = "check-only"
FILE_FAMILY_PROOF = "bazel build //quality/tests/fixtures/file_family_adapters/stylelint:corpus_starlark"
STYLELINT_REJECTED = "ambient discovery rejected; auto-supplied preset rejected"
