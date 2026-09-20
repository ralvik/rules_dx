#!/usr/bin/env bash
# Linux corpus qualification harness (issue #499).
#
# Defines plus proves the corpus slice of the Linux profiles with fixture
# evidence, without claiming a qualified hermetic-llvm backend, qualified
# floors, qualified cross routes, or Supported:
# - profiles: Linux x86_64/arm64 glibc plus static musl, native only
#   (hosts stay owned by issues #410/#411; dynamic musl explicitly out
#   of scope with no cell; no Windows/macOS cross-host claim).
# - SQLite: source-built C amalgamation through declared Bazel native
#   inputs, never a prebuilt glibc binary (prebuilt glibc never
#   musl-compatible by linker change alone).
# - OpenSSL: explicitly declared build tools or Bazel library inputs
#   (canonical perl plus declared tools); ambient host-tool discovery
#   rejected.
# - ring: C plus assembly with target-platform libraries; build scripts
#   keep execution-platform tools while applications link target libs.
# - bindgen: standalone rust_bindgen plus build-script routes with the
#   LLVM-22 baseline vs LLVM-23 target, explicit execution-platform
#   libclang closure, target parsing flags, and separate native link
#   (compat pinned under issue #473, never unpinned LLVM).
# - CXX: decided single-graph identity (`cxx == cxxbridge-cmd == 1.0.200`
#   from the single `crates` graph with `@crates//:cxxbridge-cmd`, never
#   a `cxx.rs` second graph, decided under issue #474) with corpus
#   execution proven here.
# - single-crate proof rejected: SQLite plus OpenSSL plus ring plus
#   bindgen plus CXX must all pass. Native only.
# - open with honest records: hermetic-llvm backend, PIE plus ELF plus
#   glibc-symbol plus cross-build completeness, floors (issue #500),
#   transport (issue #497), coverage (issue #501). Backends stay
#   provisional.
#
# Versioned here, run by CI via `bazel run //tools/ci:linux_corpus_qualification`,
# following //tools/ci:prebuilt_interop_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="cc/tests/fixtures/linux_corpus/pins.bzl"
pins_build="cc/tests/fixtures/linux_corpus/BUILD.bazel"
sqlite_h="cc/tests/fixtures/linux_corpus/sqlite.h"
sqlite_c="cc/tests/fixtures/linux_corpus/sqlite.c"
openssl_h="cc/tests/fixtures/linux_corpus/openssl.h"
openssl_c="cc/tests/fixtures/linux_corpus/openssl.c"
ring_h="cc/tests/fixtures/linux_corpus/ring.h"
ring_c="cc/tests/fixtures/linux_corpus/ring.c"
corpus_test="cc/tests/fixtures/linux_corpus/corpus_test.cc"
native="docs/native-toolchains.md"
matrix="docs/product/support-matrix.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

# Fixture files stay present (issue #499).
if [[ -f "$pins" && -f "$pins_build" && -f "$sqlite_h" && -f "$sqlite_c" && -f "$openssl_h" && -f "$openssl_c" && -f "$ring_h" && -f "$ring_c" && -f "$corpus_test" ]]; then
  ok
else
  bad "linux corpus fixture missing (want $pins plus $pins_build plus sqlite/openssl/ring h/c plus corpus_test.cc)"
fi

# Pins record the Linux glibc plus static-musl profiles, native only.
if grep -q -F -e 'LINUX_GLIBC_PROFILE = "linux_x86_64/arm64 glibc"' "$pins" &&
  grep -q -F -e 'LINUX_MUSL_PROFILE = "linux_x86_64/arm64 static musl"' "$pins" &&
  grep -q -F -e 'LINUX_NATIVE_ONLY = "Native only"' "$pins" &&
  grep -q -F -e 'dynamic musl explicitly out of scope' "$pins"; then
  ok
else
  bad "pins.bzl lost its Linux glibc plus static-musl profiles with native-only plus dynamic-out-of-scope under issue #499"
fi

# Pins record source-built SQLite (prebuilt glibc never musl-compatible).
if grep -q -F -e 'SQLITE_SHAPE = "source-built SQLite"' "$pins" &&
  grep -q -F -e 'SQLite source-built C amalgamation with declared Bazel native inputs' "$pins" &&
  grep -q -F -e 'never become musl-compatible by linker change alone' "$pins"; then
  ok
else
  bad "pins.bzl lost its source-built SQLite shape plus prebuilt-glibc rejection under issue #499"
fi

# Pins record OpenSSL with declared build tools (ambient discovery rejected).
if grep -q -F -e 'OPENSSL_SHAPE = "OpenSSL with declared build tools"' "$pins" &&
  grep -q -F -e 'declared build tools or Bazel library inputs' "$pins" &&
  grep -q -F -e '"perl"' "$pins" &&
  grep -q -F -e 'never ambient discovery' "$pins"; then
  ok
else
  bad "pins.bzl lost its OpenSSL declared-tools shape plus perl plus ambient rejection under issue #499"
fi

# Pins record ring C/assembly with target libs plus exec/target separation.
if grep -q -F -e 'RING_SHAPE = "ring-style C/assembly with target libs"' "$pins" &&
  grep -q -F -e 'C plus assembly with target-platform libraries' "$pins" &&
  grep -q -F -e 'exec-platform tools for scripts' "$pins"; then
  ok
else
  bad "pins.bzl lost its ring C/assembly plus target-libs plus exec/target shape under issue #499"
fi

# Pins record bindgen both routes plus execution libclang closure plus target flags.
if grep -q -F -e 'standalone rust_bindgen' "$pins" &&
  grep -q -F -e 'build-script route' "$pins" &&
  grep -q -F -e 'explicit execution-platform libclang closure' "$pins" &&
  grep -q -F -e 'target parsing flags' "$pins" &&
  grep -q -F -e '"--no-include-path-detection"' "$pins" &&
  grep -q -F -e '"--formatter=none"' "$pins"; then
  ok
else
  bad "pins.bzl lost its bindgen both-routes plus libclang closure plus target flags under issue #499"
fi

# Pins record the CXX single-graph execution identity (second graph rejected).
if grep -q -F -e 'CXX_IDENTITY_VERSION = "1.0.200"' "$pins" &&
  grep -q -F -e '@crates//:cxxbridge-cmd' "$pins" &&
  grep -q -F -e 'never' "$pins" &&
  grep -q -F -e 'cxx.rs' "$pins" &&
  grep -q -F -e 'CXX single-graph execution' "$pins"; then
  ok
else
  bad "pins.bzl lost its CXX single-graph execution identity plus cxx.rs rejection under issue #499"
fi

# Pins record the rejected substitutes (single-crate plus prebuilt plus ambient).
if grep -q -F -e '"single-crate proof"' "$pins" &&
  grep -q -F -e '"prebuilt glibc as musl-compatible"' "$pins" &&
  grep -q -F -e '"ambient host-tool discovery"' "$pins"; then
  ok
else
  bad "pins.bzl lost its single-crate plus prebuilt plus ambient rejection under issue #499"
fi

# Fixture BUILD composes the corpus lib plus test through the wrapper contracts.
if grep -q -F -e 'name = "corpus"' "$pins_build" &&
  grep -q -F -e 'name = "corpus_test"' "$pins_build" &&
  grep -q -F -e 'sqlite.c' "$pins_build" &&
  grep -q -F -e 'openssl.c' "$pins_build" &&
  grep -q -F -e 'ring.c' "$pins_build" &&
  grep -q -F -e 'corpus_test.cc' "$pins_build"; then
  ok
else
  bad "linux_corpus BUILD.bazel lost its corpus lib plus test composition (want corpus plus corpus_test with sqlite/openssl/ring)"
fi

# Native plan owns the qualified corpus record plus fixture proof.
if grep -q -F -e 'qualified seed-only under issue #499' "$native" &&
  grep -q -F -e 'linux_corpus_qualification' "$native" &&
  grep -q -F -e 'cc/tests/fixtures/linux_corpus/pins.bzl' "$native" &&
  grep -q -F -e 'Are both Linux profiles complete?' "$native" &&
  grep -q -F -e 'single-crate proof rejected' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its qualified corpus record with fixtures under issue #499"
fi

# Support matrix owns the qualified corpus record with no Supported claim.
if grep -q -F -e 'qualified seed-only under issue #499' "$matrix" &&
  grep -q -F -e 'linux_corpus_qualification' "$matrix" &&
  grep -q -F -e 'cc/tests/fixtures/linux_corpus/pins.bzl' "$matrix" &&
  grep -q -F -e 'single-crate proof rejected' "$matrix"; then
  ok
else
  bad "docs/product/support-matrix.md lost its qualified corpus record under issue #499"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "linux_corpus_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:linux_corpus_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the linux_corpus_qualification wiring (want target plus dogfood-freshness)"
fi

# Verification matrix owns the qualified seed-only record under #499.
if grep -q -F -e 'linux_corpus_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #499' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:linux_corpus_qualification' "$verify" &&
  grep -q -F -e '`linux_corpus_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix lost its #499 linux corpus qualified record"
fi

# Live proof: the corpus lib builds on the seed host.
if bazel build //cc/tests/fixtures/linux_corpus:corpus --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "linux corpus lib failed to build (want corpus green on the seed host, issue #499)"
fi

# Live proof: the corpus shapes test green (SQLite plus OpenSSL plus ring).
if bazel test //cc/tests/fixtures/linux_corpus:corpus_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "linux corpus test failed (want SQLite plus OpenSSL plus ring green, issue #499)"
fi

# Live proof: the wider corpus composition stays green (bindgen plus CXX plus hellos).
if bazel build //rust/tests/fixtures/bindgen:bindgen_headers //rust/tests/fixtures/cxx_identity:bridge //rust/tests/fixtures/cxx_identity:cxx_identity //rust/tests/fixtures/hello:hello //cc/tests/fixtures/hello:hello --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "linux corpus composition failed to build (want bindgen plus CXX plus hellos green, issue #499)"
fi

dx_test_summary "linux corpus qualification harness"
