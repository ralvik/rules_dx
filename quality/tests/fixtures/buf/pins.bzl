"""Buf check plus fix wiring (protobuf format plus lint).

"""

BUF_VERSION = "1.72.0"

BUF_ARTIFACT = "self-contained per-platform binaries with published checksums"

BUF_LINT_CHECK = "buf lint --error-format=json (exit 0 clean, exit 1 with JSONL records when dirty)"
BUF_FORMAT_CHECK = "buf format --diff --exit-code (exit 0 clean, exit 1 with unified diff when dirty)"
BUF_FORMAT_FIX = "buf format --write in-place (re-read on exit 0, keep input otherwise)"
BUF_CONFIG_POLICY = "upstream STANDARD lint set without buf.yaml, native interpretation with buf.yaml; no auto-supplied preset"

STRUCTURED_FIXTURE_BUF = "//quality/tests/fixtures/buf:corpus_starlark"

BUF_REJECTED = "SARIF ingestion rejected; IN_PLACE patching outside sandbox rejected; ambient buf.yaml discovery rejected; auto-supplied preset rejected"
