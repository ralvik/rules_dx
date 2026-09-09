"""Unit and load tests for the arithmetic subject (M01 WP1, WP2).

`expect_equal` calls below run while this file loads or while BUILD calls
the exported macros, which is the loading phase. The recorded values prove
the subject code executed there.
"""

load("//tools/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":arithmetic.bzl", "add", "greet", "mul")

# Module top-level binding: evaluated when this file loads.
LOADED_SUM = add(40, 2)

def arithmetic_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal("add(1, 2)", add(1, 2), 3),
            expect_equal("add(-1, 1)", add(-1, 1), 0),
            expect_equal("mul(3, 4)", mul(3, 4), 12),
            expect_equal("mul(0, 9)", mul(0, 9), 0),
            expect_equal("greet(world)", greet("world"), "Hello, world!"),
        ],
    )

def arithmetic_load_tests(name):
    starlark_test(
        name = name,
        mode = "load",
        checks = [
            expect_equal("module top-level binding", LOADED_SUM, 42),
            expect_equal("public entry add is callable", add(0, 0), 0),
            expect_equal("public entry mul is callable", mul(1, 1), 1),
            expect_equal("public entry greet is callable", greet("dx"), "Hello, dx!"),
        ],
    )
