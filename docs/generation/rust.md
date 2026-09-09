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
their authoritative development scope. Optional and feature-gated crates resolve only when already
enabled for that target.

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
- `allow_build_script_to_detect_nonhermetic_paths = False`.
- `emit_warnings = True`.

The selected hermetic C/C++ toolchain is exposed through the upstream build-script rule by default,
without inferring native compilation from script contents. An explicit `# keep`-protected
`use_cc_toolchain = False` opts out, avoiding those toolchain inputs for scripts that do not need
them. Additional tools, headers, libraries, and environment remain explicitly declared; this default
does not enable ambient tool discovery or activate an unused Rust foundation.

The first three settings may differ only through an explicit kept override; otherwise
generation restores the stated value. `emit_warnings` forwards `cargo::warning` through upstream
Bazel behavior, subject to the upstream global setting, and creates no private warning stream.
