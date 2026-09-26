"""pkl check plus fix wiring.

"""

PKL_VERSION = "0.32.1"
PKL_ARTIFACT = "standalone checksummed release artifact"
PKL_CHECK = "pkl --check"
PKL_FIX = "pkl --write"
FILE_FAMILY_PROOF = "bazel build //quality/tests/fixtures/file_family_adapters/pkl:corpus_starlark"
PKL_REJECTED = "ambient discovery rejected; auto-supplied preset rejected"
