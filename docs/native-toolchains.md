# Native Toolchain Qualification

Engineering qualification plan. The choices below select what to qualify
first, not an implemented public API, frozen release manifest, or verified
support. [ADR 0014](decisions/0014-tested-platform-release-stack.md#decision)
retains authority over hermetic acquisition, Windows interoperability,
required Linux profiles, and bounded cross-build scope.
[Support Matrix](product/support-matrix.md) owns support and
foundation-admission claims. Open work is tracked in GitHub issues.

## Status And Boundary

Research inspected pinned upstream source, documentation, releases, and CI.
No native build, test, coverage, remote-execution, or acquisition result below
is an implementation approval on its own; source-derived failure paths need
focused reproduction before patches are chosen.

As-built CI builds, tests, and gates coverage natively on each required host
through the pinned upstream toolchains (`rules_rust`, `aspect_rules_py`,
`aspect_rules_js`/`aspect_rules_ts`, plus the admitted foundations' rulesets):

- Linux x86_64 glibc (seed) and Linux arm64 glibc (`ubuntu-24.04-arm`).
- Linux static-musl for x86_64 (`ubuntu-latest`) and arm64 (`ubuntu-24.04-arm`)
  via Rust `extra_target_triples`; build scripts and proc macros stay on
  execution-platform tools while applications link target musl libraries.
  Dynamic musl stays out of scope. Prebuilt glibc libraries never become
  musl-compatible by linker change alone.
- macOS arm64 (`macos-14`). macOS x86_64 is not planned and has no runner.
- Windows x86_64 MSVC-compatible (`windows-latest`).

The `rules_rs`/`hermetic-llvm` backends and the `toolchains_msvc`
clang-cl/Microsoft-STL backend stay provisional candidates, not the qualified
backends. No host-installed SDK or Build Tools fallback is approved. Exact
pins, hosts, floors, and SDK/CRT identities live with the platform track, not
in this plan.

## Selected Qualification Stack

| Layer | First choice | Why and boundary |
| --- | --- | --- |
| C/C++ build semantics | `rules_cc` | Reuse `cc_library`, `cc_binary`, `cc_test`, `cc_import`, `CcInfo`, and the standard C++ toolchain. Qualify `cc_shared_library` separately where needed; no replacement compiler rules. |
| Rust | `rules_rs` and its pinned patched `rules_rust` | Retain the preferred foundation and provider identities in ADR 0013; do not compose a second independent Rust rules graph. |
| Linux/macOS native backend | `hermetic-llvm` + LLVM | Most complete inspected acquisition, target-runtime, LLVM-tool, and coverage integration. |
| Windows native backend | `toolchains_msvc`, clang-cl + lld-link + Microsoft STL | First Windows-hosted candidate with existing no-install toolchains and the required STL family. Prototype acquisition and coverage gaps block adoption unchanged. |
| Windows compatibility control | `cl.exe` through the same backend | Use to diagnose actual MSVC-library/compiler compatibility failures, not as a second default or an automatic fallback. |
| Coverage | Upstream Bazel-owned LLVM source coverage and Rust collection | Keep instrumentation and execution upstream; validate complete LCOV rather than build-only success. |
| Binding generation | Existing `rules_rs` standalone bindgen; upstream CXX generation | Prefer declared Bazel native libraries and generators; Cargo build-script binding generation is a separate qualified route. |
| C/C++ generation and IDE | Reuse `gazelle_cc` capability and action-derived compile-command extraction where conforming | Neither inspected generator nor Hedron extractor is a drop-in implementation of the accepted contracts. |

Choose clang-cl before `cl.exe` because it already supports Microsoft STL in
this backend and offers an LLVM instrumentation path consistent with the other
platforms. This is a qualification advantage, not proof of Windows coverage or
universal MSVC compatibility.

Use the exact current stable Bazel and Rust compilers when qualification begins
under [dependency currency](decisions/0008-dependency-currency.md). Do not
silently inherit an older compiler default or turn a research version into a
release pin.

### Inspected Identities

Research observed `rules_rs`, `hermetic-llvm`, `rules_cc`, `toolchains_msvc`,
`windows_support`, `gazelle_cc`, and the Hedron extractor at pinned upstream
releases/heads. Recheck latest stable and verify actual bytes when qualifying;
research observations are not release pins. Freeze compiler archives, runtime
sources, upstream patches, SDK manifests, and package hashes separately from
ruleset source hashes.

### Alternatives Not Selected

- Uber `hermetic_cc_toolchain` supplies useful Linux cross-compilation but adds
  external runtime-cache, Apple SDK, and coverage/tool integration ownership.
  Keep it as a comparison, not a second default backend.
- `toolchains_llvm` is useful with already-qualified sysroots but leaves more
  libc/SDK/runtime acquisition to the consumer.
- Released hermetic-llvm uses MinGW on Windows; its unreleased MSVC work
  requires libc++ and rejects MSVC coverage. Neither is the Microsoft-STL
  solution required here.
- Direct `rules_rust` has useful Cargo annotations and IDE APIs but does not
  solve Windows acquisition or native coverage. Retain it as the established
  alternative if bounded remediation fails.
- Do not create a project-owned compiler backend around `windows_support`,
  substitute cargo-zigbuild for Bazel toolchains, or fall back to installed
  Visual Studio/SDKs.

## Profiles And Cross Builds

Start with upstream-native runtime choices rather than another distribution
stack. These are qualification configurations, not new public profile names or
accepted minimum-OS support claims.

| Target profile | Initial configuration | Qualification boundary |
| --- | --- | --- |
| Linux x86_64/arm64 glibc | Upstream glibc symbol floor; libc++; ordinary dynamic glibc linkage | Application glibc implementations come from deployment systems, not link stubs. Native x86_64 plus native arm64 are qualified; verify test linkage as well as binaries. |
| Linux x86_64/arm64 static musl | Upstream musl static native closure; non-PIE first for Rust compatibility | Static musl only; dynamic/shared musl stays excluded with no cell and no coverage. |
| macOS arm64 | Pinned acquired Apple SDK, SDK libc++ headers/system dynamic libc++ | SDK version is not deployment floor. No host-installed SDK fallback, no secrets, no interactive acceptance. |
| Windows x86_64 MSVC | clang-cl, Microsoft STL/UCRT/VCRuntime, retail dynamic CRT `/MD` | Align Rust CRT mode, iterator-debug settings, system libraries, and redistributable deployment. `/MT` plus debug CRT are not assumed interchangeable. |
| Linux GNU C++ prebuilt compatibility | Explicit dynamic libstdc++ alternative, not default libc++ substitution | Upstream supports it only on Linux glibc. |

Prefer ordinary object linking without cross-language LTO initially. Shared
runtime, exceptions, RTTI, allocation ownership, and ABI boundaries need
explicit interop evidence; LLVM ancestry alone is not interoperability
evidence.

| Compiler execution platform | Targets to qualify | Order |
| --- | --- | --- |
| Linux x86_64 | Linux x86_64 and arm64, each glibc and static musl | First Linux cross-build cohort |
| Linux arm64 | Linux arm64 and x86_64, each glibc and static musl | First Linux cross-build cohort |
| macOS x86_64 | Not planned | Clean `unsupported_platform` refusal |
| macOS arm64 | Native macOS arm64 | Required native workflow |
| Windows x86_64 | Native Windows x86_64 MSVC | Required native workflow |

Other routes are outside the initial qualification cohort, not claims of
impossibility or permanent product exclusions. Do not require
Linux/macOS-to-Windows, Linux/Windows-to-macOS, Windows-to-Linux, or Windows
arm64 to complete this cohort. Do not weaken required native workflows to
obtain a larger cross-build table.

Record the actual action execution platform, not only the Bazel client or CI
runner OS. Run resulting artifacts on matching native target workers;
cross-building alone is insufficient. Emulation, Rosetta, remote execution, and
deployment-floor testing are distinct evidence.

## Rust And Native Integration

Use public upstream toolchains, `CcInfo`, Rust crate/dependency/build
providers, and existing execution transitions. `rules_rs` static/shared-library
wrappers map Cargo `staticlib` and `cdylib`; exact public `rules_dx`
wrapper/provider mappings are pinned with conformance tests, not as a
re-export of every upstream API.

The [build-script contract](generation/rust.md#build-scripts) remains
authoritative: native compiler exposure defaults on with a kept opt-out; shell
environment and nonhermetic-path discovery default off. Build-script
executables, generators, and proc macros run on the execution platform. Native
compiler executables also run there but compile application dependencies for
the target platform.

Prefer already-declared Bazel native libraries where integration exists, while
retaining upstream Cargo build scripts for authoritative manifests.

One source-level blocker still needs narrow upstream remediation: the pinned
build-script runner still requests linker arguments after opting out of the
C++ toolchain, while the generated Rust toolchain supplies no standalone
linker for ordinary desktop targets. On the as-built stock stack the kept
opt-out executes successfully for pure-Rust scripts. Third-party shell
environment is decided hermetic with a global default plus a narrow per-crate
opt-in.

For explicit binding generation, use `rules_rs` `rust_bindgen` at pinned
versions with the standalone route deriving the target compiler context and
the build-script route carrying an explicit execution-platform libclang
closure. For CXX, use the upstream generation pattern with identical
`cxx`/`cxxbridge-cmd` versions from the single crate-universe graph; do not
introduce a second Rust graph merely to acquire the generator.

## Windows Acquisition And Compatibility

The selected prototype uses standard rules_cc toolchains, so public Rust/C++
composition is plausible. The following source facts prevent describing it as
a drop-in hermetic stack:

- Acquisition resolves live manifests and minor-version selectors. Individual
  payload checksums and a pinned rules commit do not freeze clean
  re-resolution. Prefer a narrow upstream fixed-manifest and package-index
  input over a separate project-owned downloader.
- SDK extraction needs Windows hosts. No cross-host Windows route is established.
- The extension reads the Microsoft EULA acceptance variable through repository
  environment. Consumer acknowledgement stays deliberate: export the variable
  plus pass `--repo_env=...` after reviewing the applicable Microsoft terms,
  never automatic via `.bazelrc` or wrapper defaults. Missing acknowledgement
  fails with an actionable error before restricted payload download; unrelated
  workflows stay green.
- Toolchain registration constrains Windows/CPU but not the LLVM MSVC ABI
  constraint used by rules_rs. Qualify an explicit constraint fix.
- clang-cl flags, link inputs, batch wrappers, response files, spaces, SDK
  libraries, cc-rs discovery, and assembly tools all need declared-input
  fixtures without host Visual Studio state.
- The backend has no complete coverage wiring. A coverage-path normalization
  flag is not instrumentation.

`windows_support` demonstrates exact manifest-integrity and SDK-package
acquisition without an installer. Reuse upstream acquisition capability if
composition is narrow; do not write a new toolchain just to combine the two.
Review applicable terms rather than inferring them from a variable setting.

Qualify independently compiled MSVC static/import libraries and DLLs,
including STL values, exceptions, RTTI, and allocation ownership, under
Microsoft's compatibility limits and Clang's MSVC compatibility. Do not promise
arbitrary `/GL`/`/LTCG` objects, CRT combinations, or vendor libraries.

## Coverage Generation And IDE Gaps

The inspected LLVM coverage fixture uses Bazel LLVM coverage flags and a
raw-output workaround. Qualify the workaround or a focused patch, with
nonempty reports and deliberately missed lines. Rust coverage requires
profiler-builtins in the target standard library with compatible collector
tooling. The accepted [coverage policy](testing/README.md#coverage) is not
weakened to match missing collectors.

`gazelle_cc` resolution does not meet strict generation by configuration alone.
Reuse parser/generation capability only with bounded strictness and output
adaptation. Preserve union-of-literal-includes semantics. Named C++ modules,
header units, and PCH have no complete generation/IDE route yet; assess them
explicitly before admission.

Hedron's extractor uses actual action commands but is not the immutable,
exact-target, symlink-only environment contract. Use a narrow action-derived
snapshot adaptation with managed host-native clangd where conforming. Rust's
upstream dynamic discovery accepts paths/buildfiles rather than exact target
labels; resolver-owned exact labels flow to the upstream target interfaces.

## Qualification Questions And Delivery

These are engineering and legal-evidence tasks, not compiler-preference
questions for the user. Open items are tracked in GitHub issues. No item is
resolved by this research alone. Backends stay provisional with no `Supported`
claim until acquisition, interoperability, coverage, and release evidence
passes.

## Runner Plus SDK Rotation

SDK plus floor pin review follows the runner-plus-SDK rotation contract:
quarterly review plus on retirement notice plus on hermetic-llvm release, with
customer-flows-only qualification and no new CI job.
