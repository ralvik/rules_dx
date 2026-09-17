"""Unit and analysis tests for the GitHub Release publisher (issue #182).

Unit checks run while this file loads, pinning the tag and draft-gate
helpers. Analysis tests pin the frozen observation rendering for the
fixture draft releases: the default release profile and an explicit
debug profile, both with no app (a release publishes a set of assets,
not one app). The would-run command and draft-only flags are proven by
the `github_verify` sh_tests in BUILD (via `GH_RELEASE_DRY_RUN=1`,
without network access), not here.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":github.bzl", "github_draft_error", "github_tag_error")

def github_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "github_tag_error accepts the dry-run placeholder tag",
                github_tag_error("v0.0.0-dryrun"),
                "",
            ),
            expect_equal(
                "github_tag_error accepts dotted release tags",
                github_tag_error("v1.2.3"),
                "",
            ),
            expect_equal(
                "github_tag_error rejects an empty tag",
                github_tag_error(""),
                "github_release: invalid tag '': want a non-empty tag " +
                "(for example 'v0.0.0-dryrun')",
            ),
            expect_equal(
                "github_tag_error rejects shell-unsafe tag characters",
                github_tag_error("v1.0;curl evil"),
                "github_release: invalid tag 'v1.0;curl evil': want only " +
                "[A-Za-z0-9._-] so the tag embeds safely in the deploy launcher",
            ),
            expect_equal(
                "github_draft_error accepts the draft gate",
                github_draft_error(True),
                "",
            ),
            expect_equal(
                "github_draft_error rejects publishing without approval",
                github_draft_error(False),
                "github_release: draft=False requires explicit owner " +
                "approval per issue #5; keep the draft gate and publish " +
                "the release on GitHub after approval",
            ),
        ],
    )

EXPECTED_GITHUB_DEFAULT_OBSERVATIONS = """subject //deploy/rules:github_demo
file github_demo
field app=
field profile=release"""

EXPECTED_GITHUB_DEBUG_OBSERVATIONS = """subject //deploy/rules:github_demo_debug
file github_demo_debug
field app=
field profile=debug"""

def github_analysis_tests(name):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = [":github_demo"],
        expected_observations = EXPECTED_GITHUB_DEFAULT_OBSERVATIONS,
    )

def github_debug_analysis_tests(name):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = [":github_demo_debug"],
        expected_observations = EXPECTED_GITHUB_DEBUG_OBSERVATIONS,
    )
