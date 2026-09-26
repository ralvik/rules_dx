"""shellcheck check plus fix wiring."""

SHELLCHECK_VERSION = "v0.11.0"
SHELLCHECK_ARTIFACT = "standalone checksummed release artifact"
SHELLCHECK_CHECK = "shellcheck --format=gcc"
SHELLCHECK_FIX = "check-only"
FILE_FAMILY_PROOF = "bazel build //quality/tests/fixtures/file_family_adapters/shellcheck:corpus_starlark"
SHELLCHECK_REJECTED = "ambient discovery rejected; auto-supplied preset rejected"
