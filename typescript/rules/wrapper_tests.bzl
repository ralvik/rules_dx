"""Wrapper-contract tests for the TypeScript wrappers (#87 item 2).
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":defs.bzl", "typescript_srcs_rejection")

def typescript_wrapper_contract_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "declaration file is rejected",
                typescript_srcs_rejection(["types.d.ts"]) != None,
                True,
            ),
            expect_equal(
                "declaration rejection names the files",
                typescript_srcs_rejection(["types.d.ts"]),
                "typescript_project takes real sources only; declaration " +
                "files are inert and must not be listed in srcs " +
                "(rejected per docs/testing/generation.md): types.d.ts",
            ),
            expect_equal(
                "every declaration suffix is rejected",
                typescript_srcs_rejection(["a.d.ts", "b.d.mts", "c.d.cts"]) != None,
                True,
            ),
            expect_equal(
                "rejected files render sorted",
                typescript_srcs_rejection(["b.d.mts", "a.d.ts"]),
                "typescript_project takes real sources only; declaration " +
                "files are inert and must not be listed in srcs " +
                "(rejected per docs/testing/generation.md): a.d.ts, b.d.mts",
            ),
            expect_equal(
                "mixed srcs reject only the declarations",
                typescript_srcs_rejection(["main.ts", "types.d.ts"]),
                "typescript_project takes real sources only; declaration " +
                "files are inert and must not be listed in srcs " +
                "(rejected per docs/testing/generation.md): types.d.ts",
            ),
            expect_equal(
                "real sources are clean",
                typescript_srcs_rejection(["main.ts", "view.tsx", "lib.mts", "old.cts"]),
                None,
            ),
            expect_equal(
                "similar non-declaration names are clean",
                typescript_srcs_rejection(["d.ts", "ad.ts", "types.d.ts.bak"]),
                None,
            ),
            expect_equal(
                "empty srcs are clean",
                typescript_srcs_rejection([]),
                None,
            ),
        ],
    )
