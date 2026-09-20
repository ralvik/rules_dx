# Native Toolchain Qualification

## Status And Boundary

Engineering qualification plan. Native-stack
and cross-compilation technical choices are delegated to evidence-based upstream review. The choices below select
what to qualify first, not an implemented public API, frozen release manifest, or verified support.
[ADR 0014](decisions/0014-tested-platform-release-stack.md#decision) retains authority over hermetic
acquisition, Windows interoperability, required Linux profiles, and bounded cross-build scope.
[Support Matrix](product/support-matrix.md) owns support and foundation-admission claims.

Research inspected pinned source, documentation, releases, and upstream CI. No `rules_dx` native
build, test, coverage, remote-execution, or acquisition qualification ran. No implementation work
is approved. Source-derived failure paths below need focused reproduction before patches are chosen.

Linux arm64 glibc native is qualified (issue #410) on the current as-built
stack: CI builds, tests, and gates coverage natively on `ubuntu-24.04-arm`
runners through the pinned upstream toolchains (`rules_rust`,
`aspect_rules_py`, `aspect_rules_js`/`aspect_rules_ts`, plus the admitted
foundations' rulesets), which already resolve `linux/arm64` acquisition.
The `rules_rs`/`hermetic-llvm` first choices below remain provisional
candidates, not the qualified backend. Exact pins, hosts, floors, and
SDK/CRT identities stay owned by issues #410-#414 per
[ADR 0014](decisions/0014-tested-platform-release-stack.md#decision) and
are not pinned here. Per-host quality-tool (`dx_tools`) `linux_arm64`
artifacts stay an owned follow-up gap: on arm64, quality-tool actions fail
with the recorded no-artifact diagnostic, never a silent fallback.

Linux static-musl profiles are qualified (issue #411) on the current
as-built stack: Rust `extra_target_triples` (`x86_64-unknown-linux-musl`,
`aarch64-unknown-linux-musl`) resolve musl std through the pinned
`rules_rust` toolchain with bounded upstream maintenance; hermetic-llvm
static-only musl targets stay the provisional C/C++ backend, not the
qualified backend. Build scripts and proc macros keep execution-platform
tools while applications link target musl libraries per the
[build-script contract](generation/rust.md#build-scripts); prebuilt glibc
libraries never become musl-compatible by linker change alone. CI
cross-builds from Linux runners (`ubuntu-latest` for x86_64 musl,
`ubuntu-24.04-arm` for arm64 musl) with per-profile `bazel-musl-*` cache
scopes, and gates per-cell coverage for both musl cells with no union;
dynamic musl stays explicitly out of scope with no cell. Exact pins,
hosts, floors, and runtime-closure identities stay owned by issues #410-#414 per
ADR 0014 and are not pinned here. The SQLite plus OpenSSL plus ring plus bindgen plus CXX corpus is qualified seed-only under issue #499 (`cc/tests/fixtures/linux_corpus/pins.bzl` via `bazel run //tools/ci:linux_corpus_qualification`, single-crate proof rejected, native only). Hermetic-llvm static-only musl targets stay provisional. PIE qualification uses existing upstream constraints and stays owned as open work under issue #500 (coverage qualified seed-only under issue #501). Verify test linkage as well as binaries; this is not dynamic-musl or musl `cdylib` support. Dynamic/shared musl stays excluded per [ADR 0014](decisions/0014-tested-platform-release-stack.md#decision) with no cell and no coverage: hermetic-llvm supplies static-only musl targets and uber shared-musl capability is not complete Rust/runtime-deployment evidence; do not implement.

macOS arm64 native is qualified (issue #412) on the current as-built
stack: CI builds, tests, and gates coverage natively on `macos-14`
(arm64) runners through the pinned upstream toolchains (`rules_rust`,
`aspect_rules_py`, `aspect_rules_js`/`aspect_rules_ts`, plus the admitted
foundations' rulesets), which already resolve `macos/arm64` acquisition.
The `rules_rs`/`hermetic-llvm` Apple-SDK backend stays provisional with
immutable lazy fetch, not the qualified backend; no host-installed SDK fallback
(never approved). Apple-SDK handling leaks no secrets and needs
no interactive acceptance. Exact pins, hosts, floors, and SDK/CRT
identities stay owned by issues #410-#414 per
[ADR 0014](decisions/0014-tested-platform-release-stack.md#decision) and
are not pinned here. Per-host quality-tool (`dx_tools`) `macos_arm64`
artifacts stay an owned follow-up gap like `linux_arm64`: on macOS,
quality-tool actions fail with the recorded no-artifact diagnostic, never
a silent fallback.

macOS x86_64 best-effort native is qualified (issue #413) on the same
as-built stack: CI builds, tests, and gates coverage natively on
`macos-15-intel` (Intel) runners through the pinned upstream toolchains
(`rules_rust`, `aspect_rules_py`, `aspect_rules_js`/`aspect_rules_ts`,
plus the admitted foundations' rulesets), which already resolve
`macos/x86_64` acquisition. Best-effort by ADR 0014 definition: `macos-13`
retired December 2025, `macos-15-intel` until August 2027; gaps recorded
without blocking required-host release. The `rules_rs`/`hermetic-llvm`
Apple-SDK backend stays provisional with immutable lazy fetch, not the
qualified backend; no host-installed SDK fallback (never approved).
Apple-SDK handling leaks no secrets and needs no interactive acceptance.
Exact pins, hosts, floors, and SDK/CRT identities stay owned by issues #410-#414
per [ADR 0014](decisions/0014-tested-platform-release-stack.md#decision)
and are not pinned here. Per-host quality-tool (`dx_tools`)
`macos_x86_64` artifacts stay an owned follow-up gap like `linux_arm64`
plus `macos_arm64`: on macOS x86_64, quality-tool actions fail with the
recorded no-artifact diagnostic, never a silent fallback.

Windows x86_64 MSVC-compatible native is qualified (issue #414) on the
current as-built stack: CI builds, tests, and gates coverage natively on
`windows-latest` runners through the pinned upstream toolchains
(`rules_rust`, `aspect_rules_py`, `aspect_rules_js`/`aspect_rules_ts`,
plus the admitted foundations' rulesets), which already resolve
`windows/x86_64` acquisition. The `toolchains_msvc` clang-cl/Microsoft-STL
backend stays provisional with immutable lazy fetch, not the qualified
backend; no installed Build Tools or host SDK prerequisite or fallback
(never approved). Explicit Microsoft EULA acceptance stays deliberate
through the upstream repository-env mechanism, never automatic; merely
adding the module requires no acceptance and fetches no restricted
payloads; usage vs redistribution rights are reviewed separately.
Prebuilt-MSVC interop fixtures with explicit STL/CRT/linker/library combos
incl mixed Rust/C/C++ qualify host-to-target build routes plus target
execution separately; compiler-target availability alone is not proof.
Manifest/path/ABI gaps (batch wrappers, response files, `/external:I`
vs `/imsvc` rebasing, `.lib`/`.obj` bare paths, spaces, SDK libraries,
cc-rs discovery/assembly, proc-macro DLLs) are closed with declared-input
fixtures without host Visual Studio state. Exact pins, hosts, floors, and
SDK/CRT identities stay owned by issues #410-#414 per
[ADR 0014](decisions/0014-tested-platform-release-stack.md#decision) and
are not pinned here. Per-host quality-tool (`dx_tools`) `windows_x86_64`
artifacts stay an owned follow-up gap like `linux_arm64`/`macos_arm64` plus
`macos_x86_64`: on Windows, quality-tool actions fail with the recorded
no-artifact diagnostic, never a silent fallback.

## Selected Qualification Stack

| Layer | First choice | Why and boundary |
| --- | --- | --- |
| C/C++ build semantics | `rules_cc 0.2.22` | Reuse `cc_library`, `cc_binary`, `cc_test`, `cc_import`, `CcInfo`, and the standard C++ toolchain. Qualify `cc_shared_library` separately where needed; no replacement compiler rules. |
| Rust | `rules_rs v0.0.109` and its pinned patched `rules_rust` | Retain the preferred foundation and provider identities in ADR 0013; do not compose a second independent Rust rules graph. |
| Linux/macOS native backend | `hermetic-llvm v0.8.19`, LLVM `23.1.0` | Most complete inspected acquisition, target-runtime, LLVM-tool, and coverage integration. |
| Windows native backend | `toolchains_msvc` at `8e2aa4624bbb5a53a94f135e90995f307875d1ad`, clang-cl + lld-link + Microsoft STL | First Windows-hosted candidate with existing no-install toolchains and the required STL family. Prototype acquisition and coverage gaps block adoption unchanged. |
| Windows compatibility control | `cl.exe` through the same backend | Use to diagnose actual MSVC-library/compiler compatibility failures, not as a second default or an automatic fallback. |
| Coverage | Upstream Bazel-owned LLVM source coverage and Rust collection | Keep instrumentation and execution upstream; validate complete LCOV rather than build-only success. |
| Binding generation | Existing `rules_rs` standalone bindgen; upstream CXX generation | Prefer declared Bazel native libraries and generators; Cargo build-script binding generation is a separate qualified route. |
| C/C++ generation and IDE | Reuse `gazelle_cc` capability and action-derived compile-command extraction where conforming | Neither inspected generator nor Hedron extractor is a drop-in implementation of the accepted contracts. |

Choose clang-cl before `cl.exe` because it already supports Microsoft STL in this backend and offers
an LLVM instrumentation path consistent with the other platforms. This is a qualification advantage,
not proof of Windows coverage or universal MSVC compatibility. Switch the candidate only on concrete
compatibility/maintenance evidence; neither compiler removes the acquisition and coverage gates.

Use the exact current stable Bazel and Rust compilers when qualification begins under
[dependency currency](decisions/0008-dependency-currency.md). Bazel 9 is a provisional
initial coverage baseline because the inspected hermetic-llvm coverage fixture requires
it, not a release pin: the release default follows ADR 0008 and the exact seed pin is frozen
in `rust/tests/fixtures/stable_stack/pins.bzl` (Bazel 9.2.0 plus rules_rust 0.74.0 plus
rules_cc 0.2.22 plus Rust 1.98.0, qualified seed-only under issue #494 via
`bazel run //tools/ci:stable_stack_qualification`; ad-hoc compose rejected).
Do not silently inherit rules_rs's
older compiler default or turn a research version into a release pin.

### Inspected Identities

| Component | Identity and primary source | Important distinction |
| --- | --- | --- |
| Rust rules | [rules_rs v0.0.109](https://github.com/hermeticbuild/rules_rs/tree/v0.0.109), commit `b55b132af0c9951807c926768e40222330348632` | Latest published release observed. Its MODULE declares LLVM rules `0.8.18`, not `0.8.19`. |
| Patched Rust rules | [Pinned archive declaration](https://github.com/hermeticbuild/rules_rs/blob/v0.0.109/rs/rules_rust.bzl), commit `e9dd49f22cfa43c75ba30cd9d9bb7d8bdc459dde` | Module version `0.74.0` does not make it stock bazelbuild/rules_rust `0.74.0`. |
| Native rules | [hermetic-llvm v0.8.19](https://github.com/hermeticbuild/hermetic-llvm/tree/v0.8.19), commit `6314688712edf3a95f78642d80393868256b4ef2` | Latest release observed; LLVM `23.1.0`. Reference `v0.8.18` uses LLVM `22.1.8`; upgrading is a real stack change. |
| C/C++ rules | [rules_cc 0.2.22](https://github.com/bazelbuild/rules_cc/tree/0.2.22) | Record the resolved module graph: upstream direct declarations differ. |
| Windows backend | [toolchains_msvc pinned source](https://github.com/Dragnalith/toolchains_msvc/tree/8e2aa4624bbb5a53a94f135e90995f307875d1ad) | Latest head observed; prototype, no published release verified. Declared module `0.0.0` is not a release. |
| Microsoft acquisition alternative | [windows_support v0.4.1](https://github.com/hermeticbuild/windows_support/tree/v0.4.1), commit `43dce21da77d8cb4a486709e34f1d887e69058c6` | Acquisition building block, not a C++ compiler toolchain. |
| C/C++ generation | [gazelle_cc v0.6.0](https://github.com/EngFlow/gazelle_cc/tree/v0.6.0), commit `50dbcbcfd9199c19a50522695c568b9380caabe5` | Strict resolution and ownership conformance remain incomplete. |
| Compile commands | [Hedron extractor](https://github.com/hedronvision/bazel-compile-commands-extractor/tree/abb61a688167623088f8768cc9264798df6a9d10), commit `abb61a688167623088f8768cc9264798df6a9d10` | Inspected source reference, not an approved development-commit dependency exception. |

Freeze compiler archives, runtime sources, upstream patches, SDK manifests and package hashes
separately from ruleset source hashes. For LLVM, the pinned
[compiler archive index](https://github.com/hermeticbuild/hermetic-llvm/blob/v0.8.19/extensions/llvm_toolchain_minimal_index.json)
and [source acquisition](https://github.com/hermeticbuild/hermetic-llvm/blob/v0.8.19/extensions/llvm.bzl)
are the starting points, not an independently maintained duplicate inventory.

### Alternatives Not Selected

- [Uber hermetic_cc_toolchain v4.3.0](https://github.com/uber/hermetic_cc_toolchain/tree/v4.3.0)
  supplies Zig `0.15.2` and useful Linux cross-compilation. Its external runtime cache, Apple SDK
  gaps, and missing integrated coverage/Clang tools increase total integration ownership. Keep it
  as a comparison for a concrete Linux benefit, not a second default backend.
- [toolchains_llvm v1.9.0](https://github.com/bazel-contrib/toolchains_llvm/tree/v1.9.0) is useful
  with already-qualified sysroots, but leaves more libc/SDK/runtime acquisition to the consumer.
  Host header/SDK discovery defaults must be replaced. Windows distribution entries do not supply
  a Windows C/C++ backend; the inspected implementation produces empty repositories there.
- Released hermetic-llvm uses MinGW on Windows. Its
  [unreleased MSVC implementation](https://github.com/hermeticbuild/hermetic-llvm/tree/65a1e017a2d629f6e0e6d3d8d2189f3561ba684b)
  requires libc++ and rejects MSVC coverage. Neither is the Microsoft-STL solution required here.
- Direct [rules_rust 0.74.0](https://github.com/bazelbuild/rules_rust/tree/0.74.0) has useful Cargo
  annotations and IDE APIs but does not solve Windows acquisition or native coverage. Retain it as
  the established alternative if bounded rules_rs remediation fails, subject to ADR 0013 review.
- Do not create a project-owned compiler backend around windows_support, substitute cargo-zigbuild
  for Bazel toolchains, or fall back to installed Visual Studio/SDKs.

## Profiles And Cross Builds

Start with upstream-native runtime choices rather than another distribution stack. These are
qualification configurations, not new public profile names or accepted minimum-OS support claims.

| Target profile | Initial configuration | Qualification boundary |
| --- | --- | --- |
| Linux x86_64/arm64 glibc | Upstream glibc `2.28` symbol floor; libc++; ordinary dynamic glibc linkage | Application glibc implementations come from deployment systems, not the link stubs. Inspect ELF dependencies, kernel requirements and execution-tool floors separately. Native x86_64 (seed) plus native arm64 (issue #410, CI `ubuntu-24.04-arm`) are qualified; the SQLite plus OpenSSL plus ring plus bindgen plus CXX corpus is qualified seed-only under issue #499 (`cc/tests/fixtures/linux_corpus/pins.bzl` via `bazel run //tools/ci:linux_corpus_qualification`, single-crate proof rejected, native only); glibc `2.28` floor pinning stays owned under open work under issue #500 and ELF-dependency plus glibc-symbol plus cross-build completeness stays owned as open work under issues #500/#504. |
| Linux x86_64/arm64 static musl | Upstream musl `1.2.6`; static native closure; non-PIE first for Rust compatibility | Static musl qualified (issue #411): Rust musl std via `extra_target_triples`, exec-platform tools for build scripts/proc macros with target musl libs for apps, prebuilt glibc libs never musl-compatible by linker change alone; CI cross-builds from Linux runners with per-profile cache scopes plus per-cell coverage for both musl cells. Hermetic-llvm static-only musl targets stay provisional. PIE qualification uses existing upstream constraints and stays owned under open work under issue #499. Verify test linkage as well as binaries; this is not dynamic-musl or musl `cdylib` support. Dynamic/shared musl stays excluded per [ADR 0014](decisions/0014-tested-platform-release-stack.md#decision) with no cell and no coverage: hermetic-llvm supplies static-only musl targets and uber shared-musl capability is not complete Rust/runtime-deployment evidence; do not implement. |
| macOS x86_64/arm64 | Pinned acquired Apple SDK, SDK libc++ headers/system dynamic libc++; upstream deployment default `14.0` as the starting point | SDK version is not deployment floor. Native arm64 qualified (issue #412, CI `macos-14` with `bazel-macos-arm64-` scope, pinned upstream toolchains; hermetic-llvm Apple-SDK backend provisional with immutable lazy fetch, no host-installed SDK fallback, no secrets, no interactive acceptance). Native x86_64 best-effort qualified (issue #413, CI `macos-15-intel` with `bazel-macos-x86_64-` scope, same provisional backend plus no fallback plus no secrets plus no interactive acceptance; `macos-13` retired December 2025, `macos-15-intel` until August 2027; gaps never block required-host release). Oldest-OS execution, framework completeness and licensing remain gates. |
| Windows x86_64 MSVC | clang-cl, Microsoft STL/UCRT/VCRuntime, retail dynamic CRT `/MD` as the starting point | Align Rust CRT mode, iterator-debug settings, system libraries and redistributable deployment. `/MT` and debug CRT are not assumed interchangeable. Native Windows x86_64 qualified (issue #414, CI `windows-latest` with `bazel-windows-x86_64-` scope, pinned upstream toolchains; toolchains_msvc clang-cl/Microsoft-STL backend provisional with immutable lazy fetch plus explicit EULA never automatic, no installed fallback; prebuilt-MSVC interop plus manifest/path/ABI fixtures qualify host-to-target plus target execution separately). |
| Linux GNU C++ prebuilt compatibility | Explicit dynamic libstdc++ alternative, not default libc++ substitution | Upstream supports it only on Linux glibc. Linux interop fixtures qualified seed-only under issue #498 (`cc/tests/fixtures/prebuilt_interop/pins.bzl` via `bazel run //tools/ci:prebuilt_interop_qualification`, explicit dynamic libstdc++ on Linux glibc only with GCC ABI fixtures, single-combo proof rejected); mixed Rust/C/C++ prebuilt-MSVC combos stay owned with issue #414, not double-claimed here. |

Sources: [LLVM profiles](https://github.com/hermeticbuild/hermetic-llvm/blob/v0.8.19/README.md),
[macOS deployment setting](https://github.com/hermeticbuild/hermetic-llvm/blob/v0.8.19/toolchain/args/macos/macos_minimum_os_flag.bzl),
[Rust platform mapping](https://github.com/hermeticbuild/rules_rs/blob/v0.0.109/rs/platforms/triples.bzl),
and [Windows compiler template](https://github.com/Dragnalith/toolchains_msvc/blob/8e2aa4624bbb5a53a94f135e90995f307875d1ad/overlays/toolchain/clang-cl/BUILD.toolchain.tpl).
Prefer ordinary object linking without cross-language LTO initially. Shared-runtime
deployment, exceptions, RTTI, allocation ownership and ABI boundaries qualified seed-only
under issue #498 (`cc/tests/fixtures/prebuilt_interop/pins.bzl` via `bazel run
//tools/ci:prebuilt_interop_qualification`, single-combo proof rejected); LLVM ancestry
alone is not interoperability evidence.

| Compiler execution platform | Targets to qualify | Order |
| --- | --- | --- |
| Linux x86_64 | Linux x86_64 and arm64, each glibc and static musl | First Linux cross-build cohort; x86_64 static musl qualified (issue #411, CI `ubuntu-latest` with `bazel-musl-x86_64-` scope) |
| Linux arm64 | Linux arm64 and x86_64, each glibc and static musl | Native arm64 glibc qualified (issue #410); arm64 static musl qualified (issue #411, CI `ubuntu-24.04-arm` with `bazel-musl-arm64-` scope); arm64-to-x86_64 cross stays in the first Linux cross-build cohort |
| macOS x86_64 | Native macOS x86_64 | Best-effort native workflow; native x86_64 best-effort qualified (issue #413, CI `macos-15-intel` with `bazel-macos-x86_64-` scope; gaps do not block required-host release per [ADR 0014](decisions/0014-tested-platform-release-stack.md#required-platforms)) |
| macOS arm64 | Native macOS arm64 | Required native workflow; native arm64 qualified (issue #412, CI `macos-14` with `bazel-macos-arm64-` scope) |
| Windows x86_64 | Native Windows x86_64 MSVC | Required native workflow; native Windows x86_64 qualified (issue #414, CI `windows-latest` with `bazel-windows-x86_64-` scope, pinned upstream toolchains; toolchains_msvc backend provisional) |
| macOS arm64 | Linux x86_64/arm64 profiles, then macOS x86_64 | Optional first expansion if bounded upstream configuration suffices |

Other routes are outside the initial qualification cohort, not claims of impossibility or permanent
product exclusions. In particular, do not require Linux/macOS-to-Windows, Linux/Windows-to-macOS,
Windows-to-Linux, or Windows arm64 to complete this cohort. Do not weaken required native workflows
to obtain a larger cross-build table.

Record the actual action execution platform, not only the Bazel client or CI runner OS. Run resulting
artifacts on matching native target workers; cross-building alone is insufficient. Emulation,
Rosetta, remote execution and deployment-floor testing are distinct evidence. The inspected
[LLVM CI](https://github.com/hermeticbuild/hermetic-llvm/blob/v0.8.19/.github/workflows/ci.yaml)
uses remote execution in Linux jobs and narrower macOS smoke/coverage tests. The
[rules_rs CI](https://github.com/hermeticbuild/rules_rs/blob/v0.0.109/.github/workflows/ci.yaml)
Windows-labelled lane builds remote GNULVM targets, not native MSVC tests/coverage.

## Rust And Native Integration

Use public upstream toolchains, `CcInfo`, Rust crate/dependency/build providers and existing
execution transitions. `rules_rs` static/shared-library wrappers map Cargo `staticlib` and `cdylib`;
they expose native linking providers rather than ordinary Rust-library providers. Exact public
`rules_dx` wrapper/provider mappings are pinned (issue #470,
`bazel run //tools/ci:foundation_maps`): `rust/rules/defs.bzl` forwards `CrateInfo`/`DepInfo`
plus `TestCrateInfo`/`CcInfo` for Cc-linking shapes with `QualitySourcesInfo`, proven by
`rust/rules/wrapper_tests.bzl` conformance; not a re-export of every upstream API.

The [build-script contract](generation/rust.md#build-scripts) remains authoritative: native compiler
exposure defaults on with a kept opt-out; shell environment and nonhermetic-path discovery default
off. Build-script executables, generators and proc macros run on the execution platform. Native
compiler executables also run there but compile application dependencies for the target platform.
Proc macros' own native dependencies must instead match their execution platform. Cargo
`platform_triples` must include the union of execution and target triples.

Prefer already-declared Bazel native libraries where integration exists, while retaining upstream
Cargo build scripts for authoritative manifests. CMake, Perl, NASM, pkg-config, headers, libraries,
and binding tools do not appear automatically because compiler exposure is enabled.

One source-level blocker still needs narrow upstream remediation
(the CC opt-out path above is qualified seed-only under issue #471):

- The pinned [build-script runner rule](https://github.com/hermeticbuild/rules_rust/blob/e9dd49f22cfa43c75ba30cd9d9bb7d8bdc459dde/cargo/private/cargo_build_script.bzl)
  still requests linker arguments after opting out of the C++ toolchain. The
  [generated Rust toolchain](https://github.com/hermeticbuild/rules_rs/blob/v0.0.109/rs/toolchains/declare_rustc_toolchains.bzl)
  supplies no standalone linker for ordinary desktop targets, producing a source-derived
  no-linker failure path. On the as-built stock `rules_rust` 0.74.0 stack this
  path is closed seed-only (issue #471): the kept opt-out executes
  successfully for pure-Rust scripts, with `LD` falling back to the sysroot
  `rust-lld` and `CC`/`CXX`/`AR` at the upstream `no_cc`/`no_cxx`/`no_ar`
  stubs, proven by `rust/tests/fixtures/cc_optout/` (`bazel run
  //tools/ci:cc_optout_qualification`). The contract stays enabled. Opt-out concerns
  the script execution action, not removal of the linker needed to build the script executable.
- Third-party shell environment is decided hermetic under issue #472
  (ambiguous default rejected): `crate_universe`-generated scripts render
  without an explicit `use_default_shell_env` and defer to the global
  `@rules_rust//cargo/settings:use_default_shell_env` flag, pinned `False`
  in `.bazelrc`; the per-crate
  `crate.annotation(..., build_script_use_default_shell_env = "on")`
  escape hatch in `rules_rust` 0.74.0 (`crate_universe/extensions.bzl`,
  `crate_universe/private/crate.bzl`, `crate_universe/src/config.rs`,
  `crate_universe/src/context/crate_context.rs`) is the only opt-in
  (currently zero opt-ins). Generated-script hermetic defaults are
  implemented (`use_cc_toolchain = True`, `use_default_shell_env = False`,
  `emit_warnings = True` in `gazelle/rust/lang.go`, proven by
  `gazelle/rust/lang_test.go`); `bazel run
  //tools/ci:shell_env_qualification` pins the global flag, the zero-opt-in
  annotation state, the deferred third-party rendering (e.g. `blake3`
  `_bs` carries no explicit attr), and the hostile-`PATH` plus declared-tool
  contract in [Rust Generation](generation/rust.md#build-scripts).
  First-party generator attributes alone do not enforce the third-party
  dependency closure.

For explicit binding generation, use
[rules_rs rust_bindgen](https://github.com/hermeticbuild/rules_rs/blob/v0.0.109/rs/rules_rust_bindgen.bzl)
at the pinned versions in `rust/tests/fixtures/bindgen/pins.bzl` (issue #473;
unpinned LLVM rejected). The parser baseline is LLVM-22 (`rules_rs` v0.0.109,
commit `b55b132af0c9951807c926768e40222330348632`, declaring LLVM rules
`0.8.18`/LLVM `22.1.8`); the qualified header/flag target is LLVM-23
(`hermetic-llvm` v0.8.19, commit `6314688712edf3a95f78642d80393868256b4ef2`,
LLVM `23.1.0`). Self-contained bindgen `0.72.1` prebuilts (`v0.0.2`, six
platform sha256s in `pins.bzl`) link libclang statically and take no
separate sysroot. The standalone route derives the target compiler context,
passes `--no-include-path-detection --formatter=none` plus explicit
`bindgen_flags`/`clang_flags`, and the Rust consumer separately links the
native library; the build-script route needs an explicit
execution-platform libclang closure and target parsing flags (the minimal
compiler archive alone is not proof that a loadable libclang exists).
Compat is proven by the `rust/tests/fixtures/bindgen/bindgen.h` plus
`bindgen.expected` fixture pair (C11-only stable constructs: macro
constant, enum, struct, anonymous-union tagged struct, function pointer,
opaque struct plus extern functions; C23-only spellings excluded) which
must yield the identical expected symbol set under both LLVM identities,
with `gcc -fsyntax-only -std=c11 -Wall -Werror` proving the header
well-formed on the seed host. Pinned by `bazel run
//tools/ci:bindgen_qualification`; see [Rust Generation](generation/rust.md#binding-generation).

Use [CXX's upstream generation pattern](https://github.com/dtolnay/cxx/blob/1.0.200/tools/bazel/rust_cxx_bridge.bzl)
with identical `cxx`/`cxxbridge-cmd` versions. Its Bazel module registers direct rules_rust toolchains;
that second graph is rejected under issue #474: no `cxx.rs` module lands in `MODULE.bazel`,
and the decided provider/repository identity is `cxx` plus `cxxbridge-cmd` at `1.0.200`
from the single crate_universe `crates` graph with generator tool `@crates//:cxxbridge-cmd`.
Do not introduce a second Rust graph merely to acquire the generator. CXX identity is decided
single-graph under issue #474 (proven by `rust/tests/fixtures/cxx_identity/` via `bazel run
//tools/ci:cxx_identity_qualification`); full `cxxbridge-cmd` execution plus corpus wiring
is qualified seed-only under issue #499 (`cc/tests/fixtures/linux_corpus/pins.bzl` via
`bazel run //tools/ci:linux_corpus_qualification`, single-crate proof rejected, native only).

## Windows Acquisition And Compatibility

The selected prototype uses standard rules_cc toolchains, so public Rust/C++ composition is plausible.
The following source facts prevent describing it as a drop-in hermetic stack:

- [Acquisition](https://github.com/Dragnalith/toolchains_msvc/blob/8e2aa4624bbb5a53a94f135e90995f307875d1ad/private/vs_channel_manifest.bzl)
  resolves live VS channel manifests and minor-version selectors. Individual payload checksums and a
  pinned rules commit do not freeze clean re-resolution. Prefer a narrow upstream fixed-manifest and
  package-index input over a separate project-owned downloader.
- SDK extraction uses pinned msiutil plus Windows `cmd`; LLVM acquisition rejects non-Windows hosts.
  No cross-host Windows route is established.
- The [actual extension](https://github.com/Dragnalith/toolchains_msvc/blob/8e2aa4624bbb5a53a94f135e90995f307875d1ad/extensions.bzl)
  reads `BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA=1` through repository
  environment. Its README advertises a different variable. Preserve deliberate upstream acceptance;
  this observed API is not yet a promised `rules_dx` setup command. Deferred acceptance failure does
  not prove laziness: extension evaluation already fetches manifests.
- [Toolchain registration](https://github.com/Dragnalith/toolchains_msvc/blob/8e2aa4624bbb5a53a94f135e90995f307875d1ad/private/msvc_toolchains_repo.bzl)
  constrains Windows/CPU but not the LLVM MSVC ABI constraint used by rules_rs. Qualify an explicit
  constraint fix so it cannot accidentally satisfy GNU/GNULVM targets.
- clang-cl flags use `/external:I`, and link inputs include `.lib` paths. The pinned Rust build-script
  rebasing code handles `/imsvc` but not `/external:I`, and omits `.lib`/`.obj` from bare-path handling.
  Batch linker wrappers, response-file contents, spaces, SDK system libraries, cc-rs discovery and
  assembly tools all need declared-input fixtures without host Visual Studio state.
- The backend has no complete coverage feature/runtime/collector wiring. A coverage-path
  normalization flag is not instrumentation. The
  [upstream Windows CI run](https://github.com/Dragnalith/toolchains_msvc/actions/runs/27756868995)
  exercises compiler variants but does not establish clean-host Rust, coverage or independent prebuilt
  Microsoft-STL interoperability.

[windows_support v0.4.1](https://github.com/hermeticbuild/windows_support/blob/v0.4.1/README.md)
demonstrates exact manifest-integrity and SDK-package acquisition without an installer. Its inspected
defaults identify MSVC `14.50.35717`, redist `14.50.35710`, and SDK package `10.0.26100.7705` separately.
They are not automatically the prototype's resolved inputs. Reuse upstream acquisition capability if
composition is narrow; do not write a new toolchain just to combine the two. Its SDK implementation
does not enforce the SDK EULA variable documented by hermetic-llvm; applicable terms and acknowledgement
must be reviewed rather than inferred from setting a variable.

Windows immutable-lazy acquisition is qualified seed-only under issue #495
(`cc/tests/fixtures/windows_acquisition/pins.bzl` via `bazel run
//tools/ci:windows_acquisition_qualification`): fixed-manifest plus
package-index inputs pinned with the toolchains_msvc head plus windows_support
`v0.4.1` package identities above; mutable fetch rejected (live channel
re-resolution, minor-version selectors, checksums-only, pinned-commit-only);
laziness proven (adding the module requires no acceptance and fetches no
restricted payloads, unrelated seed workflows stay green without acceptance,
deferred failure never proves laziness since extension evaluation already
fetches manifests); project-owned downloader plus cross-host inference
rejected. The backend stays provisional; floors stay owned under issue
#500, coverage qualified seed-only under issue #501; acquisition rights are qualified seed-only under
issue #496 below; windows transport is qualified seed-only under issue #497
below; prebuilt interop is qualified seed-only
under issue #498 below; the SQLite plus OpenSSL plus ring plus
bindgen plus CXX corpus is qualified seed-only under issue #499 below;
no `Supported` claim.

Apple plus Microsoft acquisition and cache rights are qualified seed-only under issue #496
(`cc/tests/fixtures/acquisition_rights/pins.bzl` via `bazel run
//tools/ci:acquisition_rights_qualification`): hermetic-llvm `v0.8.19`
pinned MacOSX26.5 SDK extraction reviewed against the
[Apple SDK agreement](https://www.apple.com/legal/sla/docs/xcode.pdf);
Apple-hosted execution does not by itself authorize separate extraction
or unrestricted caching; toolchains_msvc head plus windows_support
`v0.4.1` package identities (MSVC `14.50.35717`, redist `14.50.35710`,
SDK package `10.0.26100.7705`) with deliberate EULA repository-env never
automatic plus README mismatch, and windows_support not enforcing the
SDK EULA variable documented by hermetic-llvm (terms reviewed rather
than inferred); usage vs redistribution reviewed separately (acceptance
is not permission to redistribute; direct downloads, permission to use,
and permission to redistribute are separate checks); mirrors,
redistribution, internal caches, and remote workers need
license-approved boundaries separately from technical download success;
official download availability is not permission; assume rights rejected.
Backends stay provisional; floors stay owned under issue #500, coverage
qualified seed-only under issue #501; windows transport is qualified seed-only under issue #497 below;
prebuilt interop is qualified seed-only
under issue #498 below; the SQLite plus OpenSSL plus ring plus bindgen
plus CXX corpus is qualified seed-only under issue #499 below;
no `Supported` claim.

Windows transport plus ABI is qualified seed-only under issue #497
(`cc/tests/fixtures/windows_transport/pins.bzl` via `bazel run
//tools/ci:windows_transport_qualification`): explicit ABI constraint fix
pinned with the toolchains_msvc head plus rules_rs `v0.0.109`
`rs/platforms/triples.bzl` LLVM MSVC ABI identity (GNU/GNULVM rejected);
`/external:I` plus `.lib`/`.obj` bare-path handling pinned with the
patched rules_rust `e9dd49f` `cargo/private/cargo_build_script.bzl`
`/imsvc`-only baseline; batch wrappers plus response-file contents plus
spaces plus SDK system libraries plus cc-rs discovery plus cc-rs assembly
tools plus proc-macro DLLs ride declared-input fixtures (`transport.expected`
plus `response.rsp` plus `batch_wrapper.txt`) without host Visual Studio state;
compiler-target availability alone is not proof of host-to-target routes or
target execution. The backend stays provisional; rights plus interop
qualified seed-only under issues #496/#498, linux corpus qualified seed-only
under issue #499, floors stay owned under issue #500, coverage qualified seed-only under issue #501;
no `Supported` claim.

Prebuilt interop is qualified seed-only under issue #498
(`cc/tests/fixtures/prebuilt_interop/pins.bzl` via `bazel run
//tools/ci:prebuilt_interop_qualification`): explicit Windows STL/CRT/linker/library
combos (clang-cl plus Microsoft STL plus retail `/MD` with UCRT/VCRuntime, static `.lib`
plus import `.lib` plus DLL shapes, `cl.exe` compat as diagnosis only) with host-to-target
plus target execution qualifying separately plus Linux explicit dynamic libstdc++ comparison
on Linux glibc only plus ordinary object linking without cross-language LTO plus
shared-runtime plus exceptions plus RTTI plus allocation-ownership plus ABI boundaries;
compiler-target availability alone plus LLVM ancestry alone plus single-combo proof rejected.
Backends stay provisional; windows transport qualified seed-only under issue
#497 above; the SQLite plus OpenSSL plus ring plus bindgen plus CXX
corpus is qualified seed-only under issue #499 below; floors stay owned
under issue #500, coverage qualified seed-only under issue #501; no `Supported` claim.

Linux corpus is qualified seed-only under issue #499
(`cc/tests/fixtures/linux_corpus/pins.bzl` via `bazel run
//tools/ci:linux_corpus_qualification`): source-built SQLite plus OpenSSL
with declared build tools plus ring-style C/assembly with target libs plus
bindgen standalone plus build-script routes with execution libclang closure
plus CXX single-graph execution, native only with single-crate proof rejected.
Backends stay provisional; windows transport qualified seed-only under issue
#497 above; floors stay owned under issue #500, coverage qualified seed-only under issue #501;
no `Supported` claim.

Qualify independently compiled MSVC static/import libraries and DLLs, including STL values,
exceptions, RTTI and allocation ownership. Match compiler/linker/redist requirements under
[Microsoft's compatibility limits](https://learn.microsoft.com/en-us/cpp/porting/binary-compat-2015-2017?view=msvc-170)
and [Clang's MSVC compatibility](https://clang.llvm.org/docs/MSVCCompatibility.html).
Do not promise arbitrary compiler-specific `/GL`/`/LTCG` objects, CRT combinations or vendor libraries.
Missing mandatory evidence blocks Windows support, not just an optional C/C++ foundation.

## Coverage Generation And IDE Gaps

The inspected [LLVM coverage fixture](https://github.com/hermeticbuild/hermetic-llvm/blob/v0.8.19/e2e/rules_cc/.bazelrc)
uses Bazel 9 LLVM coverage flags and a raw-output workaround for an unreleased rules_cc fix.
Qualify the workaround or a focused patch, with nonempty reports and deliberately missed lines.
Rust coverage requires profiler-builtins in the target standard library. rules_rs selects llvm-cov
and llvm-profdata from `@llvm`, without override parameters in its toolchain declaration; Rust/Clang
raw-profile compatibility is therefore a real upgrade gate. The inspected
[Rust collector](https://github.com/hermeticbuild/rules_rust/blob/e9dd49f22cfa43c75ba30cd9d9bb7d8bdc459dde/util/collect_coverage/collect_coverage.rs)
exports from the test executable without additional shared-library objects. LCOV accounting
is qualified seed-only under issue #501 (`cc/tests/fixtures/lcov_accounting/pins.bzl` via
`bazel run //tools/ci:lcov_accounting_qualification`): Rust-only plus C/C++-only plus
mixed/DLL LCOV with missed-line tests, coverage-tool version pairing, native ignores and
denominator validation. No ignored collection failures (unaccounted lines rejected).
The accepted [coverage policy](testing/README.md#coverage) is not weakened to match missing collectors.

[gazelle_cc resolution](https://github.com/EngFlow/gazelle_cc/blob/v0.6.0/language/cc/resolve.go)
does not meet strict generation by configuration alone: unresolved-dependency errors cover quoted
includes, unknown angle includes may disappear, ambiguity can select a first candidate, and missing
module mappings can warn and omit edges. Its module index is not the consumer's resolved graph.
Reuse parser/generation capability only with bounded strictness and output adaptation, not log parsing
or a replacement C++ preprocessor. Preserve current union-of-literal-includes semantics; its
preprocessor-derived platform selections are not the approved Go-only generation exception.
Test grouping, generated headers, names, kept attributes and lifecycle remain conformance gates.
Named C++ modules/header units and PCH have no complete generation/IDE route established; assess them
explicitly before admission rather than treating compiler flags as complete support.

[Hedron's extractor](https://github.com/hedronvision/bazel-compile-commands-extractor/blob/abb61a688167623088f8768cc9264798df6a9d10/refresh.template.py)
uses actual action commands, but its refresh runs Bazel/preprocessors, writes workspace files,
changes extraction features and can continue after failures. It is not the immutable, exact-target,
symlink-only environment contract. Investigate a narrow action-derived snapshot adaptation, managed
host-native clangd, generated-output materialization and multiple header contexts. Do not infer
compile commands from `CcInfo` or use unrestricted clangd query-driver execution. Prefer a conforming
upstream automatic refresh route under the [automatic-workflow policy](product/scope.md#automatic-workflows)
if one is established; the raw extractor currently does not supply it. No additional C++
Bazel-invoking editor-launcher behavior is selected by this research.

Rust's upstream dynamic discovery accepts paths/buildfiles rather than exact target labels, although
generation/flycheck have target interfaces. Exact-target discovery is qualified seed-only under issue #475
(query-only without contract rejected): resolver-owned exact labels flow to the upstream `gen_rust_project`/`flycheck`
`TARGETS` interfaces preserving exact context, while `Path`/`Buildfile` widening to `//pkg:all` stays rejected as the
sole route. Prefer an upstream exact-target discovery extension over
a project-owned crate graph. Internal RustAnalyzerInfo is not a substitute for a public provider API.
See the [pinned discovery source](https://github.com/hermeticbuild/rules_rust/blob/e9dd49f22cfa43c75ba30cd9d9bb7d8bdc459dde/tools/rust_analyzer/rust_project.rs)
plus `rust/tests/fixtures/discovery/pins.bzl` and `bazel run //tools/ci:exact_target_qualification`.

## Qualification Questions And Delivery

These are engineering and legal-evidence tasks, not compiler-preference questions for the user.
Open items are tracked in the linked issues. No item is resolved by this research alone.

Required-core kept CC opt-out linker failure path is qualified seed-only
(issue #471): the kept opt-out executes successfully for pure-Rust scripts on
the as-built stock `rules_rust` 0.74.0 stack (sysroot `rust-lld` fallback plus
`no_cc` stubs, proven by `rust/tests/fixtures/cc_optout/` via `bazel run
//tools/ci:cc_optout_qualification`). Third-party shell-env is decided hermetic
under issue #472 (global `False` in `.bazelrc` with narrow per-crate opt-in,
zero opt-ins). Bindgen LLVM-22-vs-23 compatibility is qualified under issue #473:
LLVM-22 parser baseline vs LLVM-23 target pinned in
`rust/tests/fixtures/bindgen/pins.bzl` with the `bindgen.h` plus
`bindgen.expected` fixture pair (unpinned LLVM rejected). CXX graph identity
is decided single-graph under issue #474
(`cxx == cxxbridge-cmd == 1.0.200` from the single `crates` graph with
`@crates//:cxxbridge-cmd`, never a `cxx.rs` second graph, proven by
`rust/tests/fixtures/cxx_identity/` via `bazel run
//tools/ci:cxx_identity_qualification`). Exact-target discovery is qualified seed-only under issue #475
(resolver-owned exact labels to upstream `TARGETS`, `Path`/`Buildfile` widening rejected, project-owned graph plus
`RustAnalyzerInfo` rejected; pinned in `rust/tests/fixtures/discovery/pins.bzl` with the hello exact-isolation pair via
`bazel run //tools/ci:exact_target_qualification`). Exact current stable-stack compose is frozen
seed-only under issue #494 (as-built Bzlmod identities plus LLVM baseline-vs-target comparison
with checksums plus source patches plus compiler/profile compatibility, pinned in
`rust/tests/fixtures/stable_stack/pins.bzl` via `bazel run //tools/ci:stable_stack_qualification`;
ad-hoc compose rejected). The remaining native gap
stays owned under issue #472: global shell-env annotation
extension. Build-script hermetic defaults are
implemented (`use_cc_toolchain = True`, `use_default_shell_env = False`, `emit_warnings = True`
in `gazelle/rust/lang.go`, proven by `gazelle/rust/lang_test.go`) and pinned by
`bazel run //tools/ci:foundation_maps`; third-party shell-env is pinned by
`bazel run //tools/ci:shell_env_qualification`; bindgen compat is pinned by
`bazel run //tools/ci:bindgen_qualification`; CXX identity is pinned by
`bazel run //tools/ci:cxx_identity_qualification`; exact-target discovery is pinned by
`bazel run //tools/ci:exact_target_qualification`; stable-stack compose is pinned by
`bazel run //tools/ci:stable_stack_qualification`; no `Supported`
claim.
claim.

Admitted C/C++ foundation stays owned under issues #476-#484: MSVC interop plus SDK licensing
is qualified for the Windows x86_64 host under issue #414 per the [support
matrix](product/support-matrix.md#initial-feasibility-review) and [Windows
acquisition](#windows-acquisition-and-compatibility) above (provisional
toolchains_msvc backend, explicit EULA never automatic, interop plus
manifest/path/ABI fixtures, per-cell coverage); no `Supported` claim until
acquisition, interoperability, coverage, and release evidence passes.

| Question to close | Preferred next evidence or remedy | Tracking |
| --- | --- | --- |
<| Does the exact current stable stack compose? | Frozen seed-only under issue #494: as-built Bzlmod identities (Bazel 9.2.0 plus rules_rust 0.74.0 plus rules_cc 0.2.22 plus Rust 1.98.0) with `MODULE.bazel.lock` integrity, rules_rs LLVM `0.8.18`/LLVM `22.1.8` baseline vs hermetic-llvm `0.8.19`/LLVM `23.1.0` candidate comparison, checksums plus source patches plus compiler/profile compatibility, ad-hoc compose rejected; pinned in `rust/tests/fixtures/stable_stack/pins.bzl` via `bazel run //tools/ci:stable_stack_qualification` (qualified seed-only under issue #494). | issue #494 |
| Can Windows acquisition be immutable and lazy? | Qualified seed-only under issue #495: fixed-manifest plus package-index inputs pinned with the toolchains_msvc head plus windows_support `v0.4.1` package identities (`cc/tests/fixtures/windows_acquisition/pins.bzl` via `bazel run //tools/ci:windows_acquisition_qualification`); mutable fetch rejected; laziness proven (adding the module requires no acceptance and fetches no restricted payloads, unrelated workflows stay green without acceptance, deferred failure never proves laziness). Backend stays provisional; acquisition rights qualified seed-only under issue #496, windows transport qualified seed-only under issue #497, prebuilt interop qualified seed-only under issue #498, linux corpus qualified seed-only under issue #499, floors stay owned under issue #500, coverage qualified seed-only under issue #501. | issue #495 |
| Are Apple/Microsoft acquisition and cache rights adequate? | Qualified seed-only under issue #496: hermetic-llvm `v0.8.19` MacOSX26.5 extraction vs [Apple SDK agreement](https://www.apple.com/legal/sla/docs/xcode.pdf) with hosted execution not authorizing separate extraction or unrestricted caching, toolchains_msvc plus windows_support `v0.4.1` identities with deliberate EULA repository-env never automatic plus SDK EULA gap reviewed not inferred, usage vs redistribution reviewed separately (acceptance is not redistribution permission), mirrors plus redistribution plus internal caches plus remote workers as license-approved boundaries separately from download success, official download not permission, assume rights rejected; pinned in `cc/tests/fixtures/acquisition_rights/pins.bzl` via `bazel run //tools/ci:acquisition_rights_qualification`. Backend stays provisional; windows transport qualified seed-only under issue #497, prebuilt interop qualified seed-only under issue #498, linux corpus qualified seed-only under issue #499, floors stay owned under issue #500, coverage qualified seed-only under issue #501. | issue #496 |
| Can the kept CC opt-out execute successfully? | Qualified seed-only under issue #471: kept opt-out succeeds for pure-Rust scripts on stock `rules_rust` 0.74.0 (sysroot `rust-lld` fallback plus `no_cc` stubs, `rust/tests/fixtures/cc_optout/` via `bazel run //tools/ci:cc_optout_qualification`); script compilation inputs stay distinct from execution inputs. | issue #471 |
| Can third-party scripts retain a declared hermetic closure? | Decided hermetic under issue #472: global shell-env False in `.bazelrc` with narrow per-crate annotation opt-in (zero opt-ins); hostile PATH, tool discovery and additional declared tools pinned by `bazel run //tools/ci:shell_env_qualification` plus [Rust Generation](generation/rust.md#build-scripts). | issue #472 |
<| Does Windows native transport preserve all inputs and ABI selection? | Qualified seed-only under issue #497: explicit ABI constraint fix (toolchains_msvc registration plus rules_rs LLVM MSVC ABI identity, GNU/GNULVM rejected) plus `/external:I` vs `/imsvc` plus `.lib`/`.obj` bare-path handling (patched rules_rust rebasing baseline) with batch wrappers plus response files plus spaces plus SDK libraries plus cc-rs discovery/assembly plus proc-macro DLLs on declared-input fixtures without host Visual Studio state (`cc/tests/fixtures/windows_transport/pins.bzl` via `bazel run //tools/ci:windows_transport_qualification`); compiler-target availability alone is not proof. Backend stays provisional; rights plus interop qualified seed-only under issues #496/#498, linux corpus qualified seed-only under issue #499, floors stay owned under issue #500, coverage qualified seed-only under issue #501. | issue #497 |
| Which prebuilt native libraries interoperate? | Qualified seed-only under issue #498: explicit Windows STL/CRT/linker/library combos (clang-cl plus Microsoft STL plus retail `/MD` with static `.lib` plus import `.lib` plus DLL shapes, `cl.exe` compat as diagnosis only) plus Linux explicit dynamic libstdc++ comparison on Linux glibc only plus ordinary object linking without cross-language LTO plus shared-runtime plus exceptions plus RTTI plus allocation-ownership plus ABI boundaries, pinned in `cc/tests/fixtures/prebuilt_interop/pins.bzl` via `bazel run //tools/ci:prebuilt_interop_qualification`; host-to-target plus target execution qualify separately with mixed Rust/C/C++ composition, compiler-target availability alone plus LLVM ancestry alone plus single-combo proof rejected. Windows x86_64 qualified (issue #414) with representative prebuilt-MSVC fixtures incl mixed Rust/C/C++ stays the host record without double-claiming here. Backends stay provisional; windows transport qualified seed-only under issue #497, linux corpus qualified seed-only under issue #499, floors stay owned under issue #500, coverage qualified seed-only under issue #501. | issue #498 |
| Are both Linux profiles complete? | Qualified seed-only under issue #499: native arm64 glibc builds/tests plus per-cell coverage landed (issue #410); static-musl closures plus per-cell coverage for both musl profiles landed (issue #411, Rust musl std plus exec/target separation, CI cross-builds with per-profile cache scopes, no union; dynamic musl explicitly out of scope); the SQLite plus OpenSSL plus ring plus bindgen plus CXX corpus is qualified seed-only (`cc/tests/fixtures/linux_corpus/pins.bzl` via `bazel run //tools/ci:linux_corpus_qualification`, single-crate proof rejected, native only). Backends stay provisional; windows transport qualified seed-only under issue #497; PIE plus ELF-dependency plus glibc-symbol plus cross-build completeness plus floors stay owned as open work under issues #500/#504, coverage qualified seed-only under issue #501. | issue #499 |
| Which deployment and execution floors are supportable? | Pin the glibc `2.28` floor plus SDK/CRT identities with oldest-target and current-host fixtures run separately; inspect compiler, clangd and bindgen loader dependencies. Check Apple's extracted SDK framework subset. macOS arm64 native is qualified (issue #412) plus macOS x86_64 best-effort native is qualified (issue #413) with SDK version not the deployment floor; oldest-OS execution plus framework completeness plus licensing remain gates; best-effort gaps never block required-host release. Windows x86_64 native is qualified (issue #414) with `/MD` retail dynamic CRT as the starting point; `/MT` plus debug CRT plus floors stay owned by issues #410-#414. | open work under issue #500 |
| Can every executable first-party line be accounted for? | Qualified seed-only under issue #501: Rust-only plus C/C++-only plus mixed/DLL LCOV with missed-line tests, coverage-tool version pairing, native ignores and denominator validation, pinned in `cc/tests/fixtures/lcov_accounting/pins.bzl` via `bazel run //tools/ci:lcov_accounting_qualification`; unaccounted lines plus ignored collection failures rejected. Backends stay provisional; floors stay owned under issue #500. | issue #501 |
| Can bindgen/CXX use one upstream graph? | Qualified under issue #473 for bindgen (LLVM-22 parser baseline vs LLVM-23 target pinned with the standalone/build-script fixture pair, execution libclang closure, target flags, `bindgen.h` plus `bindgen.expected` identical-set proof via `bazel run //tools/ci:bindgen_qualification`); CXX identity decided single-graph under issue #474 (`cxx == cxxbridge-cmd == 1.0.200` from the single `crates` graph, `rust/tests/fixtures/cxx_identity/` via `bazel run //tools/ci:cxx_identity_qualification`); CXX corpus execution plus the SQLite plus OpenSSL plus ring plus bindgen plus CXX corpus qualified seed-only under issue #499 (`cc/tests/fixtures/linux_corpus/pins.bzl` via `bazel run //tools/ci:linux_corpus_qualification`, single-crate proof rejected, native only). | issues #473, #474, #499 |
| Can public Cargo metadata represent every generated target? | Prove features, build-script metadata, target kinds and ownership without private serialized dependency-graph access; seek narrow upstream metadata exports where missing. | open work under issue #502 |
| Can generation satisfy strict ownership and resolution cheaply? | Quoted/angle/ambiguous/macro include fixtures, authoritative dependency metadata, test grouping, generated headers, assembly dialects and explicit module/PCH disposition. | open work under issue #503 |
| Can IDE setup preserve exact context and projection contracts? | Qualified seed-only under issue #475 for Rust exact-target discovery (resolver-owned exact labels to `gen_rust_project`/`flycheck` TARGETS, `Path`/`Buildfile` widening plus project-owned graph plus `RustAnalyzerInfo` rejected, hello exact-isolation pair plus `rust/tests/fixtures/discovery/pins.bzl` via `bazel run //tools/ci:exact_target_qualification`); C++ action-derived snapshot plus generated sources plus multi-context headers plus managed host tools plus Bazel-9 compatibility stay open proof. | issue #475 |
| Which cross routes actually work and execute? | Capture compiler execution platform, native target execution and separate cache/remote evidence for every claimed row. Expand only after the initial cohort passes. | open work under issue #504 |
| Is the remediation bounded enough for admission? | Reproduce defects, estimate each upstream fix, name actual owners, record patch/upstream issue/upgrade tracking and complete-workflow evidence. | open work under issue #505 |

The Apple starting point is hermetic-llvm's pinned MacOSX26.5 SDK extraction. Review the terms
accompanying that exact package against the
[Apple SDK agreement](https://www.apple.com/legal/sla/docs/xcode.pdf); Apple-hosted execution does not
by itself authorize separate extraction or unrestricted caching. Rights are qualified seed-only
under issue #496 (`cc/tests/fixtures/acquisition_rights/pins.bzl` via `bazel run
//tools/ci:acquisition_rights_qualification`, usage vs redistribution reviewed separately,
assume rights rejected).

Admission records a light inventory entry — minimal required core/framework inventory plus
dispositions only, with effort evidence recorded as each tracked item lands. Remaining
per-candidate mappings stay pending
[additional-foundation qualification](product/support-matrix.md#additional-v1-foundations) under
open work under issues #416-#420. Ownership: the sole repository
maintainer owns every row until maintenance is explicitly delegated.

The fixture corpus starts with pure-Rust scripts/default and opt-out
(`rust/tests/fixtures/cc_optout/` pinned under issue #471), cc-rs C/C++, SQLite, OpenSSL
with declared tools/libraries, ring-style C/assembly, bindgen, CXX, native proc-macro dependencies,
Rust staticlib/cdylib consumers, and independently built Microsoft-STL libraries. The SQLite plus
OpenSSL plus ring plus bindgen plus CXX corpus is qualified seed-only under issue #499
(`cc/tests/fixtures/linux_corpus/pins.bzl` via `bazel run //tools/ci:linux_corpus_qualification`,
single-crate proof rejected, native only). Use application-locked
crate versions and feature sets; crate names alone do not define tested workflows.

If fixes require a replacement acquisition engine, compiler backend, Cargo graph or coverage engine,
stop that slice and revisit alternatives under the maintenance policy. Do not defer required Rust
Windows support, admit complete C/C++ prematurely, or weaken hermeticity to make the table green.
