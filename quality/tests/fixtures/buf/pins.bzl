"""Buf check plus fix wiring (protobuf format plus lint).

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`.
"""

# Pinned reference version (qualified seed-only under issue #488; living
# at head rejected; digests stay owned under issue #799).
BUF_VERSION = "1.72.0"

# Distribution identity (checksummed native/self-contained artifact route
# as self-contained per-platform binaries with published checksums, no
# target compiler context, execution-platform lazy; digests stay owned
# under issue #799).
BUF_ARTIFACT = "self-contained per-platform binaries with published checksums"

# Invocation shapes (lint JSONL check-only; format whole-file rewrite
# with check/diff mode).
BUF_LINT_CHECK = "buf lint --error-format=json (exit 0 clean, exit 1 with JSONL records when dirty)"
BUF_FORMAT_CHECK = "buf format --diff --exit-code (exit 0 clean, exit 1 with unified diff when dirty)"
BUF_FORMAT_FIX = "buf format --write in-place (re-read on exit 0, keep input otherwise)"
BUF_CONFIG_POLICY = "upstream STANDARD lint set without buf.yaml, native interpretation with buf.yaml; no auto-supplied preset"

# Live proof labels (adapter dispatch owned under issue #799; protobuf
# foundations are not admitted as build/test targets, so the fixture
# pair plus matrix cells are the live proof).
STRUCTURED_FIXTURE_BUF = "//quality/tests/fixtures/buf:corpus_starlark"

# Rejected: SARIF ingestion (no SARIF in 1.71.0), IN_PLACE mutation
# outside sandbox-apply-and-diff, ambient config discovery,
# auto-supplied preset.
BUF_REJECTED = "SARIF ingestion rejected; IN_PLACE patching outside sandbox rejected; ambient buf.yaml discovery rejected; auto-supplied preset rejected"
