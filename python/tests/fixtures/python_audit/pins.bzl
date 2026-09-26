PYTHON_AUDIT_TOOL = "ruff audit over python plus python_stub via S ruleset"
PYTHON_AUDIT_EMPTY = "curated audit stays empty with explicit disablement"
PYTHON_AUDIT_FAMILY_EMPTY = "python family audit stays empty with explicit disablement"
PYTHON_AUDIT_BANDIT_EXCLUDED = "Bandit excluded from v1"
PYTHON_AUDIT_SECRETS_SEPARATE = "secrets family rides Gitleaks detect with redact plus SARIF"

PYTHON_AUDIT_VERSION = "ruff 0.16.7 standalone artifact"
PYTHON_AUDIT_RULESET = "S flake8-bandit ruleset via native ruff.toml opt-in"
PYTHON_AUDIT_ACQUISITION = "checksummed standalone artifact with no new acquisition"
PYTHON_AUDIT_UPSTREAM_DEFAULTS = "pinned upstream defaults otherwise clean with no hidden preset"

PYTHON_AUDIT_ADAPTER = "ruff audit claims python plus python_stub check-only via ruff check JSON"
PYTHON_LINT = "python family lint pydoclint plus ruff"
PYTHON_FORMAT = "python family format ruff"
PYTHON_TYPECHECK = "python family typecheck ty"
PYTHON_BASELINE_OPTINS = "flake8 plus pylint stay baseline opt-ins"

SOURCE_VS_ECOSYSTEM = "per-language source audit distinct from ecosystem dx audit plus dx update live execution"

REJECTED_TAXONOMY_UNDER = "leaving under taxonomy rejected with mismatched scope"
REJECTED_BANDIT_RESELECT = "Bandit re-selection rejected here with exclusion standing"

PLATFORM_RUFF_HOSTS = "ruff per-host artifacts linux_x86_64 plus linux_arm64 plus macos_arm64 plus windows_x86_64"
PLATFORM_RUFF_ROUTE = "standalone artifact route with per-host digests in quality/artifacts/ruff"

CONSUMER_ADOPT_PYTHON = "examples/adopt-python consumer with lazy audit opt-in"

RELEASE_CHECKLIST = "promotion-checklist plus SBOM plus signing linkage with no Supported claim"

OWNED_SELECTION_801 = "future tool selection owned with successor to closed #613"
OWNED_TAXONOMY_512 = "stays taxonomy-only with no Python audit ownership"
OWNED_GAPS_NOTE = "platform plus consumer plus release evidence stays owned gap"
OWNED_PROMOTION_808 = "Supported promotion stays owned under pinned with per-host #803 through #807"
NO_SUPPORTED_CLAIM = "no Supported claim"
BACKENDS_PROVISIONAL = "backends stay provisional"

SEED_ONLY = "qualified seed-only"

PYTHON_AUDIT_PROOF = "bazel build //python/tests/fixtures/python_audit:corpus_starlark plus python hello fixture green with ruff check path proven"
