"""Unit and analysis tests for the bootstrap environment registry (M11 WP1).

Unit checks run while this file loads, pinning every `env_name_error`
branch (empty, dot segments, separators, executable suffixes, reserved
stems), `env_tool_error` wrapping (bad primary, bad alias), and
`env_collision_error` (clean composition, same-owner repeats,
primary/alias cross-tool collisions, case folding, multi-claimant
listing). The analysis test pins the transitive composition observation
rendering for the fixture config.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":defs.bzl", "env_collision_error", "env_name_error", "env_tool_error", "env_tool_record")

def env_defs_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "env_name_error accepts a dotted, dashed tool name",
                env_name_error("my-tool.cli"),
                "",
            ),
            expect_equal(
                "env_name_error rejects an empty name",
                env_name_error(""),
                "invalid host name '': must be a non-empty single path component",
            ),
            expect_equal(
                "env_name_error rejects dot segments",
                [env_name_error("."), env_name_error("..")],
                [
                    "invalid host name '.': must not be '.' or '..'",
                    "invalid host name '..': must not be '.' or '..'",
                ],
            ),
            expect_equal(
                "env_name_error rejects path separators",
                [env_name_error("a/b"), env_name_error("a\\b")],
                [
                    "invalid host name 'a/b': must not contain '/' or '\\'",
                    "invalid host name 'a\\b': must not contain '/' or '\\'",
                ],
            ),
            expect_equal(
                "env_name_error rejects executable suffixes case-insensitively",
                [env_name_error("tool.exe"), env_name_error("tool.BAT")],
                [
                    "invalid host name 'tool.exe': must not carry an executable suffix (found '.exe')",
                    "invalid host name 'tool.BAT': must not carry an executable suffix (found '.bat')",
                ],
            ),
            expect_equal(
                "env_name_error rejects Windows reserved stems with extensions",
                [env_name_error("CON"), env_name_error("com1"), env_name_error("nul.txt")],
                [
                    "invalid host name 'CON': stem 'con' is reserved on Windows",
                    "invalid host name 'com1': stem 'com1' is reserved on Windows",
                    "invalid host name 'nul.txt': stem 'nul' is reserved on Windows",
                ],
            ),
            expect_equal(
                "env_tool_error accepts a record with aliases",
                env_tool_error("alpha", ["a"]),
                "",
            ),
            expect_equal(
                "env_tool_error rejects a bad primary name",
                env_tool_error("", []),
                "invalid bin_name: invalid host name '': must be a non-empty single path component",
            ),
            expect_equal(
                "env_tool_error rejects a bad alias",
                env_tool_error("alpha", ["good", "bad/x"]),
                "invalid alias 'bad/x': invalid host name 'bad/x': must not contain '/' or '\\'",
            ),
            expect_equal(
                "env_collision_error accepts disjoint records",
                env_collision_error([
                    env_tool_record("//env:tool_alpha", "alpha", ["a"]),
                    env_tool_record("//env:tool_beta", "beta", []),
                ]),
                "",
            ),
            expect_equal(
                "env_collision_error ignores same-owner repeats",
                env_collision_error([
                    env_tool_record("//env:tool_alpha", "alpha", ["alpha", "a"]),
                ]),
                "",
            ),
            expect_equal(
                "env_collision_error reports a primary/alias cross-tool collision",
                env_collision_error([
                    env_tool_record("//env:tool_alpha", "alpha", ["a"]),
                    env_tool_record("//env:tool_beta", "beta", ["a"]),
                ]),
                "host-name collision: host name 'a' claimed by //env:tool_alpha, //env:tool_beta",
            ),
            expect_equal(
                "env_collision_error folds case across owners",
                env_collision_error([
                    env_tool_record("//env:tool_alpha", "Alpha", []),
                    env_tool_record("//env:tool_beta", "ALPHA", []),
                ]),
                "host-name collision: host name 'alpha' claimed by //env:tool_alpha, //env:tool_beta",
            ),
            expect_equal(
                "env_collision_error lists every collision sorted",
                env_collision_error([
                    env_tool_record("//env:tool_alpha", "shared", ["z-shared"]),
                    env_tool_record("//env:tool_beta", "beta", ["Shared"]),
                    env_tool_record("//env:tool_gamma", "gamma", ["Z-Shared"]),
                ]),
                "host-name collision: host name 'shared' claimed by //env:tool_alpha, //env:tool_beta; " +
                "host name 'z-shared' claimed by //env:tool_alpha, //env:tool_gamma",
            ),
        ],
    )

EXPECTED_ENV_OBSERVATIONS = """subject //env:config_under_test
field count=2
field names=a,alpha,beta"""

def env_config_analysis_tests(name):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = [":config_under_test"],
        expected_observations = EXPECTED_ENV_OBSERVATIONS,
    )
