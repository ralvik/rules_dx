"""Unit tests for the normalized environment plans (M25 WP2).

Unit checks pin every `env_plan_key_error` branch (empty, separators,
pipe), `env_plan_value_error` (empty, pipe), `env_plan_exec_error`
(empty logical-only, relative suffixes, shard-suffix refusal),
`env_plan_record_error` (bad producer, bad integration, empty entries,
bad entry key/value/exec, within-record duplicates),
`env_plan_integration_error` (admitted Rust versus deferred
Python/Node), `env_plan_conflict_error` (clean merge, silent identical
duplicates, cross-producer collisions, same-producer divergent values,
divergent exec paths), and the deterministic merge/fingerprint
rendering (owner grouping, entry sorting over the full (key, value,
exec) triple, duplicate collapse, exec-bound identity).
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(
    ":plan.bzl",
    "DX_ENV_ADMITTED_INTEGRATIONS",
    "DX_ENV_PLAN_OUTPUT_GROUP",
    "DX_ENV_SHARD_SUFFIX",
    "env_plan_conflict_error",
    "env_plan_entry",
    "env_plan_exec_error",
    "env_plan_fingerprint",
    "env_plan_integration_error",
    "env_plan_key_error",
    "env_plan_merge_records",
    "env_plan_record",
    "env_plan_record_error",
    "env_plan_value_error",
)

def _record_a():
    return env_plan_record(
        "//env:alpha",
        "rust",
        [env_plan_entry("runtime", "stable-x86_64")],
    )

def _record_b():
    return env_plan_record(
        "//env:beta",
        "rust",
        [env_plan_entry("runtime", "stable-aarch64")],
    )

def env_plan_defs_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "env plan freezes the private output group, shard suffix, and first integration",
                [
                    DX_ENV_PLAN_OUTPUT_GROUP,
                    DX_ENV_SHARD_SUFFIX,
                    DX_ENV_ADMITTED_INTEGRATIONS,
                ],
                ["dx_env_plans", ".dxenv.pb", ("rust",)],
            ),
            expect_equal(
                "env_plan_key_error accepts single tokens",
                [env_plan_key_error("runtime"), env_plan_key_error("abi")],
                ["", ""],
            ),
            expect_equal(
                "env_plan_key_error rejects empty keys, separators, and pipes",
                [
                    env_plan_key_error(""),
                    env_plan_key_error("a/b"),
                    env_plan_key_error("a\\b"),
                    env_plan_key_error("a|b"),
                ],
                [
                    "invalid env plan key '': must be a non-empty single token",
                    "invalid env plan key 'a/b': must not contain '/' or '\\'",
                    "invalid env plan key 'a\\b': must not contain '/' or '\\'",
                    "invalid env plan key 'a|b': must not contain '|'",
                ],
            ),
            expect_equal(
                "env_plan_value_error accepts normalized values",
                [env_plan_value_error("stable-x86_64"), env_plan_value_error("1.89.0")],
                ["", ""],
            ),
            expect_equal(
                "env_plan_value_error rejects empty values and pipes",
                [
                    env_plan_value_error(""),
                    env_plan_value_error("a|b"),
                ],
                [
                    "invalid env plan value '': must be a non-empty identity input",
                    "invalid env plan value 'a|b': must not contain '|'",
                ],
            ),
            expect_equal(
                "env_plan_exec_error accepts empty logical-only and relative suffixes",
                [env_plan_exec_error(""), env_plan_exec_error("toolchain/rustc"), env_plan_exec_error("rustc")],
                ["", "", ""],
            ),
            expect_equal(
                "env_plan_exec_error rejects bad suffixes",
                [
                    env_plan_exec_error("/out/rustc"),
                    env_plan_exec_error("tool\\chain"),
                    env_plan_exec_error("src/./rustc"),
                    env_plan_exec_error("a.dxenv.pb"),
                ],
                [
                    "invalid env plan exec path '/out/rustc': must not be absolute",
                    "invalid env plan exec path 'tool\\chain': must not contain '\\'",
                    "invalid env plan exec path 'src/./rustc': must not contain '.' or '..' segments",
                    "invalid env plan exec path 'a.dxenv.pb': must not use the reserved shard suffix '.dxenv.pb'",
                ],
            ),
            expect_equal(
                "env_plan_record_error accepts a well-formed record",
                env_plan_record_error(_record_a()),
                "",
            ),
            expect_equal(
                "env_plan_record_error rejects bad producers",
                [
                    env_plan_record_error(env_plan_record("", "rust", [env_plan_entry("k", "v")])),
                    env_plan_record_error(env_plan_record("env:alpha", "rust", [env_plan_entry("k", "v")])),
                ],
                [
                    "invalid env plan record: producer must be a non-empty label",
                    "invalid env plan record 'env:alpha': producer must be a label in observation rendering",
                ],
            ),
            expect_equal(
                "env_plan_record_error rejects empty integration and empty entries",
                [
                    env_plan_record_error(env_plan_record("//env:a", "", [env_plan_entry("k", "v")])),
                    env_plan_record_error(env_plan_record("//env:a", "rust", [])),
                ],
                [
                    "invalid env plan record '//env:a': integration must be a non-empty language class",
                    "invalid env plan record '//env:a': entries must be non-empty (targets with no contribution carry no record)",
                ],
            ),
            expect_equal(
                "env_plan_record_error rejects bad entry keys",
                env_plan_record_error(env_plan_record("//env:a", "rust", [env_plan_entry("a/b", "v")])),
                "invalid env plan record '//env:a': invalid env plan key 'a/b': must not contain '/' or '\\'",
            ),
            expect_equal(
                "env_plan_record_error rejects bad entry values",
                env_plan_record_error(env_plan_record("//env:a", "rust", [env_plan_entry("k", "")])),
                "invalid env plan record '//env:a': invalid env plan value '': must be a non-empty identity input",
            ),
            expect_equal(
                "env_plan_record_error rejects bad exec paths",
                env_plan_record_error(env_plan_record("//env:a", "rust", [env_plan_entry("k", "v", "x.dxenv.pb")])),
                "invalid env plan record '//env:a': invalid env plan exec path 'x.dxenv.pb': must not use the reserved shard suffix '.dxenv.pb'",
            ),
            expect_equal(
                "env_plan_record_error rejects within-record duplicate keys",
                env_plan_record_error(env_plan_record("//env:a", "rust", [env_plan_entry("k", "v1"), env_plan_entry("k", "v2")])),
                "invalid env plan record '//env:a': duplicate key 'k'",
            ),
            expect_equal(
                "env_plan_conflict_error accepts disjoint records",
                env_plan_conflict_error([_record_a(), env_plan_record("//env:beta", "rust", [env_plan_entry("abi", "gnu")])]),
                "",
            ),
            expect_equal(
                "env_plan_conflict_error merges identical duplicates silently",
                env_plan_conflict_error([_record_a(), _record_a()]),
                "",
            ),
            expect_equal(
                "env_plan_conflict_error fails cross-producer collisions",
                env_plan_conflict_error([_record_a(), _record_b()]),
                "env plan conflict: identity key 'runtime' claimed by //env:alpha, //env:beta",
            ),
            expect_equal(
                "env_plan_conflict_error fails divergent values from one producer",
                env_plan_conflict_error([
                    _record_a(),
                    env_plan_record("//env:alpha", "rust", [env_plan_entry("runtime", "nightly-x86_64")]),
                ]),
                "env plan conflict: identity key 'runtime' claimed by //env:alpha, //env:alpha",
            ),
            expect_equal(
                "env_plan_conflict_error fails divergent exec paths",
                env_plan_conflict_error([
                    _record_a(),
                    env_plan_record("//env:alpha", "rust", [env_plan_entry("runtime", "stable-x86_64", "out/rustc")]),
                ]),
                "env plan conflict: identity key 'runtime' claimed by //env:alpha, //env:alpha",
            ),
            expect_equal(
                "env_plan_merge_records groups by owner and sorts entries",
                [
                    (record.producer, record.integration, [entry.key for entry in record.entries])
                    for record in env_plan_merge_records([
                        env_plan_record("//env:b", "rust", [env_plan_entry("z-key", "1"), env_plan_entry("a-key", "2")]),
                        _record_a(),
                        _record_a(),
                        env_plan_record("//env:a", "python", [env_plan_entry("interpreter", "3.12")]),
                    ])
                ],
                [
                    ("//env:a", "python", ["interpreter"]),
                    ("//env:alpha", "rust", ["runtime"]),
                    ("//env:b", "rust", ["a-key", "z-key"]),
                ],
            ),
            expect_equal(
                "env_plan_fingerprint renders the normalized hash input",
                env_plan_fingerprint([_record_b(), _record_a(), _record_a()]),
                "[{\"entries\":[{\"exec_path\":\"\",\"key\":\"runtime\",\"value\":\"stable-x86_64\"}],\"integration\":\"rust\",\"producer\":\"//env:alpha\"}," +
                "{\"entries\":[{\"exec_path\":\"\",\"key\":\"runtime\",\"value\":\"stable-aarch64\"}],\"integration\":\"rust\",\"producer\":\"//env:beta\"}]",
            ),
            expect_equal(
                "env_plan_fingerprint binds exec paths",
                env_plan_fingerprint([env_plan_record("//env:a", "rust", [env_plan_entry("k", "v", "out/rustc")])]),
                "[{\"entries\":[{\"exec_path\":\"out/rustc\",\"key\":\"k\",\"value\":\"v\"}],\"integration\":\"rust\",\"producer\":\"//env:a\"}]",
            ),
            expect_equal(
                "env_plan_integration_error admits only the Rust first integration",
                [
                    env_plan_integration_error("rust"),
                    env_plan_integration_error("python"),
                    env_plan_integration_error("node"),
                ],
                [
                    "",
                    "unsupported env plan integration 'python': admitted first-release integrations are (\"rust\",)",
                    "unsupported env plan integration 'node': admitted first-release integrations are (\"rust\",)",
                ],
            ),
        ],
    )
