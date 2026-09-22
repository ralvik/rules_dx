"""Unit tests for typed native-config validation (WP2).

Contract: `docs/quality/native-configuration.md`.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(
    ":native_config.bzl",
    "collect_native_configs",
    "native_config_error",
    "native_config_extension",
)

_HINTS = [
    struct(tool_id = "buildifier"),
    struct(tool_id = "taplo"),
]

def native_config_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "native_config_extension pins the buildifier/biome JSON transport",
                [
                    native_config_extension("buildifier"),
                    native_config_extension("biome"),
                ],
                [".json", ".json"],
            ),
            expect_equal(
                "native_config_extension pins TOML for taplo/rustfmt/ruff",
                [
                    native_config_extension("taplo"),
                    native_config_extension("rustfmt"),
                    native_config_extension("ruff"),
                ],
                [".toml", ".toml", ".toml"],
            ),
            expect_equal(
                "native_config_extension pins the vale INI transport",
                native_config_extension("vale"),
                ".ini",
            ),
            expect_equal(
                "native_config_extension pins the eslint JS flat-config transport",
                native_config_extension("eslint"),
                ".js",
            ),
            expect_equal(
                "native_config_extension pins Scala/.NET transports",
                [
                    native_config_extension("scalafmt"),
                    native_config_extension("scalafix"),
                    native_config_extension("csharpier"),
                    native_config_extension("fsharplint"),
                ],
                [".conf", ".conf", ".yaml", ".json"],
            ),
            expect_equal(
                "native_config_extension pins native cohort transports",
                [
                    native_config_extension("clang_format"),
                    native_config_extension("clang_tidy"),
                    native_config_extension("cppcheck"),
                    native_config_extension("staticcheck"),
                ],
                [".clang-format", ".clang-tidy", ".txt", ".conf"],
            ),
            expect_equal(
                "native_config_extension pins Structured transports",
                [
                    native_config_extension("buf"),
                    native_config_extension("qmlformat"),
                    native_config_extension("qmllint"),
                ],
                [".yaml", ".ini", ".ini"],
            ),
            expect_equal(
                "native_config_extension pins file-family transports (See: docs/quality/tool-integrations.md)",
                [
                    native_config_extension("stylelint"),
                    native_config_extension("djlint"),
                    native_config_extension("yamllint"),
                ],
                [".json", ".toml", ".yaml"],
            ),
            expect_equal(
                "native_config_error rejects biome.jsonc",
                native_config_error(
                    "biome",
                    "cfg/biome.jsonc",
                    True,
                    [],
                ),
                "native_config (biome): src must end in '.json', got cfg/biome.jsonc",
            ),
            expect_equal(
                "native_config_error accepts a checked-in config",
                native_config_error(
                    "buildifier",
                    ".buildifier.json",
                    True,
                    [],
                ),
                "",
            ),
            expect_equal(
                "native_config_error accepts checked-in data",
                native_config_error(
                    "vale",
                    ".vale.ini",
                    True,
                    [struct(is_source = True, path = "styles/Test/Cotton.yml")],
                ),
                "",
            ),
            expect_equal(
                "native_config_error rejects an unknown tool",
                native_config_error("prettier", "x.json", True, []),
                "native_config: unknown tool 'prettier': want one of " +
                "biome, buf, buildifier, checkstyle, clang_format, clang_tidy, cppcheck, csharpier, djlint, eslint, fsharplint, qmlformat, qmllint, ruff, rustfmt, scalafix, scalafmt, staticcheck, stylelint, taplo, vale, yamllint",
            ),
            expect_equal(
                "native_config_error requires a config",
                native_config_error("taplo", "", True, []),
                "native_config (taplo): src is required",
            ),
            expect_equal(
                "native_config_error rejects a generated config",
                native_config_error("taplo", "gen/taplo.toml", False, []),
                "native_config (taplo): src must be a checked-in source " +
                "file, got generated gen/taplo.toml",
            ),
            expect_equal(
                "native_config_error rejects a mis-suffixed config",
                native_config_error("vale", ".vale.json", True, []),
                "native_config (vale): src must end in '.ini', got .vale.json",
            ),
            expect_equal(
                "native_config_error rejects a generated data member",
                native_config_error(
                    "vale",
                    ".vale.ini",
                    True,
                    [struct(is_source = False, path = "gen/Cotton.yml")],
                ),
                "native_config (vale): data must be checked-in source " +
                "files, got generated gen/Cotton.yml",
            ),
            expect_equal(
                "collect_native_configs resolves hints in stage-tool order",
                collect_native_configs(
                    _HINTS,
                    ["taplo", "buildifier"],
                    "//q:t",
                ),
                {"buildifier": _HINTS[0], "taplo": _HINTS[1]},
            ),
            expect_equal(
                "collect_native_configs omits stage tools without a hint",
                collect_native_configs(_HINTS, ["vale", "taplo"], "//q:t"),
                {"taplo": _HINTS[1]},
            ),
            expect_equal(
                "collect_native_configs ignores hints outside the stages",
                collect_native_configs(_HINTS, ["taplo"], "//q:t"),
                {"taplo": _HINTS[1]},
            ),
            expect_equal(
                "collect_native_configs is empty without hints",
                collect_native_configs([], ["taplo"], "//q:t"),
                {},
            ),
        ],
    )
