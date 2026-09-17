"""Unit tests for the frozen v1 curated defaults (issue #89 item 2).

Pins the `quality/curated_defaults.bzl` manifest shape that the
`//tools/ci:release_policy` shell harness diffs against the default
lifecycle policy: every curated family keeps its exact default tool
set, and the FORMAT_FROZEN formatter set never drifts without a major
release. Additions to curated lint/audit membership arrive here with
compat-qual + notes + prior-set override review, never silently.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":curated_defaults.bzl", "CURATED_DEFAULTS", "FORMAT_FROZEN")

def curated_defaults_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "curated families stay frozen at eight",
                sorted(CURATED_DEFAULTS.keys()),
                [
                    "javascript",
                    "json",
                    "markdown",
                    "python",
                    "rust",
                    "starlark",
                    "toml",
                    "typescript",
                ],
            ),
            expect_equal(
                "python curated defaults stay Ruff + Ty + pydoclint",
                CURATED_DEFAULTS["python"],
                {
                    "audit": [],
                    "format": ["ruff"],
                    "lint": ["pydoclint", "ruff"],
                    "typecheck": ["ty"],
                },
            ),
            expect_equal(
                "typescript curated defaults stay Biome + tsc",
                CURATED_DEFAULTS["typescript"],
                {
                    "audit": [],
                    "format": ["biome"],
                    "lint": ["biome"],
                    "typecheck": ["tsc"],
                },
            ),
            expect_equal(
                "javascript curated defaults stay Biome-only",
                CURATED_DEFAULTS["javascript"],
                {
                    "audit": [],
                    "format": ["biome"],
                    "lint": ["biome"],
                    "typecheck": [],
                },
            ),
            expect_equal(
                "json curated defaults stay Biome lint + Prettier format",
                CURATED_DEFAULTS["json"],
                {
                    "audit": [],
                    "format": ["prettier"],
                    "lint": ["biome"],
                    "typecheck": [],
                },
            ),
            expect_equal(
                "rust curated defaults stay rustfmt + Clippy + rustc",
                CURATED_DEFAULTS["rust"],
                {
                    "audit": [],
                    "format": ["rustfmt"],
                    "lint": ["clippy"],
                    "typecheck": ["rustc"],
                },
            ),
            expect_equal(
                "formatter set stays frozen (major-release gate)",
                FORMAT_FROZEN,
                {
                    "javascript": ["biome"],
                    "json": ["prettier"],
                    "markdown": [],
                    "python": ["ruff"],
                    "rust": ["rustfmt"],
                    "starlark": ["buildifier"],
                    "toml": ["taplo"],
                    "typescript": ["biome"],
                },
            ),
        ],
    )
