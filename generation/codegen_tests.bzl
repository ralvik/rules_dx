"""Unit tests for the normalized codegen plan records (M25 WP1).

Pins every `codegen_path_error` branch (empty, absolute, backslash, dot
segments), `codegen_record_error` (bad producer, bad language, empty
entries, bad entry paths, within-record duplicates), `codegen_conflict_error`
(clean merge, silent identical duplicates, cross-producer collisions,
same-producer divergent roots), and the deterministic merge/fingerprint
rendering (owner grouping, entry sorting, duplicate collapse).
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(
    ":codegen.bzl",
    "DX_CODEGEN_ADMITTED_PAIRS",
    "DX_CODEGEN_PLAN_OUTPUT_GROUP",
    "DX_CODEGEN_SHARD_SUFFIX",
    "codegen_conflict_error",
    "codegen_entry",
    "codegen_merge_records",
    "codegen_path_error",
    "codegen_plan_fingerprint",
    "codegen_record",
    "codegen_record_error",
)

def _record_a():
    return codegen_record(
        "//gen:alpha",
        "rust",
        [codegen_entry("src/alpha.rs", "src", "alpha")],
    )

def _record_b():
    return codegen_record(
        "//gen:beta",
        "rust",
        [codegen_entry("src/beta.rs", "src", "beta")],
    )

def codegen_defs_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "codegen freezes the private output group, shard suffix, and first pair",
                [
                    DX_CODEGEN_PLAN_OUTPUT_GROUP,
                    DX_CODEGEN_SHARD_SUFFIX,
                    DX_CODEGEN_ADMITTED_PAIRS,
                ],
                ["dx_codegen_plans", ".dxcodegen.pb", (("protobuf", "rust"),)],
            ),
            expect_equal(
                "codegen_path_error accepts relative paths",
                [codegen_path_error("src/a.rs"), codegen_path_error("a")],
                ["", ""],
            ),
            expect_equal(
                "codegen_path_error rejects empty and absolute paths",
                [codegen_path_error(""), codegen_path_error("/src/a.rs")],
                [
                    "invalid codegen path '': must be a non-empty workspace-relative path",
                    "invalid codegen path '/src/a.rs': must not be absolute",
                ],
            ),
            expect_equal(
                "codegen_path_error rejects backslashes and dot segments",
                [
                    codegen_path_error("src\\a.rs"),
                    codegen_path_error("src/./a.rs"),
                    codegen_path_error("src/../a.rs"),
                ],
                [
                    "invalid codegen path 'src\\a.rs': must not contain '\\'",
                    "invalid codegen path 'src/./a.rs': must not contain '.' or '..' segments",
                    "invalid codegen path 'src/../a.rs': must not contain '.' or '..' segments",
                ],
            ),
            expect_equal(
                "codegen_entry marks projections read-only",
                codegen_entry("src/a.rs", "src", "a").read_only,
                True,
            ),
            expect_equal(
                "codegen_record_error accepts a well-formed record",
                codegen_record_error(_record_a()),
                "",
            ),
            expect_equal(
                "codegen_record_error rejects bad producers",
                [
                    codegen_record_error(codegen_record("", "rust", [codegen_entry("a", "b")])),
                    codegen_record_error(codegen_record("gen:alpha", "rust", [codegen_entry("a", "b")])),
                ],
                [
                    "invalid codegen record: producer must be a non-empty label",
                    "invalid codegen record 'gen:alpha': producer must be a label in observation rendering",
                ],
            ),
            expect_equal(
                "codegen_record_error rejects empty language and empty entries",
                [
                    codegen_record_error(codegen_record("//gen:a", "", [codegen_entry("a", "b")])),
                    codegen_record_error(codegen_record("//gen:a", "rust", [])),
                ],
                [
                    "invalid codegen record '//gen:a': language must be a non-empty file class",
                    "invalid codegen record '//gen:a': entries must be non-empty (targets with no contribution carry no record)",
                ],
            ),
            expect_equal(
                "codegen_record_error rejects bad entry paths",
                codegen_record_error(codegen_record("//gen:a", "rust", [codegen_entry("/a", "b")])),
                "invalid codegen record '//gen:a': invalid codegen path '/a': must not be absolute",
            ),
            expect_equal(
                "codegen_record_error rejects within-record duplicate paths",
                codegen_record_error(codegen_record("//gen:a", "rust", [codegen_entry("a", "b"), codegen_entry("a", "c")])),
                "invalid codegen record '//gen:a': duplicate logical path 'a'",
            ),
            expect_equal(
                "codegen_conflict_error accepts disjoint records",
                codegen_conflict_error([_record_a(), _record_b()]),
                "",
            ),
            expect_equal(
                "codegen_conflict_error merges identical duplicates silently",
                codegen_conflict_error([_record_a(), _record_a()]),
                "",
            ),
            expect_equal(
                "codegen_conflict_error fails cross-producer collisions",
                codegen_conflict_error([
                    _record_a(),
                    codegen_record("//gen:evil", "rust", [codegen_entry("src/alpha.rs", "src", "alpha")]),
                ]),
                "codegen path conflict: logical path 'src/alpha.rs' claimed by //gen:alpha, //gen:evil",
            ),
            expect_equal(
                "codegen_conflict_error fails divergent roots from one producer",
                codegen_conflict_error([
                    _record_a(),
                    codegen_record("//gen:alpha", "rust", [codegen_entry("src/alpha.rs", "other", "alpha")]),
                ]),
                "codegen path conflict: logical path 'src/alpha.rs' claimed by //gen:alpha, //gen:alpha",
            ),
            expect_equal(
                "codegen_merge_records groups by owner and sorts entries",
                [
                    (record.producer, record.language, [entry.logical_path for entry in record.entries])
                    for record in codegen_merge_records([
                        codegen_record("//gen:b", "rust", [codegen_entry("z.rs", "s"), codegen_entry("a.rs", "s")]),
                        _record_a(),
                        _record_a(),
                        codegen_record("//gen:a", "python", [codegen_entry("m.py", "s")]),
                    ])
                ],
                [
                    ("//gen:a", "python", ["m.py"]),
                    ("//gen:alpha", "rust", ["src/alpha.rs"]),
                    ("//gen:b", "rust", ["a.rs", "z.rs"]),
                ],
            ),
            expect_equal(
                "codegen_plan_fingerprint renders the normalized hash input",
                codegen_plan_fingerprint([_record_b(), _record_a(), _record_a()]),
                "[{\"entries\":[{\"import_root\":\"src\",\"logical_path\":\"src/alpha.rs\",\"namespace\":\"alpha\",\"read_only\":true}],\"language\":\"rust\",\"producer\":\"//gen:alpha\"}," +
                "{\"entries\":[{\"import_root\":\"src\",\"logical_path\":\"src/beta.rs\",\"namespace\":\"beta\",\"read_only\":true}],\"language\":\"rust\",\"producer\":\"//gen:beta\"}]",
            ),
        ],
    )
