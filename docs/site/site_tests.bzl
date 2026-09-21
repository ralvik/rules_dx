"""Unit plus execution tests for docs site execution."""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":site.bzl", "MDBOOK_VERSION", "site_api_path", "site_html_name", "site_index_name", "site_prose_error", "site_records_name", "site_search_record", "site_shard_name", "site_summary_name", "site_api_name", "site_symbol_id", "site_symbol_id_error", "site_url_for_symbol")

def site_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "pinned mdBook version stays explicit",
                MDBOOK_VERSION,
                "0.4.43",
            ),
            expect_equal(
                "symbol IDs join language package and name",
                site_symbol_id("python", "demo", "AccountService.create"),
                "python:demo:AccountService.create",
            ),
            expect_equal(
                "symbol identity requires every segment",
                [
                    site_symbol_id_error("", "demo", "Name"),
                    site_symbol_id_error("python", "", "Name"),
                    site_symbol_id_error("python", "demo", ""),
                    site_symbol_id_error("python", "demo", "Name"),
                ],
                [
                    "docs_site: language is required",
                    "docs_site: package is required",
                    "docs_site: qualified name is required",
                    "",
                ],
            ),
            expect_equal(
                "API paths mirror symbol IDs workspace-relatively",
                site_api_path("python:demo:AccountService.create"),
                "api/python/demo/AccountService.create.md",
            ),
            expect_equal(
                "rendered URLs mirror symbol IDs",
                site_url_for_symbol("python:demo:AccountService.create"),
                "api/python/demo/AccountService.create.html",
            ),
            expect_equal(
                "shard outputs stay Bazel-owned textproto",
                site_shard_name("demo"),
                "demo.ir.textproto",
            ),
            expect_equal(
                "aggregate outputs name SUMMARY plus API plus records",
                [site_summary_name("demo"), site_api_name("demo"), site_records_name("demo")],
                ["demo_SUMMARY.md", "demo_api.md", "demo_search_records.json"],
            ),
            expect_equal(
                "render outputs name entry plus single search index",
                [site_html_name("demo"), site_index_name("demo")],
                ["demo_index.html", "demo_searchindex.json"],
            ),
            expect_equal(
                "prose inputs must be Markdown",
                [site_prose_error("guide.md"), site_prose_error("guide.txt")],
                ["", "docs_site: prose inputs must be Markdown, got 'guide.txt'"],
            ),
            expect_equal(
                "search records keep sorted keys",
                site_search_record("api/python/demo/AccountService.create.html", "AccountService.create", "Creates a new account."),
                "{\"body\": \"Creates a new account.\", \"title\": \"AccountService.create\", \"url\": \"api/python/demo/AccountService.create.html\"}",
            ),
        ],
    )

def site_file_tests(name, shard, summary, api, records, html, index):
    starlark_test(
        name = name,
        mode = "execution",
        file_checks = {
            shard: "language: \"python\"\npackage: \"demo\"",
            summary: "# Summary",
            api: "# API Reference",
            records: "\"title\"",
            html: "<!-- rendered by mdBook 0.4.43 fixture -->",
            index: "\"docs\"",
        },
    )
