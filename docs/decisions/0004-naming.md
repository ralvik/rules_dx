# ADR 0004: Naming

## Status

Accepted.

## Context

The platform needs recognizable project-owned names without renaming upstream
projects or creating multiple conventions.

## Decision

The Bzlmod module is `rules_dx`, and the executable is `dx`. Public Bazel rules,
macros, aspects, and providers use conventional names without a `dx_` or
`dx_rules_` prefix. Package and load labels provide the project namespace.
Examples include `starlark_test`, `python_test`, `lint_aspect`, `typecheck_aspect`,
`format_aspect`, and `audit_aspect`.

Tool adapters are internal implementation details and do not automatically create
public rule families for tools such as Ruff or Ty.

Upstream projects retain upstream names, including `aspect_rules_py`. A new public
symbol must describe its Bazel behavior directly and live under the narrowest
suitable `rules_dx` package.

Consumer repositories use a canonical `//dx` package. Its fixed targets are named
`config`, `codegen`, `env`, and `generate`; the package label supplies the namespace, so
targets do not repeat `dx` as a prefix or suffix. Lint, typecheck, format, and audit use the
configuration target with external `rules_dx` aspects rather than consumer-owned
workflow targets.

`setup` is a CLI composition command rather than another canonical Bazel target. It
orchestrates the codegen and environment workflows and commits one shared selection.

First-party generation names each per-source Python, JavaScript, and TypeScript library from the
source basename without its final language extension, normalizing non-alphanumeric separators to
underscores. The durable constraint is basename-only naming with fail-on-collision and no
invented language affixes. The exact character mapping that follows is provisional detail
pending fixture evidence under [O22, O25, and O27](../open-decisions.md); do not treat it as
frozen API through this record. For example, `app/models/user.py` generates `//app/models:user`, while
`app/models/user-profile.ts` generates `//app/models:user_profile`. The name does not include a
language prefix or extension suffix. Multiple supported sources in one Bazel package that normalize
to the same name fail generation with every claimant; extensions do not select one language or
invent suffixes. The exact normalization preserves ASCII letters, ASCII digits, and internal
underscores; replaces each run of every other character, including non-ASCII characters, with one
underscore; and trims leading and trailing underscores. An empty result fails generation. Rust
crate roots use authoritative Cargo names and paths when present. Without Cargo metadata, a
conventional `<crate-directory>/src/lib.rs` or `<crate-directory>/src/main.rs` uses the normalized
`<crate-directory>` basename. For example, `tools/parser/src/lib.rs` uses `parser`. When both roots
exist, the library owns that name and module tree and the source-only thin binary uses
`<normalized-crate-name>_bin`, such as `parser_bin`. A Cargo-declared binary name remains
authoritative. Nonconventional Rust roots require authoritative Cargo target names and paths rather
than another source-only fallback.
Per-file test targets use the same rule: remove only the final supported language extension, retain
the `_test` basename suffix when it remains after normalization, and apply the same exact
normalization. For example,
`app/models/user_test.py` generates `//app/models:user_test`. Same-package normalized test-name
collisions across languages fail without an implicit language affix or second test suffix.
Rust crate unit tests use `<normalized-crate-target-name>_test`. Rust integration tests use
`<normalized-source-basename>_test`; if the normalized source basename already ends in `_test`, the
suffix is not duplicated. For example, `<crate-directory>/tests/login.rs` uses `login_test`, while
`tests/login_test.rs` also uses `login_test`. An explicit authoritative Cargo `[[test]]` name wins
when it maps unambiguously to the source. Every collision between a unit test, integration test, or
other generated/handwritten target fails without another suffix.
The Cargo name also applies to an explicitly declared custom `[[test]]` path outside `tests/`.
An explicitly declared ordinary-binary Cargo example named `<name>` uses the Bazel target
`<normalized-name>_example`. Its Cargo crate/logical name remains authoritative metadata rather than
the suffixed Bazel label. If `test = true`, the crate-test wrapper appends the standard test suffix to
the target, producing `<normalized-name>_example_test`. Every normalized collision still fails; no
additional affix is invented.
An explicitly declared supported Cargo benchmark named `<name>` uses
`<normalized-name>_bench`. The unsuffixed Cargo logical/crate name remains authoritative metadata.
Normalized collisions fail without another affix.
A Cargo package build script uses `<normalized-package-name>_build_script`, whether Cargo selected
the conventional `build.rs` or an explicit `package.build` path. The unsuffixed Cargo package name
remains authoritative metadata. `package.build = false` generates no build-script label. Every
collision with generated or handwritten targets fails without another affix.

## Consequences

- Consumers load concise symbols from explicit `rules_dx` labels, avoiding global
  prefixes while preserving origin at the load site.
- Logical packages are released together in the `rules_dx` module.
- `starlark_test` is the public Starlark-testing facade; `python_test` is the
  conventional name for a project-owned Python test rule when that API is added.
- Canonical consumer labels are `//dx:config`, `//dx:codegen`, `//dx:env`, and
  `//dx:generate`.
- Per-source Python, JavaScript, and TypeScript libraries have concise underscore-normalized basename
  labels, with normalized collisions rejected rather than disambiguated implicitly.
- Per-file tests have the same basename-only labels and collision behavior.
- Rust crate unit tests use `<crate-target>_test`; integration tests append exactly one `_test` unless
  an authoritative Cargo `[[test]]` name applies.
- Cargo examples use `<Cargo-name>_example`, with `<example-target>_test` for `test = true`.
- Supported Cargo benchmarks use `<Cargo-name>_bench`.
- Cargo package build scripts use `<Cargo-package-name>_build_script`.
- Conventional source-only Rust crates derive their fallback name from the directory immediately
  above `src`, while Cargo-declared names remain authoritative.

## Rejected Alternatives

- Renaming upstream dependencies for visual consistency.
- Generic company-specific namespaces.
- Prefixing every public rule with `dx_` or `dx_rules_`.
- Root labels such as `//:dx_env` or redundant labels such as `//dx:dx_env`.
- Inventing a repository-policy name before its responsibility exists.
