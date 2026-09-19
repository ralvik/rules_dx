"""Unit tests for the single-sourced versioned registry (issue #321).

Validates the aggregated registry queries without pinning exact contents:
versions stay v1, schemas validate, and every query derives from the
single-sourced maps, so adding a language/tool edits registry data plus
compat, never a parallel allowlist.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":registry.bzl", "REGISTRY_SCHEMA_VERSION", "is_curated_family", "is_registry_class", "is_registry_tool", "registry_classes", "registry_curated_families", "registry_deferred_classes", "registry_families", "registry_schema_error", "registry_tools")

def registry_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "registry schema version stays v1",
                REGISTRY_SCHEMA_VERSION,
                1,
            ),
            expect_equal(
                "aggregated registry schemas validate",
                registry_schema_error(),
                "",
            ),
            expect_equal(
                "registry queries stay non-empty",
                [
                    len(registry_classes()) > 0,
                    len(registry_families()) > 0,
                    len(registry_tools()) > 0,
                    len(registry_curated_families()) > 0,
                    len(registry_deferred_classes()) > 0,
                ],
                [True, True, True, True, True],
            ),
            expect_equal(
                "registry queries admit the frozen core",
                [
                    is_registry_class("rust"),
                    is_registry_class("python"),
                    is_registry_tool("ruff"),
                    is_registry_tool("biome"),
                    is_curated_family("python"),
                    is_curated_family("rust"),
                ],
                [True, True, True, True, True, True],
            ),
            expect_equal(
                "registry queries reject unknown IDs",
                [
                    is_registry_class("not_a_class"),
                    is_registry_tool("not_a_tool"),
                    is_curated_family("not_a_family"),
                ],
                [False, False, False],
            ),
        ],
    )
