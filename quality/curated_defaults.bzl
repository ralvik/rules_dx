"""Frozen v1 curated quality defaults (issue #89 item 2).

Versioned manifest of the curated default lint/audit membership and
default formatter set per policy family, matching the curated baseline
in `docs/tools/tool-baseline.md#curated-differences`. Omitted workspace
selections use these defaults; supported alternatives (flake8, pylint,
ESLint, Prettier-as-alternative) remain explicit opt-ins and never run
without selection.

Release policy (`docs/tools/tool-baseline.md#default-lifecycle-direction`,
`docs/quality/quality-testing.md:390-398`):
- Removing a default, changing the default formatter set, or otherwise
  breaking consumer workflows requires a major release.
- Only additions to curated lint/audit membership are eligible for a
  minor release, and only after compatibility qualification, explicit
  release notes, and a tested workspace override reproducing the prior
  lint/audit set.
- Normal tool-version updates within the pinned set follow the exact-pin
  currency policy and never authorize membership changes.

The `//tools/ci:release_policy` harness diffs this manifest against the
frozen baseline below and fails closed on removals, formatter-set
changes, or additions missing compat evidence. Never shrink a passing
scope without a recorded major-release decision.
"""

# Family -> capability -> curated default tool IDs, in stable pipeline
# order. Families absent from this map have no curated defaults yet
# (their classes are PARITY_DEFERRED in quality/parity_tests.bzl).
CURATED_DEFAULTS = {
    "javascript": {
        "audit": [],
        "format": ["biome"],
        "lint": ["biome"],
        "typecheck": [],
    },
    "json": {
        "audit": [],
        "format": ["prettier"],
        "lint": ["biome"],
        "typecheck": [],
    },
    "markdown": {
        "audit": [],
        "format": [],
        "lint": ["markdown_check", "vale"],
        "typecheck": [],
    },
    "python": {
        "audit": [],
        "format": ["ruff"],
        "lint": ["pydoclint", "ruff"],
        "typecheck": ["ty"],
    },
    "rust": {
        "audit": [],
        "format": ["rustfmt"],
        "lint": ["clippy"],
        "typecheck": ["rustc"],
    },
    "starlark": {
        "audit": [],
        "format": ["buildifier"],
        "lint": ["buildifier"],
        "typecheck": [],
    },
    "toml": {
        "audit": [],
        "format": ["taplo"],
        "lint": ["taplo"],
        "typecheck": [],
    },
    "typescript": {
        "audit": [],
        "format": ["biome"],
        "lint": ["biome"],
        "typecheck": ["tsc"],
    },
}

# Frozen default formatter set per family: changing any entry requires a
# major release (never a minor). The harness greps this file for the
# exact lines below, so keep the `FORMAT_FROZEN[<family>] = [...]`
# shape stable.
FORMAT_FROZEN = {
    "javascript": ["biome"],
    "json": ["prettier"],
    "markdown": [],
    "python": ["ruff"],
    "rust": ["rustfmt"],
    "starlark": ["buildifier"],
    "toml": ["taplo"],
    "typescript": ["biome"],
}
