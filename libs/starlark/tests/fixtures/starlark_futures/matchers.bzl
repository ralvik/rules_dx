"""Concrete richer-matchers use case (issue #790).

Contract: `docs/testing/starlark.md#authoring`, `docs/decisions/0009-starlark-testing.md`.
Fixture: `libs/starlark/tests/fixtures/starlark_futures/` via
`bazel run //tools/ci:starlark_futures_qualification`.
"""

def greet_report(name):
    return "Hello, " + name + "!"

def pair_error(pair, language):
    if pair == "protobuf" and language == "rust":
        return ""
    return "unsupported codegen pair '" + pair + "/" + language + "': want one of protobuf/rust"

def admitted_pairs():
    return ["protobuf/rust"]

def subject_fields():
    return {"left": "40", "right": "2", "sum": "42"}

def fingerprint_like(producer):
    return json.encode({
        "entries": [{"logical_path": "src/alpha.rs"}],
        "language": "rust",
        "producer": producer,
    })

def is_even(n):
    return n % 2 == 0
