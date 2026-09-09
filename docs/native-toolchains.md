# Native Toolchain Qualification

## Status And Boundary

Engineering qualification plan. Native-stack
and cross-compilation technical choices are delegated to evidence-based upstream review. The choices below select
what to qualify first, not an implemented public API, frozen release manifest, or verified support.
[ADR 0014](decisions/0014-tested-platform-release-stack.md#decision) retains authority over hermetic
acquisition, Windows interoperability, required Linux profiles, and bounded cross-build scope.
[Support Matrix](product/support-matrix.md) owns support and foundation-admission claims.

Research inspected pinned source, documentation, releases, and upstream CI. No `rules_dx` native
build, test, coverage, remote-execution, or acquisition qualification ran. No implementation milestone
is approved. Source-derived failure paths below need focused reproduction before patches are chosen.

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
it, not a release pin: the release default follows ADR 0008 and the exact seed pin is
owned by O14. Do not silently inherit rules_rs's
older compiler default or turn a research version into a release pin.

### Inspected Identities

| Component | Identity and primary source | Important distinction |
| --- | --- | --- |
| Rust rules | [rules_rs v0.0.109](https://github.com/hermeticbuild/rules_rs/tree/v0.0.109), commit `b55b132af0c9951807c926768e40222330348632` | Latest published release observed. Its MODULE declares LLVM rules `0.8.18`, not `0.8.19`. |
| Patched Rust rules | [Pinned archive declaration](https://github.com/hermeticbuild/rules_rs/blob/v0.0.109/rs/rules_rust.bzl), commit `e9dd49f22cfa43c75ba30cd9d9bb7d8bdc459dde` | Module version `0.74.0` does not make it stock bazelbuild/rules_rust `0.74.0`. |
| Native rules | [hermetic-llvm v0.8.19](https://github.com/hermeticbuild/hermetic-llvm/tree/v0.8.19), commit `6314688712edf3a95f78642d80393868256b4ef2` | Latest release observed; LLVM `23.1.0`. Reference `v0.8.18` uses LLVM `22.1.8`; upgrading is a real stack change. |
| C/C++ rules | [rules_cc 0.2.22](https://github.com/bazelbuild/rules_cc/tree/0.2.22) | Record the resolved module graph: upstream direct declarations differ. |
| Windows backend | [toolchains_msvc pinned source](https://github.com/Dragnalith/toolchains_msvc/tree/8e2aa4624bbb5a53a94f135e90995f307875d1ad) | Latest head observed; prototype, no published release verified. Declared module `0.1.0` is not a release. |
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
| Linux x86_64/arm64 glibc | Upstream glibc `2.28` symbol floor; libc++; ordinary dynamic glibc linkage | Application glibc implementations come from deployment systems, not the link stubs. Inspect ELF dependencies, kernel requirements and execution-tool floors separately. |
| Linux x86_64/arm64 static musl | Upstream musl `1.2.6`; static native closure; non-PIE first for Rust compatibility | Use existing upstream PIE constraints. Verify test linkage as well as binaries; this is not dynamic-musl or musl `cdylib` support. |
| macOS x86_64/arm64 | Pinned acquired Apple SDK, SDK libc++ headers/system dynamic libc++; upstream deployment default `14.0` as the starting point | SDK version is not deployment floor. Oldest-OS execution, framework completeness and licensing remain gates. |
| Windows x86_64 MSVC | clang-cl, Microsoft STL/UCRT/VCRuntime, retail dynamic CRT `/MD` as the starting point | Align Rust CRT mode, iterator-debug settings, system libraries and redistributable deployment. `/MT` and debug CRT are not assumed interchangeable. |
| Linux GNU C++ prebuilt compatibility | Explicit dynamic libstdc++ alternative, not default libc++ substitution | Upstream supports it only on Linux glibc. Patched Rust runtime selection and actual GCC ABI/library fixtures must pass before claiming it. |

Sources: [LLVM profiles](https://github.com/hermeticbuild/hermetic-llvm/blob/v0.8.19/README.md),
[macOS deployment setting](https://github.com/hermeticbuild/hermetic-llvm/blob/v0.8.19/toolchain/args/macos/macos_minimum_os_flag.bzl),
[Rust platform mapping](https://github.com/hermeticbuild/rules_rs/blob/v0.0.109/rs/platforms/triples.bzl),
and [Windows compiler template](https://github.com/Dragnalith/toolchains_msvc/blob/8e2aa4624bbb5a53a94f135e90995f307875d1ad/overlays/toolchain/clang-cl/BUILD.toolchain.tpl).
Prefer ordinary object linking without cross-language LTO initially. Qualify shared-runtime
deployment, exceptions, RTTI, allocation ownership and ABI boundaries explicitly; LLVM ancestry
alone is not interoperability evidence.

| Compiler execution platform | Targets to qualify | Order |
| --- | --- | --- |
| Linux x86_64 | Linux x86_64 and arm64, each glibc and static musl | First Linux cross-build cohort |
| Linux arm64 | Linux arm64 and x86_64, each glibc and static musl | First Linux cross-build cohort |
| macOS x86_64 | Native macOS x86_64 | Best-effort when a host is available; gaps do not block required-host release per [ADR 0014](decisions/0014-tested-platform-release-stack.md#required-platforms) |
| macOS arm64 | Native macOS arm64 | Required native workflow |
| Windows x86_64 | Native Windows x86_64 MSVC | Required native workflow; backend independently blocked |
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
`rules_dx` wrapper/provider mappings remain O16/O24 work, not a re-export of every upstream API.

The [build-script contract](generation/rust.md#build-scripts) remains authoritative: native compiler
exposure defaults on with a kept opt-out; shell environment and nonhermetic-path discovery default
off. Build-script executables, generators and proc macros run on the execution platform. Native
compiler executables also run there but compile application dependencies for the target platform.
Proc macros' own native dependencies must instead match their execution platform. Cargo
`platform_triples` must include the union of execution and target triples.

Prefer already-declared Bazel native libraries where integration exists, while retaining upstream
Cargo build scripts for authoritative manifests. CMake, Perl, NASM, pkg-config, headers, libraries,
and binding tools do not appear automatically because compiler exposure is enabled.

Two source-level blockers need narrow upstream remediation:

- The pinned [build-script runner rule](https://github.com/hermeticbuild/rules_rust/blob/e9dd49f22cfa43c75ba30cd9d9bb7d8bdc459dde/cargo/private/cargo_build_script.bzl)
  still requests linker arguments after opting out of the C++ toolchain. The
  [generated Rust toolchain](https://github.com/hermeticbuild/rules_rs/blob/v0.0.109/rs/toolchains/declare_rustc_toolchains.bzl)
  supplies no standalone linker for ordinary desktop targets, producing a source-derived
  no-linker failure path. Reproduce and fix opt-out without disabling the contract. Opt-out concerns
  the script execution action, not removal of the linker needed to build the script executable.
- Third-party [crate annotations](https://github.com/hermeticbuild/rules_rs/blob/v0.0.109/rs/extensions.bzl)
  expose additional tools/environment but not per-crate CC/shell-env switches. Generated scripts
  inherit an upstream shell-env default of True. Qualify the public global False setting or a narrow
  annotation extension; first-party generator attributes alone do not enforce the dependency closure.

For explicit binding generation, start with
[rules_rs rust_bindgen](https://github.com/hermeticbuild/rules_rs/blob/v0.0.109/rs/rules_rust_bindgen.bzl).
It acquires self-contained bindgen executables, derives target compiler context and disables include
discovery. Its LLVM-22-based parser still needs compatibility tests against LLVM-23 headers/flags,
and the Rust consumer must separately link the native library. Build-script bindgen needs an explicit
execution-platform libclang closure and target parsing flags; the minimal compiler archive is not
proof that a loadable libclang exists.

Use [CXX's upstream generation pattern](https://github.com/dtolnay/cxx/blob/1.0.200/tools/bazel/rust_cxx_bridge.bzl)
with identical `cxx`/`cxxbridge-cmd` versions. Its Bazel module registers direct rules_rust toolchains;
qualify provider/repository identity before composing it with rules_rs. Do not introduce a second
Rust graph merely to acquire the generator.

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
exports from the test executable without additional shared-library objects. Mixed-language/DLL
coverage, included test code, static musl and source-denominator accounting need explicit fixtures.
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
generation/flycheck have target interfaces. Prefer an upstream exact-target discovery extension over
a project-owned crate graph. Internal RustAnalyzerInfo is not a substitute for a public provider API.
See the [pinned discovery source](https://github.com/hermeticbuild/rules_rust/blob/e9dd49f22cfa43c75ba30cd9d9bb7d8bdc459dde/tools/rust_analyzer/rust_project.rs).

## Qualification Questions And Delivery

These are engineering and legal-evidence tasks, not compiler-preference questions for the user.
Existing decision IDs retain their scope and deadlines. No item is resolved by this research alone.

| Question to close | Preferred next evidence or remedy | Gate and delivery owner |
| --- | --- | --- |
| Does the exact current stable stack compose? | Freeze resolved Bzlmod identities; compare rules_rs's LLVM reference with the newer candidate; record checksums, source patches and compiler/profile compatibility. | O14/O37; bootstrap and release maintainers |
| Can Windows acquisition be immutable and lazy? | Reproduce clean re-resolution; qualify upstream fixed-manifest/package inputs and observed downloads, including missing acceptance and unrelated workflows. | O46; native integration maintainer |
| Are Apple/Microsoft acquisition and cache rights adequate? | Review actual package terms, deliberate acceptance, extraction, mirrors, redistribution, internal caches and remote workers. Official download availability is not permission. | O46/O39; release maintainer with qualified legal review |
| Can the kept CC opt-out execute successfully? | Reproduce no-linker analysis path; narrow upstream runner fix; distinguish script compilation inputs from execution inputs. | O24; M12 Rust maintainer |
| Can third-party scripts retain a declared hermetic closure? | Qualify global shell-env False or upstream annotation extension; test hostile PATH, tool discovery and additional declared tools. | O24; M12 Rust maintainer |
| Does Windows native transport preserve all inputs and ABI selection? | Fix ABI constraints and path rebasing upstream; test batch wrappers, response files, cc-rs assembly/discovery, SDK libraries and proc-macro DLLs. | O24/O46; Rust and native maintainers |
| Which prebuilt native libraries interoperate? | Independent MSVC fixtures and Linux libstdc++ comparison; verify STL/CRT modes, unwinding, ownership and runtime deployment. | O24/O37/O46; native maintainer |
| Are both Linux profiles complete? | Native and cross builds/tests, ELF dependencies, glibc symbols, static-musl test linkage and negative shared-musl cases. | O24/O37; Rust and native maintainers |
| Which deployment and execution floors are supportable? | Run oldest-target and current-host fixtures separately; inspect compiler, clangd and bindgen loader dependencies. Check Apple's extracted SDK framework subset. | O37; release maintainer |
| Can every executable first-party line be accounted for? | Rust-only, C/C++-only and mixed/DLL LCOV, missed-line tests, coverage-tool version pairing, native ignores and denominator validation. No ignored collection failures. | O24/O46; language and coverage maintainers |
| Can bindgen/CXX use one upstream graph? | Separate standalone/build-script bindgen fixtures; execution libclang closure, target flags and identical CXX crate/generator versions. | O24/O33; Rust and codegen maintainers |
| Can public Cargo metadata represent every generated target? | Prove features, build-script metadata, target kinds and ownership without private serialized dependency-graph access; seek narrow upstream metadata exports where missing. | O22/O24; Rust generation maintainer |
| Can generation satisfy strict ownership and resolution cheaply? | Quoted/angle/ambiguous/macro include fixtures, authoritative dependency metadata, test grouping, generated headers, assembly dialects and explicit module/PCH disposition. | O46; M22 generation maintainer |
| Can IDE setup preserve exact context and projection contracts? | Upstream exact-target Rust discovery and action-derived C++ snapshot proof, generated sources, multi-context headers, managed host tools and Bazel-9 compatibility. | O24/O46; Rust and M22 environment maintainers |
| Which cross routes actually work and execute? | Capture compiler execution platform, native target execution and separate cache/remote evidence for every claimed row. Expand only after the initial cohort passes. | O37/O46; release maintainer |
| Is the remediation bounded enough for admission? | Reproduce defects, estimate each upstream fix, name actual owners, record patch/upstream issue/upgrade tracking and complete-workflow evidence. | O46; release and foundation maintainers |

The Apple starting point is hermetic-llvm's pinned MacOSX26.5 SDK extraction. Review the terms
accompanying that exact package against the
[Apple SDK agreement](https://www.apple.com/legal/sla/docs/xcode.pdf); Apple-hosted execution does not
by itself authorize separate extraction or unrestricted caching.

O46 uses a light freeze at M00
entry — minimal required core/framework inventory plus dispositions only, with detailed
effort evidence for interfaces affecting M00–M03. Remaining per-candidate mappings,
effort, and ownership stay pending O30/O31 qualification in M22/M23. The table's roles do not
pretend maintainers, patch estimates or upstream issue numbers have already been assigned. O16 gates
early Rust providers, O24 gates M12 integration, O46 gates additional-foundation disposition/M22
mappings, and O37 owns final route/pin qualification. Preserve the existing milestone DAG.

The fixture corpus starts with pure-Rust scripts/default and opt-out, cc-rs C/C++, SQLite, OpenSSL
with declared tools/libraries, ring-style C/assembly, bindgen, CXX, native proc-macro dependencies,
Rust staticlib/cdylib consumers, and independently built Microsoft-STL libraries. Use application-locked
crate versions and feature sets; crate names alone do not define tested workflows.

If fixes require a replacement acquisition engine, compiler backend, Cargo graph or coverage engine,
stop that slice and revisit alternatives under the maintenance policy. Do not defer required Rust
Windows support, admit complete C/C++ prematurely, or weaken hermeticity to make the table green.
