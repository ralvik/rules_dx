"""Single source of truth for the repository Rust edition (issue #82).

Split from `defs.bzl` (issue #55): `quality/real_aspects.bzl` needs
`RUST_EDITION` for the provider-less fixture fallback, but loading
`defs.bzl` pulls `@crates//:crates.bzl` (via `dx_rust_crate`), which
fails in consumer workspaces where `rules_dx` is a non-root module
without a crate_universe lockfile ("repinning is not supported across
module boundaries"). This leaf module has no loads, so consumer
aspects stay crate-free. `defs.bzl` re-exports the constant, so
existing callers are unaffected.
"""

RUST_EDITION = "2021"
