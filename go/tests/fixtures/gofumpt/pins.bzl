"""Gofumpt check plus fix wiring.

"""

GOFUMPT_VERSION = "v0.11.0"

GOFUMPT_ARTIFACT = "standalone checksummed release artifact; strict superset of gofmt"

GOFUMPT_CHECK = "gofumpt -d with unified diff on stdout (exit 0 clean with empty diff, exit 0 with diff headers when dirty)"
GOFUMPT_FIX = "gofumpt -w in-place rewrite (re-read on exit 0, keep input otherwise)"
GOFUMPT_CONFIG_POLICY = "strict superset of gofmt; whole-file rewrite with check/diff mode, no rule-set selection, no config file"

GO_FIXTURE_HELLO = "//go/tests/fixtures/hello:hello_test"

GOFUMPT_REJECTED = "gofmt-only scope rejected; exit-code-only classification rejected; auto-supplied preset rejected"
