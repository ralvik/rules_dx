"""rubocop check plus fix wiring.

"""

RUBOCOP_VERSION = "1.91.0"
RUBOCOP_ARTIFACT = "release-assembled Ruby closure member"
RUBOCOP_CHECK = "rubocop --format json"
RUBOCOP_FIX = "check-only"
FILE_FAMILY_PROOF = "bazel build //quality/tests/fixtures/file_family_adapters/rubocop:corpus_starlark"
RUBOCOP_REJECTED = "ambient discovery rejected; auto-supplied preset rejected"
