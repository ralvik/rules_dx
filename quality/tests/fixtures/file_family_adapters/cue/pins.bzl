"""cue check plus fix wiring."""

CUE_VERSION = "v0.17.1"
CUE_ARTIFACT = "standalone checksummed release artifact; cue fmt whole-file rewrite"
CUE_CHECK = "cue fmt --check --diff"
CUE_FIX = "cue fmt --write"
FILE_FAMILY_PROOF = "bazel build //quality/tests/fixtures/file_family_adapters/cue:corpus_starlark"
CUE_REJECTED = "ambient discovery rejected; auto-supplied preset rejected"
