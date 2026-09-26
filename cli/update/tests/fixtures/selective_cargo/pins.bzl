"""Selective Cargo `dx update` per-crate pins."""

SELECTIVE_CARGO = "wont-fix"

SELECTIVE_CARGO_FULL = "CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello"

SELECTIVE_CARGO_SELECTOR = "cargo:anyhow"
SELECTIVE_CARGO_SELECTOR_LABEL = "cargo:<crate>"

SELECTIVE_CARGO_HINT = "use `dx update cargo` for the set"

BUMP_FOLLOWUP_CARGO = "dx update cargo"

REJECTED_SILENT_FULL_SUBSTITUTION = "silent full-update substitution rejected"
REJECTED_PRIVATE_CARGO_UPDATE_P = "private cargo update -p rejected"

NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #633"
