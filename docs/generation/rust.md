# Rust Generation Contract

Rust follows the [common contract](common.md). See
[ADR 0013](../decisions/0013-rust-javascript-typescript-foundations.md) and
[ADR 0015](../decisions/0015-first-party-gazelle-extensions.md) for rationale and the
[generation test matrix](../testing/generation.md#generate-command-and-gazelle-extensions) for
required evidence.

## Crates And Cargo

Rust ownership is per crate, not per file. An authoritative crate root owns its root and every
recursively loaded module, including explicit `#[path]` modules. A module is never rewritten as an
independent crate. Integration-test roots separately own their loaded module trees; ownership that
cannot preserve Rust module semantics fails generation.

A source-only crate may generate without Cargo metadata when it uses recognized roots, local modules,
standard-library references, and tested ruleset defaults. The complete source-only layout is:

- `<crate-directory>/src/lib.rs`.
- `<crate-directory>/src/main.rs`.
- Direct `<crate-directory>/tests/*.rs` integration-test roots.

The fallback name for either conventional source root is the normalized crate-directory basename.
When both roots exist, the library owns that name and module tree and the thin binary is
`<normalized-crate-name>_bin`. No other standalone root, custom path, `src/bin` tree, example,
benchmark, or build script is inferred without Cargo metadata.

When `Cargo.toml` exists, its package and target names, paths, editions, features, crate kinds,
build-script policy, and dependency scopes are authoritative. Generation does not invent or
contradict Cargo policy and does not read private upstream graph serialization. Missing, duplicate,
ambiguous, or conflicting declarations fail rather than falling back to source inference.

The initial ordinary-target mapping parses checked-in Cargo declarations in the first-party
extension, resolves path crates through Gazelle's local rule index, and emits exact imported external
names through the public `@crates//:crates.bzl` `crate_deps` and `aliases` macros. The macro
`package_name` is the crate_universe map key for the manifest: the parent Bazel directory joined
with the Cargo package name (for example `dx/dx_env` for the `dx_env` package in `//cli/env`), not
the Bazel package path alone. The
extension never reads `Cargo.Bazel.lock`, `cargo-bazel.json`, or crate_universe's private dependency
maps. Production imports must be declared in `[dependencies]`; test imports may additionally use
`[dev-dependencies]`.

The `crate_deps` call and `aliases` heads are fully managed and rewritten on every generate.
Hand-maintained labels in a plain-list `+` tail (for example toolchain labels no import resolves)
are preserved across regenerations and never removed by generate; delete them manually when stale.

Cargo behavior is generated only through a stable, fixture-proven public `rules_rs` or patched
`rules_rust` mapping. The accepted target kinds are ordinary libraries, binaries, tests, proc macros,
sole `cdylib`, and sole `staticlib`. Unsupported kinds, `dylib`, and multiple crate types for one
source owner fail with manifest context and a user-owned kept-target route; generation does not coerce
kinds or duplicate the crate root.

A nonempty Cargo target `required-features` list remains unsupported until upstream exposes exact
first-party target admissibility. Generation reports the target, features, and manifest and points to
a kept handwritten `testonly` target with explicit `crate_features`; it does not enable features or
emit the target unconditionally.

Production crates resolve active normal dependencies. Tests, examples, and benchmarks may add only
their authoritative development scope, in every code position: Cargo links `[dev-dependencies]`
into those targets unconditionally, so a dev-declared name counts wherever its use item sits.
Optional and feature-gated crates resolve only when already enabled for that target.

Declared first-party path dependencies are also mirrored without detection evidence: Cargo links
every declared dependency into a target while import detection only sees `use` items, so an
expression path such as `api::digest(words)` still resolves through the rule index. Mirror misses
and ambiguities stay silent (an unused declared dep is legal; rustc reports a used one precisely),
while detected imports keep strict fail-closed resolution. Bins, tests, examples, and benches link
the same-package library automatically; unit-test wrappers stay narrow because their `crate` edge
already carries the package. Emitted library flavors (`rust_library`, `rust_proc_macro`,
`rust_shared_library`, `rust_static_library`) are `//visibility:public` so cross-crate workspace
edges analyze; bins, tests, and scripts stay private. Files that already declare a default
visibility keep it.

A first-party path dependency carrying a `version` requirement resolves single-version: the
provider's `[package] version` must satisfy the depender's requirement (caret, tilde, exact, and
ordered clauses), else generation fails naming both manifests. Unconstrained path edges pass.

## Native Config

The extension recognizes four checked-in tool configs by exact basename and generates one typed
target per file named `<tool>_config` (`buildifier_config` for `.buildifier.json`, `taplo_config`
for `taplo.toml`, `vale_config` for `.vale.ini`, `rustfmt_config` for `rustfmt.toml`). Vale style closure walks the INI `StylesPath` (default `styles`) into sorted
`data` labels. Generated targets carry `//<package>:__subpackages__` visibility (`//:__subpackages__`
at root). Only rustfmt binds to Rust rules, through direct `aspect_hints` entries;
buildifier, taplo, and vale targets exist for their own language owners. Clippy is
intentionally unmanaged: Rust lint is upstream-delegated and a `clippy.toml`
belongs to the `rules_rust` `clippy.toml` label flag, never to a dx-side config target,
so `clippy` is not a valid `dx_native_tools` id.

`# gazelle:dx_native_tools <tool...>` selects the managed set per directory (nearest wins,
inherits otherwise; absent means all). Unknown ids and bare directives fail before BUILD emission,
as do multiple same-kind config targets and same-name different-kind claims. A hand-authored
same-kind target always wins and is never stubbed; only generator-shaped rules stub out on file
removal or deselection. Binding merge, lifecycle, and closure semantics follow
[native configuration](../quality/native-configuration.md#binding).

## Tests

Each direct `<crate-directory>/tests/*.rs` file is an independently runnable integration-test crate
root regardless of basename. Direct names such as `common.rs`, `helper.rs`, and `helpers.rs` are not
reserved. Shared helpers belong in loaded nested modules such as `tests/common/mod.rs`; imports never
reclassify a direct root.

An integration-test target is `<normalized-source-basename>_test`, without duplicating an existing
`_test`. An unambiguous explicit Cargo `[[test]]` name and path take precedence and may locate the root
outside `tests/`. Missing or conflicting declarations and naming or ownership collisions fail without
fallback inference.

Explicit `[[test]] harness = false` maps to the tested upstream no-libtest setting. Other declared
tests and source-only direct roots use libtest; source syntax or a source-provided `main` never
implicitly disables it.

Each library or executable crate whose owned module tree contains parsed `#[test]` or parsed
`#[cfg(test)]` syntax receives one crate unit-test target named
`<normalized-crate-target-name>_test`. It uses the upstream crate-testing edge and does not own the
sources again. Comments, strings, and unexpanded macro output are inert; additional forms require
parser fixtures. No recognized syntax means no generated unit-test target.

References exclusive to recognized unit-test regions attach only to the crate unit-test target and
may use active development dependencies. Production-reachable references remain on the production
crate. Ambiguous parser context fails rather than widening production scope or dropping an edge.

## Examples And Benchmarks

Only explicitly declared ordinary-binary Cargo `[[example]]` targets are generated. They are normal
non-`manual` binaries named `<normalized-Cargo-name>_example`, retain the unsuffixed Cargo logical
name as metadata, participate in broad builds, and use development dependencies. No source-only
`examples/` discovery occurs.

Absent or false `test` creates no Bazel test. Explicit `test = true` adds a crate-test wrapper named
`<example-target>_test` without duplicate source ownership. Non-binary or multi-kind examples and
nonempty `required-features` remain unsupported under their upstream gates.

Only explicitly declared ordinary-binary Cargo `[[bench]]` targets with `harness = false` are
generated. They are normal non-`manual` binaries named `<normalized-Cargo-name>_bench`, retain the
Cargo logical name, use development dependencies, and participate in broad builds but never Bazel
test patterns. Source-only `benches/` discovery, libtest/nightly benchmarks, absent or true harness,
non-binary or multi-kind benchmarks, and nonempty `required-features` are unsupported.

All example and benchmark naming collisions fail without another affix.

## Build Scripts

Cargo's conventional `build.rs`, explicit `package.build` path, and `package.build = false` are
authoritative. An active script is generated only through the stable public upstream build-script
mapping after complete execution, metadata, generated-output, provider, cross-platform, and
cross-compilation fixtures pass. The generated rule is
`<normalized-Cargo-package-name>_build_script`; the unsuffixed Cargo package name remains metadata.

The script runs later as a Bazel execution action, never during generation. Generation does not parse
the script to infer behavior or outputs, recreate Cargo's protocol, or infer `build.rs` for a
source-only crate. Cargo-declared build dependencies are automatic, and script outputs connect to all
affected first-party crate targets.

User-owned upstream `data`, `tools`, `build_script_env`, `build_script_env_files`, and `toolchains`
attributes must be explicit and protected with `# keep`; generation preserves but never infers them.
Missing or incomplete upstream representation fails every affected target with package, script,
manifest, unsupported-semantics, and handwritten kept-target guidance.

Generated build-script rules set:

- `use_default_shell_env = False`.
- `use_cc_toolchain = True`.
- `emit_warnings = True`.

The selected hermetic C/C++ toolchain is exposed through the upstream build-script rule by default,
without inferring native compilation from script contents. An explicit `# keep`-protected
`use_cc_toolchain = False` opts out, avoiding those toolchain inputs for scripts that do not need
them. Additional tools, headers, libraries, and environment remain explicitly declared; this default
does not enable ambient tool discovery or activate an unused Rust foundation.

The first three settings may differ only through an explicit kept override; otherwise
generation restores the stated value. `emit_warnings` forwards `cargo::warning` through upstream
Bazel behavior, subject to the upstream global setting, and creates no private warning stream.

CC opt-out linker path is qualified seed-only (issue #471): on the as-built
stock `rules_rust` 0.74.0 stack the kept opt-out executes successfully for
pure-Rust scripts. Opt-out concerns the script execution action, not removal
of the linker needed to build the script executable (that binary still
compiles through the normal Rust toolchain). With `use_cc_toolchain = False`
the execution action carries no C++ toolchain inputs — `CC`/`CXX`/`AR` point
at the upstream `no_cc`/`no_cxx`/`no_ar` stubs with empty
`CFLAGS`/`CXXFLAGS`/`LDFLAGS` — while `LD` falls back to the sysroot
`rust-lld`. The historical rules_rs no-linker report does not reproduce here;
a script that actually needs CC still fails clearly when opted out. Proven by
`rust/tests/fixtures/cc_optout/` (`cc_optout_build_script` with kept opt-out
plus `cc_optout` lib plus `cc_optout_test` under `bazel test //...`,
`bazel run //tools/ci:cc_optout_qualification`); silent kept opt-out stays
rejected. Scope is pure-Rust scripts on the seed host; other platforms and
bindgen/CXX/exact-target stay owned under issues #410-#414 and #473-#475
with no `Supported` claim.

Third-party shell-env contract (decided under issue #472; ambiguous default
rejected): `crate_universe`-generated `cargo_build_script` targets render
without an explicit `use_default_shell_env` and defer to the global
`@rules_rust//cargo/settings:use_default_shell_env` flag, pinned `False` in
`.bazelrc`. First-party generation sets `use_default_shell_env = False`
per rule (above); the global flag closes the third-party half so every
build script, first- or third-party, runs without the host shell
environment. Under empty and hostile ambient `PATH`, home, locale, and
arbitrary shell variables, declared tools and environment behave
identically while undeclared host-tool lookup fails at the upstream action
boundary. A crate that genuinely needs host `PATH` probing opts back in
narrowly and only via a per-crate
`crate.annotation(crate = "<name>", build_script_use_default_shell_env = "on")`
in `MODULE.bazel` (currently zero opt-ins); never flip the global back to
`True`. Additional tools, headers, libraries, and environment stay declared
through the annotation's `build_script_tools`, `build_script_data`,
`build_script_env`, and related attributes. First-party
`use_default_shell_env = True` likewise survives only with `# keep`.

## Versions

Rust omits per-target toolchain-version selection under
[ADR 0012](../decisions/0012-language-toolchain-versions.md): the pinned upstream only accepts a
single toolchain version, so there is no supported version-selectable API to map. The effective
default is the single pinned toolchain (`MODULE.bazel`); `edition` defaults to `RUST_EDITION`
(`rust/rules/defs.bzl`, currently `"2021"`) with a
per-target override. Cargo `[package] version` stays authoritative for the generated build-script
`version` attribute and single-version path-dependency checks, never for toolchain selection.
