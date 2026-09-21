"""Python source-audit selection pins.

Contract: `docs/product/support-matrix.md`, `docs/tools/tool-baseline.md#curated-differences`,
`docs/quality/action-model.md#capability-semantics`, `docs/decisions/0010-python-foundation.md`.
Fixture: `python/tests/fixtures/python_audit/` via
`bazel run //tools/ci:python_audit_qualification`.
"""

# Source-audit selection: Ruff S (flake8-bandit) via the pinned Ruff
# standalone artifact; curated audit stays empty with explicit
# disablement for the python family (audit opt-in, no default fetch).
PYTHON_AUDIT_TOOL = "ruff audit over python plus python_stub via S ruleset"
PYTHON_AUDIT_EMPTY = "curated audit stays empty with explicit disablement"
PYTHON_AUDIT_FAMILY_EMPTY = "python family audit stays empty with explicit disablement"
PYTHON_AUDIT_BANDIT_EXCLUDED = "Bandit excluded from v1 by ADR 0019"
PYTHON_AUDIT_SECRETS_SEPARATE = "secrets family rides Gitleaks detect with redact plus SARIF"

# Selected artifact plus ruleset plus acquisition: Ruff 0.16.7
# standalone checksummed artifact, S ruleset via native ruff.toml
# opt-in with pinned upstream defaults otherwise clean, no new
# acquisition beyond the qualified Ruff route.
PYTHON_AUDIT_VERSION = "ruff 0.16.7 standalone artifact"
PYTHON_AUDIT_RULESET = "S flake8-bandit ruleset via native ruff.toml opt-in"
PYTHON_AUDIT_ACQUISITION = "checksummed standalone artifact with no new acquisition"
PYTHON_AUDIT_UPSTREAM_DEFAULTS = "pinned upstream defaults otherwise clean with no hidden preset"

# Audit adapter claim: ruff audit claims python plus python_stub,
# check-only via the hermetic ruff check JSON path shared with lint;
# lint plus format plus typecheck stay owned by their delivered layers.
PYTHON_AUDIT_ADAPTER = "ruff audit claims python plus python_stub check-only via ruff check JSON"
PYTHON_LINT = "python family lint pydoclint plus ruff"
PYTHON_FORMAT = "python family format ruff"
PYTHON_TYPECHECK = "python family typecheck ty"
PYTHON_BASELINE_OPTINS = "flake8 plus pylint stay baseline opt-ins"

# Source versus ecosystem scope: per-language source audit over declared
# source owners stays distinct from ecosystem dx audit plus dx update
# live execution delivered repo-wide.
SOURCE_VS_ECOSYSTEM = "per-language source audit distinct from ecosystem dx audit plus dx update live execution"

# Rejected substitutes: leaving under taxonomy rejected with mismatched
# scope; Bandit re-selection rejected here (stays excluded by ADR 0019,
# reconsideration owned under issue #970, never silent).
REJECTED_TAXONOMY_UNDER = "leaving under taxonomy rejected with mismatched scope"
REJECTED_BANDIT_RESELECT = "Bandit re-selection rejected here with ADR 0019 exclusion standing"

# Platform evidence: Ruff per-host standalone artifacts already
# delivered for the required hosts; audit rides the same bytes.
PLATFORM_RUFF_HOSTS = "ruff per-host artifacts linux_x86_64 plus linux_arm64 plus macos_arm64 plus macos_x86_64 plus windows_x86_64"
PLATFORM_RUFF_ROUTE = "standalone artifact route with per-host digests in quality/artifacts/ruff"

# Consumer evidence: adopt-python proves the consumer path; audit
# opt-in creates no fetch when unselected (lazy, no hidden preset).
CONSUMER_ADOPT_PYTHON = "examples/adopt-python consumer with lazy audit opt-in"

# Release evidence: promotion checklist plus SBOM plus signing linkage;
# no Supported claim until platform plus consumer plus release passes.
RELEASE_CHECKLIST = "promotion-checklist plus SBOM plus signing linkage with no Supported claim"

# Owned gaps stay explicit: selection owned under issue #801 (successor
# to closed #613); taxonomy stays taxonomy-only under closed #512;
# adapter execution plus digests plus platform plus consumer plus release
# promotion stays owned gap under #802 plus #808; no Supported claim.
OWNED_SELECTION_801 = "future tool selection owned under issue #801 with successor to closed #613"
OWNED_TAXONOMY_512 = "issue #512 stays taxonomy-only with no Python audit ownership"
OWNED_GAPS_NOTE = "platform plus consumer plus release evidence stays owned gap"
OWNED_PROMOTION_808 = "Supported promotion stays owned under #808 with per-host #803 through #807"
NO_SUPPORTED_CLAIM = "no Supported claim"
BACKENDS_PROVISIONAL = "backends stay provisional"

# Honesty line (never pinned as supported here).
SEED_ONLY = "qualified seed-only under issue #801"

# Live proof shape: the fixture triple plus audit samples plus grep
# contract checks plus bazel build of the fixture plus the python hello
# fixture is the live proof; audit runs the hermetic ruff check path
# already proven by the runner lint cells plus parser units.
PYTHON_AUDIT_PROOF = "bazel build //python/tests/fixtures/python_audit:corpus_starlark plus python hello fixture green with ruff check path proven"
