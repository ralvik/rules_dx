"""Single-graph CXX bridge identity (issue #474).

Contract: `docs/native-toolchains.md#rust-and-native-integration`.
"""

# Decided under issue #474 (ad-hoc identity rejected): `cxx` plus
# `cxxbridge-cmd` resolve from the single crate_universe `crates` graph at
# identical versions. Upstream `rust_cxx_bridge.bzl` at CXX 1.0.200 uses
# `tool = "@cxx.rs//:codegen"` with its own `rules_rust` plus `crates.io`
# repos; that second graph is rejected. Decided tool: `@crates//:cxxbridge-cmd`.
CXX_VERSION = "1.0.200"

CXXBRIDGE_CMD_VERSION = "1.0.200"

CXX_CRATE_LABEL = "@crates//:cxx"

CXXBRIDGE_CMD_LABEL = "@crates//:cxxbridge-cmd"
