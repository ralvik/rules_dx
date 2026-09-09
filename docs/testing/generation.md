# First-Party Generation Test Matrix

This matrix covers `dx generate`, first-party Gazelle extensions, language wrappers,
dependency resolution, and BUILD merge behavior. Native quality-config generation and
binding tests are authoritative in
[Quality Workflow Testing](../quality/quality-testing.md#configured-policy).

## Language Consumer Fixtures

For each required core/framework and admitted additional foundation, a clean external consumer on
every required platform exercises generation, build, test, dependency resolution, applicable quality
and coverage, and matching environment/IDE setup together. Use only documented bootstrap
prerequisites, one module dependency, and authoritative application manifests/locks; no separately
installed language runtime or quality tool may mask an acquisition gap. Check unused-foundation
laziness in the same consumer. See [first-release admission](../product/scope.md#first-release-admission).

Any ruleset patch or project-owned missing integration adds focused failure/reproduction fixtures
and reruns the complete affected platform workflow. Packaging and source-built upstream artifacts
retain the [acquisition evidence](tools.md); an approved upstream switch reruns provider, dependency,
generation, and environment compatibility fixtures, not just compilation.

Python fixtures verify that `python_test` always runs pytest, supports discovery
and representative plugins, reports through Bazel's standard test protocol, and
participates in `bazel coverage`. Supplying a generic main or alternate test driver
must be rejected.

JavaScript/TypeScript fixtures verify that conventional test wrappers use
`aspect_rules_jest`, support Bazel test filtering, emit standard test logs, and
participate in `bazel coverage` without a project-specific result protocol.

TypeScript compiler fixtures prove that `tsc` diagnostics appear under typecheck,
compiler inputs and configuration come from `typescript_project`, and no wrapper
independently enumerates sources or invokes a second compiler outside Bazel. Tests
also prove that lint and test do not duplicate `tsc` actions.

## Generate Command And Gazelle Extensions

- Verify `generate` invokes only the repository-defined Gazelle workflow.
- Verify `generate` is callable in an empty repository and in a repository containing supported
  sources but no authoritative manifest, BUILD file, or application target.
- Verify each supported source-only language package receives its conventional initial targets,
  repeated generation is idempotent, and existing hand-written/generated targets use normal
  Gazelle merge behavior.
- Verify first-party Python, JavaScript, TypeScript, and Rust extensions against golden and
  semantic fixtures covering source grouping, local/import resolution, wrapper kinds, naming,
  tests/binaries, mixed-language packages, deterministic ordering, and package boundaries.
- Verify all first-party extensions use Gazelle desired/empty-rule and merge APIs with explicit
  owned attributes, conservative matching, all `# keep` levels, stale cleanup, and preservation of
  unfamiliar user content. No extension implements a BUILD parser, writer, or ownership database.
- Verify source-only generation does not invent external dependencies, package-manager settings,
  compiler/runtime versions, or other project metadata that belongs to an authoritative manifest.
  For every language, standard-library-only and fully local source graphs generate without an
  ecosystem manifest or lockfile. Every unknown or ambiguous literal dependency reference and every external
  import with missing or stale required lock metadata fails generation with an actionable
  diagnostic. No guessed/omitted dependency, disabled validation, custom unresolved marker, or
  successful incomplete graph is accepted. Standard exact Gazelle mappings and project-owned exact
  ignores remain user-authored escape hatches; mappings are preferred and resolution order is
  identical across extensions.
- Verify lock presence is insufficient without target admissibility. For each ecosystem, production
  imports resolve only from production-selected scope; test imports may additionally resolve from
  explicitly assigned test/development scope. Cover inactive optional dependencies, Python
  groups/extras and environment-only `ide_groups`, Rust `dev-dependencies` and features, pnpm
  `devDependencies`/`optionalDependencies`/importers, and platform variants. Wrong-scope imports fail
  with target, dependency, active scope, and correction location; no group, feature, importer,
  platform, manifest, or lockfile changes.
- Verify identical lockfiles with different authoritative target scope produce the corresponding
  admissible dependency edges or failures, while unrelated locked groups do not enter generated
  attributes or derived target indexes. Explicit mappings and ignores keep their accepted precedence
  without silently activating ecosystem scope.
- Verify each language's tested dependency-bearing runtime-load forms. Ordinary imports and
  recognized forms with literal identities enter the same strict resolver and produce the same
  dependency edges. Recognized computed forms preserve manually declared `# keep` dependencies,
  generate no inferred edge or ignore, and are explicitly outside automatic completeness claims.
  They require no source/directive acknowledgment and emit no notice, warning, diagnostic, or status
  change solely for being computed; text and structured output match the equivalent source with no
  recognized computed form. Arbitrary calls, reflection APIs, and module-looking strings outside a
  language's recognizer do not become dependencies.
- Outside the approved Go build-constraint exception below, verify each language adds the same ordinary dependency for a literal reference whether it is
  unconditional or appears under representative platform checks, feature gates, conditions, or
  exception handling. Generated dependencies are the deduplicated union of literal identities; no
  source condition creates `select()`, feature-specific attributes, computed-reference treatment,
  or different resolution precedence. Exact ignores retain their normal explicit behavior.
- For the approved Go exception, verify declared build constraints and platform source selection
  preserve upstream source/dependency semantics on each required platform. Runtime conditions do
  not become generation policy; unknown active imports still fail strict resolution. Package-level
  test fixtures cover shared helpers, `TestMain`, internal/external test packages, and test-only
  dependencies without forcing independently runnable per-file targets or inferring `testdata`.
- Verify each language computes that union per generated target. Imports present only in test
  sources appear only on the owning test target, while production imports remain on production
  targets and the conventional test-to-production edge is preserved separately. Cover local and
  locked external test-only dependencies, repeated imports, and mixed production/test packages.
  Ambiguous ownership fails with all competing context and never duplicates a source/edge or broadens
  a production dependency set.
- Verify Python, JavaScript, and TypeScript test-classify only supported source basenames ending in
  `_test` immediately before the language extension. Negative fixtures cover `test_` prefixes,
  conventional test directories, test-like source syntax, and custom runner filename settings. For
  Rust, verify every direct `<crate-directory>/tests/*.rs` file is an integration-test root regardless
  of basename, while nested files become modules/helpers only when loaded and never automatic tests.
  Cover direct `common.rs`/`helper.rs`/`helpers.rs` as roots, nested `common/mod.rs` as a helper, and
  imports that never reclassify a direct root.
- Verify JavaScript source discovery for `.js`, `.jsx`, `.mjs`, and `.cjs`, and TypeScript discovery
  for `.ts`, `.tsx`, `.mts`, and `.cts`, across production, `_test`, import/export, dynamic-import,
  CommonJS require, JSX/TSX, entry-point, naming, collision, and stale-cleanup fixtures. Module-format
  semantics follow authoritative package metadata and stable upstream rules, not the extension alone.
- Verify `.d.ts`, `.d.mts`, `.d.cts`, source maps, `.vue`, `.svelte`, `.astro`, `.mdx`, and
  representative unknown/framework extensions are inert to the core JS/TS extensions. Vue, Svelte,
  Astro, and MDX are claimed only by their separate first-release adapters after each passes its own
  stable upstream parser/compiler and full build/test/run/IDE/quality fixture matrix. Unknown formats
  remain inert and no container falls back to plain JavaScript/TypeScript.
- Verify each Rust library or executable module tree containing either parsed `#[test]` or parsed
  `#[cfg(test)]` generates exactly one independently runnable crate unit-test target using upstream
  `rust_test(crate = <crate-target>)`, with
  no duplicate `srcs`. A tree without recognized unit-test syntax generates none. Cover recognized
  syntax in root and nested modules, multiple tests collapsing to one target, direct integration tests
  remaining separate, removal of the last recognized test removing the stale wrapper, and fixture-
  gated additional attribute forms such as `cfg_attr`. Comments, strings, and unexpanded macro output
  containing equivalent tokens do not trigger a wrapper.
- Verify Rust unit-test wrappers use `<normalized-crate-target-name>_test`. Integration tests use
  `<normalized-source-basename>_test`, append no duplicate suffix when the basename already ends in
  `_test`, and defer to an unambiguously mapped explicit Cargo `[[test]]` name. Cover unit/integration,
  Cargo/source, generated/handwritten, and normalization collisions; all fail with every claimant and
  no invented affix.
- Verify references exclusive to recognized Rust unit-test regions attach only to the crate unit-test
  target and resolve from its active `dev-dependencies` scope. Production-reachable references remain
  on the tested crate even when tests also use them. Cover library and executable crates, nested `#[cfg(test)]`
  modules, standard/local/external references, inactive development scope failures, and ambiguous parser
  contexts that fail instead of widening production or omitting an edge.
- Except for the accepted [native Go package-level tests](../generation/common.md#ownership-and-naming),
  verify every recognized test source in each supported language generates a separate independently
  runnable test target, with deterministic naming and no package, crate, or runner-level aggregate
  substituted implicitly. Python/JavaScript/TypeScript names remove only the final language extension
  and retain `_test`; Rust integration names use the explicit rule above. All basename-derived names
  use the shared normalizer, and same-package normalized-name collisions fail with all claimants and
  no implicit affix. Multiple files in one package retain isolated source and dependency
  ownership. Python, JavaScript, and TypeScript tests depend on ordinary one-source libraries without
  absorbing them based on importer count. Rust test roots own their recursively loaded modules and
  may depend on separately authored crates; verify equivalent runtime semantics, no duplicate source
  ownership, unrelated production edge, or synthetic per-module crate. If an upstream ruleset cannot
  provide the required shape, verify the foundation remains blocked. Exact naming and Rust
  root/module recognition follow their separately frozen fixture contracts.
- Verify every supported non-test Python, JavaScript, and TypeScript source receives one ordinary
  reusable one-source language-library target, independent of importer count. Imports produce edges
  between those targets; tests do not absorb imported non-test sources. Verify target names use the
  source basename without its final language extension, normalize non-alphanumeric separators to
  underscores, and fail same-package normalized-name claims across supported extensions with every
  claimant and no invented affix. Verify Rust instead produces
  one owner per authoritative crate root containing all recursively loaded modules, including nested
  modules and explicit `#[path]` cases. Reject per-module `rust_library` rewriting, duplicate module
  ownership, orphan modules, crate-root ambiguity, and test-module sharing that cannot preserve
  native crate semantics.
- Verify Python `.py` runtime owners may carry a same-directory, same-basename `.pyi` through the
  pinned upstream type-metadata shape. The paired stub creates no additional target, test, binary,
  entry point, runtime dependency edge, or normalized-name collision. Stub imports affect only the
  supported type-analysis path. Cover add/remove/rename stale merge, a `_test.pyi` paired with
  `_test.py`, and orphan `.pyi` files that remain fully inert without diagnostics or status changes.
- Verify source-only Rust crate roots with nested local modules and standard-library references
  generate, build, and test without `Cargo.toml` or `Cargo.lock` using tested ruleset defaults and a
  deterministic fallback Bazel name. With Cargo metadata, verify declared package/target names,
  paths, editions, features, build scripts, and dependency scopes take precedence. Reject any
  inferred Cargo policy and fail sources that require missing Cargo-owned metadata with its expected
  declaration location.
- Verify deleting the last owning source or authoritative manifest declaration emits a matching empty
  rule and removes the stale generated application target when ordinary Gazelle merge permits it.
  Cover library, test, binary, source rename, manifest-entry removal, complete-rule/attribute/value
  `# keep`, unfamiliar user attributes/comments, incompatible handwritten kinds, idempotence, and an
  empty retained BUILD file. Edited stale rules are preserved with successful status and no partial
  recognized-field stripping. No unrelated rule or BUILD file is deleted.
- Verify conventional source-only `<crate-directory>/src/lib.rs` and `src/main.rs` roots derive their
  fallback from the normalized `<crate-directory>` basename, independent of checkout path and host.
  Cover punctuation normalization, empty/colliding names, nested crate directories, and coexisting
  roots; the latter retain one library owner and a `<normalized-crate-name>_bin` thin binary without
  duplicate module ownership. Cargo-declared binary names remain authoritative.
- Verify source-only Rust rejects standalone/custom roots, `src/bin`, examples, benchmarks, and other
  nonconventional target layouts with a Cargo target-declaration diagnostic. Equivalent roots named
  and mapped through authoritative Cargo metadata generate normally without source-derived naming or
  path inference.
- Verify explicit Cargo `[[test]]` declarations generate authoritatively named test roots both inside
  and outside `tests/`. Cover custom paths, names needing Bazel normalization, missing files,
  duplicate names, one name mapped to multiple paths, multiple names claiming one source, overlap
  with default direct roots, module/source ownership conflicts, and collisions with crate unit tests,
  binaries, integration tests, and handwritten targets. Invalid declarations never fall back.
- Verify explicit Cargo `[[test]] harness = false` maps to upstream
  `use_libtest_harness = False`, executes the source-provided `main` under Bazel's test protocol, and
  preserves test exit/log behavior. Declarations without the field and source-only direct
  `tests/*.rs` roots retain libtest. A source `main`, custom framework-looking syntax, or missing test
  attributes never disables libtest implicitly; malformed custom-harness sources fail at normal
  compile/test boundaries.
- Verify Cargo semantics are accepted only through stable fixture-proven public upstream mappings.
  A test with absent or empty `required-features` follows normal behavior; a nonempty list fails with
  target/name, every required feature, manifest location, the missing stable upstream-admissibility
  boundary, and guidance for a user-owned kept `testonly` target with explicit `crate_features`.
  Verify no unconditional target, feature activation, private `rules_rs` data dependency, or project-
  owned Cargo feature resolver.
- Verify the Cargo target-kind matrix against pinned public upstream wrappers: ordinary library,
  binary, test, proc macro, sole `cdylib`, and sole `staticlib`. For each supported kind, cover exact
  rule selection, crate name/root, dependency and proc-macro edges, providers/output groups, platform
  matrix, downstream consumption, quality/IDE visibility, and stale cleanup. Reject `dylib`, multiple
  crate types on one root, unknown kinds, and every missing-wrapper case with target/kinds/manifest
  context and a kept handwritten-target route. No kind coercion, duplicate root ownership, or private
  patched-symbol load is accepted. Build scripts, examples, and benchmarks follow their accepted
  workflows below.
- Verify explicitly declared ordinary-binary Cargo examples generate non-`manual` Rust binaries and
  participate in broad build patterns. An absent/false `test` field creates no Bazel test; explicit
  `test = true` creates one crate-test wrapper over the example without duplicate source ownership.
  Cover development-only dependencies, custom paths, harness pass-through where supported, stale
  cleanup, broad build/test patterns, non-binary/multi-kind failures, nonempty `required-features`
  failure, and an inert source-only `examples/` directory.
- Verify example labels use `<normalized-Cargo-name>_example`, preserve the unsuffixed Cargo
  crate/logical name, and use `<example-target>_test` for `test = true`. Cover normalized collisions
  with libraries, binaries, integration tests, crate-test wrappers, other examples, and handwritten
  targets; no additional affix is invented.
- Verify explicit ordinary-binary Cargo benchmarks with `harness = false` generate non-`manual`
  `rust_binary` targets named `<normalized-Cargo-name>_bench`, preserving the Cargo logical name and
  development dependency scope. Broad build patterns compile them; explicit Bazel build/run works;
  broad and explicit Bazel test selection does not treat them as test rules. Reject absent/true
  harness, libtest/nightly `#[bench]`, non-binary/multi-kind, and nonempty `required-features` with
  actionable kept-target guidance. Cover collisions, stale cleanup, and inert source-only `benches/`.
- Verify conventional Cargo `build.rs` and explicit `package.build` paths generate only through the
  pinned public `rules_rs` build-script mapping and connect to every affected first-party crate
  target. Verify `package.build = false` suppresses conventional discovery and stale generated script
  targets are cleaned through normal merge. Cover script compilation, normal/build dependency
  separation, aliases, Cargo-provided environment, emitted `cfg`/environment/link metadata, generated
  files consumed by the crate, rerun metadata, provider/output propagation, rust-analyzer and quality
  visibility, remote execution, and each tested host/target cross-product. Verify scripts run as Bazel
  exec actions and never during `dx generate`.
- Verify build-script labels use `<normalized-Cargo-package-name>_build_script`, preserve the
  unsuffixed package name as metadata, and normalize with the shared ASCII/run-collapse/edge-trim
  algorithm. Cover collisions with crate targets, tests, examples, benchmarks, another normalized
  package name, and handwritten targets; every claimant is reported and no further suffix is added.
- Verify Cargo-declared build dependencies are generated automatically. User-authored upstream
  `data`, `tools`, `build_script_env`, `build_script_env_files`, and `toolchains` values protected by
  `# keep` survive repeated generation and enable representative protobuf, bindgen, and native-code
  scripts. Without explicit values, generation does not infer package files, executable tools,
  environment, or toolchains from source, manifests, names, directories, or broad globs.
- Verify every generated build-script rule sets `use_default_shell_env = False`. Run with empty and
  hostile ambient `PATH`, home, locale, and arbitrary shell variables; declared tools/environment
  behave identically while undeclared host-tool lookup fails at the upstream action boundary. Verify
  an explicit `use_default_shell_env = True` survives repeated generation only with `# keep`, and an
  unkept value is restored to `False`.
- Verify every generated build-script rule defaults to `use_cc_toolchain = True`. Representative
  `cc` scripts use the selected hermetic compiler without a kept opt-in. CMake, bindgen, and other
  native scripts still fail clearly without required additional declared tools, headers, or libraries,
  and succeed when those inputs are supplied. Verify execution-platform tools compile for the selected
  target in cross-build fixtures without ambient compiler/SDK discovery.
- Verify an explicit `# keep`-protected `use_cc_toolchain = False` survives repeated generation,
  while an unkept false value is restored to `True`. Opted-out pure-Rust scripts have no C/C++
  toolchain action inputs; scripts requiring those tools fail clearly when opted out. Measure cold
  downloads, action inputs, and compiler-update invalidation for the default and opt-out paths.
  Generation alone and an unused Rust foundation still activate no native-toolchain payload.
- Verify every generated build-script rule sets
  `allow_build_script_to_detect_nonhermetic_paths = False`. Scripts that emit absolute host paths
  through `rustc-link-search`, `rustc-env`, or metadata fail with the pinned upstream diagnostic in
  local, sandboxed, and remote execution. An explicit kept true override survives regeneration and
  permits the fixture; an unkept true value is restored to `False`. Relative and declared generated
  paths continue to work without an override.
- Verify every generated build-script rule sets `emit_warnings = True`. A representative
  `cargo::warning` appears exactly once on Bazel stderr, not during `dx generate`; the pinned upstream
  global force-off suppresses it and force-on overrides a kept per-rule false value. Warning text does
  not enter the private generation result manifest or alter generation status.
- Verify a missing public mapping, manifest-declared input/tool/native dependency that lacks an
  accepted mapping, absent required exec platform, or other statically known incomplete upstream
  representation fails generation for every affected target with package, script path, manifest,
  exact unsupported semantics, and kept-attribute or kept-target guidance. Unsupported output
  discovered only when the script executes fails the upstream Bazel action with its native
  diagnostic. No partial crate generation, private `rules_rs` graph access, source interpretation,
  Cargo-protocol reimplementation, or silent omission is accepted. A source-only `build.rs` is never
  inferred as a build script.
- Verify basename-derived names preserve ASCII letters/digits and internal underscores, collapse
  runs of punctuation/whitespace/non-ASCII characters to one underscore, trim edge underscores, and
  reject empty results. Cover normalized collisions across separator spellings, languages, library
  versus test targets, and host filesystems; every diagnostic reports all claimant source paths.
- Verify every language treats test and production data/resources as user-owned merge content.
  Representative literal/computed file APIs, static inclusion constructs, framework loaders,
  resource manifest fields, conventional directories, missing files, and declared/undeclared
  generated producers create no inferred or validated `data`/resource entry, glob, notice,
  diagnostic, or status change. Existing handwritten values and all `# keep` levels survive normal
  Gazelle merge behavior; removing source resource access does not delete kept values.
- Verify recognized mixed importable/executable sources generate one reusable library source owner
  and one thin binary that depends on it. The source and its source-derived dependencies appear only
  on the library; execution through the binary is equivalent to the language's direct entry-point
  semantics. Fixtures reject binary-only ownership, duplicate source/dependency attributes, and
  unsupported upstream launcher adaptation rather than accepting a degraded shape.
- Verify packages with zero, one, and multiple recognized entry points. Every entry receives one
  thin binary over the same library owner. Authoritative manifest names win only for exact mappings;
  entries without one use deterministic package-relative source-path names independent of discovery
  order and host path syntax. Fixtures reject duplicate authoritative names, ambiguous mappings,
  normalized-name collisions, invented suffixes, omitted entries, and arbitrary default selection.
- Verify each first-party extension gives an effective exact `# gazelle:resolve` mapping precedence
  over local and authoritative external lookup, uses `# gazelle:dx_ignore_import` only as the final
  exact-match exception, and never creates either directive. Reject regex/prefix ignores,
  conflicting equally effective mappings, mapping-plus-ignore conflicts, and stale ignores that
  match no parsed literal dependency reference in their effective subtree.
- Verify every accepted ignore emits one deterministic `ignored_import` notice per distinct source
  path/language/import in JSON and concise text output, including repeated-import deduplication and
  stable ordering. The notice does not alter status, diagnostic counts, or SARIF and is suppressed
  in patch-only diff output. Notices derive from the canonical Gazelle result manifest; the CLI
  does not parse sources or rerun generation to reconstruct them.
- Verify generation does not analyze or execute generated targets and activates no build, test,
  dependency-version resolution, application codegen, environment, IDE, target-coupled quality,
  usable application toolchain, or compiler/runtime payload workflow. A declared metadata-index
  preparation may materialize only artifacts already selected and checksummed by the authoritative
  lockfile; it does not install packages for application use or choose/build a different version.
- Verify generation creates or modifies only `BUILD` and `BUILD.bazel` in the consumer source
  tree. Import maps, dependency indexes, provider inventories, and result manifests remain declared
  Bazel/private outputs; fixtures reject `gazelle_python.yaml` and analogous generated
  YAML/JSON/TOML sidecars.
- Verify pre-existing legacy sidecars are inert. Their presence, absence, invalid contents, and
  byte changes produce identical generated BUILD candidates, events, diagnostics, status, and
  Bazel action keys; generation neither updates nor deletes them and emits no warning.
- Verify authoritative metadata compatibility for Python uv/wheels, JavaScript/TypeScript pnpm
  importers/package stores, and Rust Cargo/crate providers. Upstream shape changes fail closed and
  never fall back to a package-manager invocation, lockfile reconstruction, or guessed dependency.
- Once [O48](../open-decisions.md) freezes scoped selection, verify accepted path, label, and
  target-pattern forms, empty-scope handling, scoped freshness, manifest boundaries, and Gazelle
  merge behavior against the [generate contract](../cli/commands/generate.md#invocation-and-scope).
- Verify `generate --check` selects check mode on the same canonical workflow as default
  generate, writes no workspace file, succeeds on current metadata, and emits one exact
  `change` event per stale or missing Gazelle-maintained `BUILD` or `BUILD.bazel` file without reading Git or
  exposing a second public workflow target. Every proposed change produces nonzero status.
- Verify default JSON `generate` emits each exact change before one mutation per terminal create
  or modification attempt, with `applied` only after a confirmed write and `not_applied` after a failed attempt.
  Unchanged files produce no event, and a later Gazelle failure preserves earlier edits and
  matching `applied` events.
- Verify both generate modes derive events from the same exact Gazelle-owned
  result manifest during one execution. Each check event reconstructs the exact candidate from
  the validated source digest, byte ranges, and replacements. JSON stdout contains only
  NDJSON; diff stdout contains only the complete unified patch. Neither reruns Gazelle,
  compares workspace trees, or parses BUILD syntax in Rust.
- Verify `generate` rejects Gazelle application arguments (including flags, modes, index controls,
  and raw directory arguments), distinct from O48-scoped selection. Post-`--` values are accepted
  only as valid Bazel options, never Gazelle arguments.
- Verify `generate` does not scan ownership markers or skip files, preserves Gazelle's
  diagnostics and failure status for ownership conflicts, and adds no Rust-side BUILD
  interpretation.
