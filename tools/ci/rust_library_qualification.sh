#!/usr/bin/env bash
# Rust library extraction qualification harness.
#
# The `Rust library extraction` planning line tracked in GitHub issues with no
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
#   the GitHub issue #469 decided line.
#
# Versioned here, run by CI via `bazel run //tools/ci:rust_library_qualification`,
# following //tools/ci:closeout_battery_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

adr="docs/decisions/0023-rust-libraries-internal.md"
adr_index="docs/decisions/README.md"
arch="docs/architecture/facade.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Planned work lives in GitHub issues only (docs/roadmap.md removed under #981).
if [[ ! -f "docs/roadmap.md" ]]; then
  ok
else
  bad "docs/roadmap.md still exists (planned work lives in GitHub issues only, #981)"
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

# Architecture owns the boundary section linking ADR 0023 under.
if grep -q -F -e '## Rust Library Boundary (Issue #469)' "$arch" &&
  grep -q -F -e '0023-rust-libraries-internal.md' "$arch" &&
  grep -q -F -e 'rust_library_qualification' "$arch" &&
  grep -q -F -e 'only' "$arch"; then
  ok
else
  bad "facade lost its Rust library boundary section with ADR 0023 plus guard under #469"
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

# Public Rust boundary stays entry points plus public Starlark API:
# explicit public visibility only in cli/cli plus cli/env plus dx (entry
# points, Cargo exports, man pages) plus deploy/rules (public Starlark
# rules) (hermetic file list: BSD grep lacks --include; BRE class spelled
# as Python `\s`, issue #1006).
explicit_public_files="$(dx_hermetic_grep tree-list --re --include 'BUILD.bazel' --roots . -- '^\s*visibility = \["//visibility:public"\]' 2>/dev/null | sed -e 's|^\./||' | sort | tr '\n' ' ' | sed -e 's/ $//')"
if [[ "$explicit_public_files" == "cli/cli/BUILD.bazel cli/env/BUILD.bazel deploy/rules/BUILD.bazel dx/BUILD.bazel" ]]; then
  ok
else
  bad "explicit public targets drifted (want cli/cli plus cli/env plus dx plus deploy/rules, got: $explicit_public_files)"
fi

# No Rust library package defaults to public (crate directories only; the
# quality/ plus generation/ roots stay public Starlark API per visibility)
# (hermetic file list, issue #1006).
public_defaults="$(dx_hermetic_grep tree-list --fixed --include 'BUILD.bazel' --roots cli libs quality/adapter quality/evaluator quality/markdown quality/result quality/runner generation/result generation/codegen_shard -- 'package(default_visibility = ["//visibility:public"])' 2>/dev/null | tr '\n' ' ' | sed -e 's/ $//')"
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
if grep -q -F -e 'name = "rust_library_qualification"' "tools/ci/ci_targets_c.bzl" &&
  grep -q -F -e 'rust_library_qualification.sh' "tools/ci/ci_targets_c.bzl"; then
  ok
else
  bad "tools/ci/ci_targets_c.bzl lost its rust_library_qualification target"
fi

if grep -q -F -e '//tools/ci:rust_library_qualification' tools/ci/dogfood_freshness.sh; then
  ok
else
  bad "dogfood_freshness.sh lost its rust_library_qualification step"
fi

dx_test_summary "rust library extraction harness"
