"""Gofumpt check plus fix wiring.

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`.
"""

# Pinned reference version (qualified seed-only under issue #487; living
# at head rejected; digests stay owned under issue #798).
GOFUMPT_VERSION = "v0.11.0"

# Distribution identity (standalone checksummed release artifact; strict
# superset of gofmt; digests stay owned under issue #798).
GOFUMPT_ARTIFACT = "standalone checksummed release artifact; strict superset of gofmt"

# Invocation shapes (whole-file rewrite with check/diff mode, no rule-set
# selection).
GOFUMPT_CHECK = "gofumpt -d with unified diff on stdout (exit 0 clean with empty diff, exit 0 with diff headers when dirty)"
GOFUMPT_FIX = "gofumpt -w in-place rewrite (re-read on exit 0, keep input otherwise)"
GOFUMPT_CONFIG_POLICY = "strict superset of gofmt; whole-file rewrite with check/diff mode, no rule-set selection, no config file"

# Live proof labels (foundation consumers stay green; adapter dispatch
# owned under issue #798).
GO_FIXTURE_HELLO = "//go/tests/fixtures/hello:hello_test"

# Rejected: gofmt-only scope, -l exit-code-only classification, preset.
GOFUMPT_REJECTED = "gofmt-only scope rejected; exit-code-only classification rejected; auto-supplied preset rejected"
