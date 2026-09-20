"""Python source-audit split pins (issue #613).

Contract: `docs/product/support-matrix.md`,
`docs/tools/tool-baseline.md#curated-differences`,
`docs/quality/action-model.md#capability-semantics`,
`docs/decisions/0010-python-foundation.md`,
`docs/testing/verification-matrix.md`.

Fixture: `python/tests/fixtures/python_audit/` via
`bazel run //tools/ci:python_audit_qualification`.

Splits Python source-audit tooling out of the quality family taxonomy
(issue #512 stays taxonomy-only): Python source audit carries no
selected tool in v1 with fixture evidence. Docs stay in place; this
harness pins execution. Seed only: no platform plus consumer plus
release claim, no Supported claim. Backends stay provisional.
"""

# Source-audit disposition: no selected tool in v1; curated audit stays
# empty with explicit disablement for the python family.
PYTHON_AUDIT_EMPTY = "curated audit stays empty with explicit disablement"
PYTHON_AUDIT_FAMILY_EMPTY = "python family audit stays empty with explicit disablement"
PYTHON_AUDIT_BANDIT_EXCLUDED = "Bandit excluded from v1 by ADR 0019"
PYTHON_AUDIT_SECRETS_SEPARATE = "secrets family rides Gitleaks detect with redact plus SARIF"

# Unaffected Python quality: lint plus format plus typecheck stay owned
# by their delivered verification layers, never by this tracker.
PYTHON_LINT = "python family lint pydoclint plus ruff"
PYTHON_FORMAT = "python family format ruff"
PYTHON_TYPECHECK = "python family typecheck ty"
PYTHON_BASELINE_OPTINS = "flake8 plus pylint stay baseline opt-ins"

# No audit adapter claim: no audit capability rides any real adapter
# for python plus python_stub; classification exists, adapter claim
# does not.
PYTHON_NO_AUDIT_ADAPTER = "no audit adapter claims python plus python_stub"

# Source versus ecosystem scope: per-language source audit over declared
# source owners stays distinct from ecosystem dx audit plus dx update
# live execution delivered repo-wide.
SOURCE_VS_ECOSYSTEM = "per-language source audit distinct from ecosystem dx audit plus dx update live execution"

# Rejected substitute per the issue alternatives: leaving Python audit
# under the taxonomy stays rejected here as mismatched scope.
REJECTED_TAXONOMY_UNDER = "leaving under taxonomy rejected with mismatched scope"

# Owned gaps stay explicit: future tool selection stays owned under
# issue #613; platform plus consumer plus release evidence stays owned
# gap; no Supported claim.
OWNED_SELECTION_613 = "future tool selection stays owned under issue #613"
OWNED_GAPS_NOTE = "platform plus consumer plus release evidence stays owned gap"
NO_SUPPORTED_CLAIM = "no Supported claim"
BACKENDS_PROVISIONAL = "backends stay provisional"

# Honesty line (never pinned as supported here).
SEED_ONLY = "qualified seed-only under issue #613"

# Live proof shape (no Python audit adapter test exists: the fixture
# triple plus grep contract checks plus bazel build of the fixture plus
# the python hello fixture is the live proof; audit claims nothing).
PYTHON_AUDIT_PROOF = "bazel build //python/tests/fixtures/python_audit:corpus_starlark plus python hello fixture green"
