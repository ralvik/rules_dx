#!/usr/bin/env bash
# Rust library extraction qualification harness (issue #469).
#
# docs/roadmap.md carried the bare line `Rust library extraction` with no
# owner, while cli/* crates plus quality/runner plus libs/ boundaries stayed
# unclear. This harness pins the decided internal-only outcome:
# - decided: 34 internal Rust crates stay internal (27 cli/* plus 5
#   quality/* plus 2 generation/*), all version 0.0.0 unpublishable, no
#   extraction, no crates.io publication, no separate repository;
# - boundary: only //cli/cli:dx and //cli/env:env are public tool entry
#   points; every Rust library stays scoped, and libs/ is Starlark-only;
# - consumer migration: none (consumers use the binaries plus the //dx
#   Starlark facade, never the libraries directly);
# - canonical record: ADR 0023 plus the architecture boundary section plus
#   the roadmap decided line.
#
# Versioned here, run by CI via `bazel run //tools/ci:rust_library_qualification`,
# following //tools/ci:closeout_battery_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

roadmap="docs/roadmap.md"
adr="docs/decisions/0023-rust-libraries-internal.md"
adr_index="docs/decisions/README.md"
arch="docs/architecture/README.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Roadmap owns the decided internal-only record under #469.
if grep -q -F -e 'Rust library extraction decided internal-only under issue #469' "$roadmap" &&
  grep -q -F -e 'ADR 0023' "$roadmap"; then
  ok
else
  bad "roadmap lost its Rust library decided internal-only record under #469 with ADR 0023"
fi

# ADR 0023 exists and stays Accepted with inventory plus boundary plus wont-extract.
if [[ -f "$adr" ]] &&
  grep -q -F -e '# ADR 0023: Rust Libraries Stay Internal' "$adr" &&
  grep -q -F -e 'Accepted.' "$adr" &&
  grep -q -F -e '34 internal Rust crates' "$adr" &&
  grep -q -F -e 'no extraction' "$adr" &&
  grep -q -F -e 'issue #469' "$adr"; then
  ok
else
  bad "ADR 0023 lost its Accepted internal-only record with 34-crate inventory under #469"
fi

# ADR index owns the 0023 entry.
if grep -q -F -e '0023-rust-libraries-internal.md' "$adr_index" &&
  grep -q -F -e 'Rust Libraries Stay Internal' "$adr_index"; then
  ok
else
  bad "decisions README lost its ADR 0023 index entry"
fi

# Architecture owns the boundary section linking ADR 0023 under #469.
if grep -q -F -e '### Rust Library Boundary (Issue #469)' "$arch" &&
  grep -q -F -e '0023-rust-libraries-internal.md' "$arch" &&
  grep -q -F -e 'rust_library_qualification' "$arch" &&
  grep -q -F -e 'only' "$arch"; then
  ok
else
  bad "architecture lost its Rust library boundary section with ADR 0023 plus guard under #469"
fi

# Inventory holds: 27 cli/* plus 5 quality/* plus 2 generation/* Cargo manifests.
cli_count="$(ls cli/*/Cargo.toml 2>/dev/null | wc -l | tr -d ' ')"
quality_count="$(ls quality/*/Cargo.toml 2>/dev/null | wc -l | tr -d ' ')"
generation_count="$(ls generation/*/Cargo.toml 2>/dev/null | wc -l | tr -d ' ')"
if [[ "$cli_count" == "27" && "$quality_count" == "5" && "$generation_count" == "2" ]]; then
  ok
else
  bad "Rust crate inventory drifted (want 27 cli plus 5 quality plus 2 generation Cargo.toml, got $cli_count plus $quality_count plus $generation_count)"
fi

# Every Rust manifest stays version 0.0.0 (unpublishable, no publish metadata).
unversioned=""
for f in cli/*/Cargo.toml quality/*/Cargo.toml generation/*/Cargo.toml; do
  if ! grep -q -F -e 'version = "0.0.0"' "$f"; then
    unversioned="$unversioned $f"
  fi
done
if [[ -z "$unversioned" ]]; then
  ok
else
  bad "Rust manifests lost their 0.0.0 unpublishable pin:$unversioned"
fi

# libs/ stays Starlark-only (no Rust crates to extract).
if ! find libs -name 'Cargo.toml' -o -name '*.rs' 2>/dev/null | grep -q .; then
  ok
else
  bad "libs/ gained Rust sources (want Starlark-only, no Cargo.toml or .rs)"
fi

# Public Rust boundary stays binaries-only: explicit public visibility only
# in cli/cli plus cli/env (entry points, Cargo exports, man pages).
explicit_public_files="$(grep -rl -e '^[[:space:]]*visibility = \["//visibility:public"\]' --include='BUILD.bazel' . 2>/dev/null | sed -e 's|^\./||' | sort | tr '\n' ' ' | sed -e 's/ $//')"
if [[ "$explicit_public_files" == "cli/cli/BUILD.bazel cli/env/BUILD.bazel" ]]; then
  ok
else
  bad "explicit public targets leaked (want only cli/cli/BUILD.bazel plus cli/env/BUILD.bazel, got: $explicit_public_files)"
fi

# No Rust library package defaults to public (crate directories only; the
# quality/ plus generation/ roots stay public Starlark API per visibility).
public_defaults="$((grep -rl -F -e 'package(default_visibility = ["//visibility:public"])' --include='BUILD.bazel' cli libs quality/adapter quality/evaluator quality/markdown quality/result quality/runner generation/result generation/codegen_shard 2>/dev/null || true) | tr '\n' ' ' | sed -e 's/ $//')"
if [[ -z "$public_defaults" ]]; then
  ok
else
  bad "Rust library package defaulted to public (want scoped only, got: $public_defaults)"
fi

# cli/ root owns the binaries-only boundary statement.
if grep -q -F -e 'Only `//cli/cli:dx` and `//cli/env:env` are public tool entry points' cli/BUILD.bazel; then
  ok
else
  bad "cli/BUILD.bazel lost its binaries-only public boundary statement"
fi

# BUILD target plus CI wiring stay pinned.
if grep -q -F -e 'name = "rust_library_qualification"' "$build" &&
  grep -q -F -e 'rust_library_qualification.sh' "$build"; then
  ok
else
  bad "tools/ci/BUILD.bazel lost its rust_library_qualification target"
fi

if grep -q -F -e '//tools/ci:rust_library_qualification' "$ci"; then
  ok
else
  bad "ci.yml lost its rust_library_qualification step"
fi

dx_test_summary "rust library extraction harness"
