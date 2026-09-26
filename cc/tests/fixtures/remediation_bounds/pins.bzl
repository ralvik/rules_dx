"""Bounded remediation pins.

Contract: `docs/native-toolchains.md#qualification-questions-and-delivery`,
`docs/decisions/0013-rust-javascript-typescript-foundations.md#core-language-completion-and-remediation`,
`docs/decisions/0014-tested-platform-release-stack.md#decision`, `docs/decisions/0008-dependency-currency.md#decision`.
Fixture: `cc/tests/fixtures/remediation_bounds/` via
`bazel run //tools/ci:remediation_bounds_qualification`.
"""

# Bounded-remediation definition: reproduce each defect, estimate the
# upstream fix, name the actual owner, and record patch plus
# upstream-issue plus upgrade tracking with complete-workflow evidence.
BOUNDED_DEFINITION = "Reproduce defects, estimate each upstream fix, name actual owners, record patch plus upstream issue plus upgrade tracking and complete-workflow evidence"
BOUNDED_EVIDENCE = "complete-workflow evidence"
BOUNDED_SCOPE = "Scope only"

# Allowed remediation forms per ADR 0013 plus ADR 0014 plus ADR 0008:
ALLOWED_FORMS = [
    "focused, tested, pinned patches",
    "bounded integration",
    "small missing integration pieces with clearly bounded scope and maintenance responsibility",
    "preserving upstream language semantics and verified public provider boundaries",
    "prefer upstreaming those fixes",
    "compare established alternative upstream rules before considering replacement",
    "explicit contract plus API review before adoption",
]
ALLOWED_NOTE = "Focused upstream rules patches are allowed when pinned reproducibly and tested against the accepted contracts"

# Defect inventory: source-derived failure paths with reproduction
# plus estimate plus owner plus tracking plus complete-workflow
# evidence. Each slice stops instead of growing into a replacement
# engine when the bound below is exceeded.
DEFECT_CC_OPTOUT = "kept CC opt-out linker failure path with sysroot rust-lld fallback plus no_cc stubs"
DEFECT_CC_OPTOUT_SOURCE = "cargo/private/cargo_build_script.bzl no-linker failure path"
DEFECT_CC_OPTOUT_OWNER = "hermeticbuild/rules_rust"
DEFECT_CC_OPTOUT_EVIDENCE = "rust/tests/fixtures/cc_optout/ via cc_optout_qualification under issue #471"
DEFECT_TRANSPORT = "Windows transport rebasing with /external:I plus .lib plus .obj bare-path handling"
DEFECT_TRANSPORT_SOURCE = "cargo/private/cargo_build_script.bzl /imsvc-only baseline plus batch wrappers plus response files plus spaces plus SDK libraries plus cc-rs discovery plus cc-rs assembly plus proc-macro DLLs"
DEFECT_TRANSPORT_OWNER = "toolchains_msvc plus patched rules_rust"
DEFECT_TRANSPORT_EVIDENCE = "cc/tests/fixtures/windows_transport/pins.bzl via windows_transport_qualification under issue #497"
DEFECT_ABI = "Windows ABI constraint fix so toolchains_msvc cannot satisfy GNU plus GNULVM targets"
DEFECT_ABI_SOURCE = "private/msvc_toolchains_repo.bzl plus rs/platforms/triples.bzl LLVM MSVC ABI identity"
DEFECT_ABI_EVIDENCE = "windows_transport_qualification under issue #497 with GNU plus GNULVM rejected"
DEFECT_ACQUISITION = "Windows acquisition fixed-manifest plus package-index inputs"
DEFECT_ACQUISITION_SOURCE = "private/vs_channel_manifest.bzl live VS channel manifests plus minor-version selectors"
DEFECT_ACQUISITION_EVIDENCE = "cc/tests/fixtures/windows_acquisition/pins.bzl via windows_acquisition_qualification under issue #495"
DEFECT_COVERAGE = "Coverage workaround with Bazel 9 LLVM flags plus raw-output workaround plus llvm-cov plus llvm-profdata selection"
DEFECT_COVERAGE_SOURCE = "hermetic-llvm e2e rules_cc .bazelrc plus unreleased rules_cc fix plus rules_rs toolchain without override parameters plus profiler-builtins"
DEFECT_COVERAGE_EVIDENCE = "cc/tests/fixtures/lcov_accounting/pins.bzl via lcov_accounting_qualification under issue #501"
DEFECT_GENERATION = "Generation strictness with gazelle_cc resolve plus module index plus test grouping plus generated headers plus assembly plus modules plus PCH"
DEFECT_GENERATION_SOURCE = "language/cc/resolve.go quoted plus angle plus ambiguity plus warn-and-omit"
DEFECT_GENERATION_EVIDENCE = "cc/tests/fixtures/strict_generation/pins.bzl via strict_generation_qualification under issue #503"
DEFECT_IDE = "IDE snapshot adaptation with action-derived commands plus managed clangd plus generated-output materialization plus multi-context headers"
DEFECT_IDE_SOURCE = "refresh.template.py Bazel plus preprocessors plus workspace writes plus continued-after-failures"
DEFECT_IDE_EVIDENCE = "exact-target discovery qualified seed-only under issue #475 plus C++ snapshot qualified seed-only under issue #754 via cpp_snapshot_qualification"
DEFECT_FLOORS = "PIE plus ELF-dependency plus glibc-symbol uses existing upstream constraints"
DEFECT_FLOORS_NOTE = "PIE qualification uses existing upstream constraints"
DEFECT_FLOORS_EVIDENCE = "glibc 2.28 symbol floor plus deployment 14.0 qualified seed-only under issue #500"
DEFECT_CROSS = "Cross-product expansion only after the initial cohort passes with bounded upstream configuration"
DEFECT_CROSS_EVIDENCE = "cc/tests/fixtures/cross_routes/pins.bzl via cross_routes_qualification under issue #504"

# Owner plus estimate plus tracking record: actual upstream owner is
OWNER_NOTE = "name actual owners"
ESTIMATE_NOTE = "estimate each upstream fix"
TRACKING_NOTE = "record patch plus upstream issue plus upgrade tracking"
FAIL_CLOSED_NOTE = "the affected capability fails closed until a compliant remedy passes the required evidence"
UPSTREAMING_NOTE = "Prefer upstreaming those fixes"

# Stop-slice bound: if fixes require a replacement acquisition engine,
STOP_SLICE_NOTE = "If fixes require a replacement acquisition engine, compiler backend, Cargo graph or coverage engine, stop that slice and revisit alternatives under the maintenance policy"
STOP_SLICE_CASES = [
    "replacement acquisition engine",
    "compiler backend",
    "Cargo graph",
    "coverage engine",
]
NO_SHORTCUT_NOTE = "Do not defer required Rust Windows support, admit complete C/C++ prematurely, or weaken hermeticity to make the table green"

# Cross-product bound: do not implement missing infra merely to fill
# the cross-product. Linux cross is the first priority, not a mandate
# to build every target from every host; broader cross-builds are
# desirable only when upstream configuration keeps maintenance bounded.
CROSS_PRODUCT_NOTE = "Do not implement missing infra merely to fill cross-product"
CROSS_PRIORITY_NOTE = "Linux cross first priority not mandate every target from every host"
CROSS_BOUND_NOTE = "Broader cross-builds are desirable when upstream configuration keeps maintenance bounded"

# Rejected substitutes per the issue alternatives: unbounded fork plus
# engine replacements plus owned-backend plus substitutions plus
# fallback plus cross-product infra fill plus weakened natives.
REJECTED_ALTERNATIVES = [
    "unbounded fork",
    "whole ruleset rewrite",
    "replacement language engine",
    "replacement acquisition engine",
    "compiler backend",
    "Cargo graph",
    "coverage engine",
    "project-owned compiler backend around windows_support",
    "cargo-zigbuild substitution",
    "installed Visual Studio fallback",
    "installed SDK fallback",
    "missing infra to fill cross-product",
    "all-cross mandate",
    "weakened native workflows for a larger table",
]

# Backend stays provisional; floors plus coverage plus corpus plus
# routes stay owned.
BACKEND_NOTE = "Backends stay provisional"
OWNED_GAPS = ["#499", "#500", "#501", "#504"]

# Live proof labels: the seed hello builds on the seed host; the corpus
# plus floors targets prove the fixture is wired without claiming a
# backend, coverage, or Supported.
REMEDIATION_BOUNDS_FIXTURE_CORPUS = "//cc/tests/fixtures/remediation_bounds:corpus_starlark"
REMEDIATION_BOUNDS_LIVE_HELLO = "//cc/tests/fixtures/hello:hello"
REMEDIATION_BOUNDS_LINUX_CORPUS = "//cc/tests/fixtures/linux_corpus:corpus"
REMEDIATION_BOUNDS_FLOORS_CORPUS = "//cc/tests/fixtures/deployment_floors:corpus_starlark"
