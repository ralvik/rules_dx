"""jsonnetfmt check plus fix wiring.

"""

JSONNETFMT_VERSION = "v0.22.0"
JSONNETFMT_ARTIFACT = "standalone checksummed release artifact; go-jsonnet rewrite"
JSONNETFMT_CHECK = "jsonnetfmt --test"
JSONNETFMT_FIX = "jsonnetfmt -i"
FILE_FAMILY_PROOF = "bazel build //quality/tests/fixtures/file_family_adapters/jsonnetfmt:corpus_starlark"
JSONNETFMT_REJECTED = "ambient discovery rejected; auto-supplied preset rejected"
