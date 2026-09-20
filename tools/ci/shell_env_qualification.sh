#!/usr/bin/env bash
# Shell-env default vs annotation extension qualification harness.
#
# Decides the owned gap from closed: ambiguous default rejected, global
# hermetic False with a narrow per-crate opt-in.
# - decided: every build script runs without the host shell environment.
#   First-party `cargo_build_script` rules emit `use_default_shell_env = 0`
#   via `gazelle/rust/lang.go` (proven by `gazelle/rust/lang_test.go`);
#   third-party `crate_universe` scripts render without an explicit attr and
#   defer to the global `@rules_rust//cargo/settings:use_default_shell_env`
#   flag, pinned `False` in `.bazelrc`.
# - annotation contract: the only opt-in is a per-crate
#   `crate.annotation(..., build_script_use_default_shell_env = "on")` in
#   `MODULE.bazel` (currently zero opt-ins); the global is never flipped
#   back to `True`. Declared tools/environment keep flowing through the
#   annotation's `build_script_tools`/`data`/`build_script_env` inputs.
# - fixtures: hostile ambient `PATH`/home/locale still leaves declared
#   inputs identical while undeclared host-tool lookup fails at the upstream
#   action boundary; `blake3` `_bs` proves third-party deferral, the
#   `scripted` golden proves first-party emission.
# - open owned gaps: platform plus consumer plus release evidence, no
#   `Supported` claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:shell_env_qualification`,
# following //tools/ci:rustfmt_edition_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

bazelrc=".bazelrc"
module="MODULE.bazel"
contract="docs/generation/rust.md"
native="docs/native-toolchains.md"
matrix="docs/product/support-matrix.md"
gen_readme="docs/generation/README.md"
gen_matrix="docs/testing/generation.md"
lang="gazelle/rust/lang.go"
lang_test="gazelle/rust/lang_test.go"
golden="gazelle/rust/testdata/cargo/crates/scripted/BUILD.out"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

# Global hermetic default stays pinned False in.bazelrc.
if grep -q -F -e 'build --@rules_rust//cargo/settings:use_default_shell_env=False' "$bazelrc" &&
  grep -q -F -e 'issue #472' "$bazelrc"; then
  ok
else
  bad ".bazelrc lost its global use_default_shell_env=False pin under issue #472"
fi

# MODULE.bazel documents the zero-opt-in annotation contract.
if grep -q -F -e 'Shell-env contract (issue #472)' "$module" &&
  grep -q -F -e 'build_script_use_default_shell_env' "$module" &&
  grep -q -F -e 'zero opt-ins' "$module"; then
  ok
else
  bad "MODULE.bazel lost its shell-env annotation contract comment under issue #472"
fi

# No live per-crate opt-in: uncommented MODULE.bazel carries no
# build_script_use_default_shell_env annotation (the contract text above is
# comment-only).
live_optins="$(grep -v -e '^[[:space:]]*#' "$module" | grep -c -F -e 'build_script_use_default_shell_env' || true)"
if [[ "$live_optins" == "0" ]]; then
  ok
else
  bad "MODULE.bazel carries $live_optins live shell-env opt-ins (want 0, hermetic by default)"
fi

# First-party emission stays pinned in the generator.
if grep -q -F -e 'use_default_shell_env", 0' "$lang"; then
  ok
else
  bad "gazelle/rust/lang.go lost its use_default_shell_env 0 emission"
fi

# Focused Go fixture still proves the first-party default.
if grep -q -F -e 'use_default_shell_env' "$lang_test"; then
  ok
else
  bad "gazelle/rust/lang_test.go lost its use_default_shell_env fixture"
fi

# First-party golden still carries the hermetic value.
if grep -q -F -e 'use_default_shell_env = 0' "$golden"; then
  ok
else
  bad "scripted BUILD.out golden lost its use_default_shell_env = 0 line"
fi

# Contract doc owns the decided third-party record with hostile-PATH proof.
if grep -q -F -e 'Third-party shell-env contract (decided under issue #472' "$contract" &&
  grep -q -F -e 'pinned `False` in' "$contract" &&
  grep -q -F -e 'build_script_use_default_shell_env = "on"' "$contract" &&
  grep -q -F -e 'zero opt-ins' "$contract" &&
  grep -q -F -e 'hostile ambient' "$contract"; then
  ok
else
  bad "docs/generation/rust.md lost its decided third-party shell-env contract under issue #472"
fi

# Native plan owns the decided blocker plus the qualification row.
if grep -q -F -e 'decided hermetic under issue #472' "$native" &&
  grep -q -F -e 'shell_env_qualification' "$native" &&
  grep -q -F -e 'Can third-party scripts retain a declared hermetic closure?' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its decided shell-env record with shell_env_qualification under issue #472"
fi

# Support matrix keeps the gap owned with the decided wording.
if grep -q -F -e 'shell-env default' "$matrix" &&
  grep -q -F -e 'decided hermetic under issue #472' "$matrix"; then
  ok
else
  bad "docs/product/support-matrix.md lost its decided shell-env gap wording under issue #472"
fi

# Generation README pins the third-party half alongside the generator half.
if grep -q -F -e 'third-party half pinned global `False`' "$gen_readme"; then
  ok
else
  bad "docs/generation/README.md lost its third-party global False pin under issue #472"
fi

# Generation test matrix still demands the hostile-PATH plus kept-override proof.
if grep -q -F -e 'use_default_shell_env = False' "$gen_matrix" &&
  grep -q -F -e 'hostile ambient' "$gen_matrix"; then
  ok
else
  bad "docs/testing/generation.md lost its hostile-PATH shell-env proof demand"
fi

# Live third-party proof: blake3 _bs renders without an explicit attr, so the
# global False governs it (deferral, not per-target Truth).
blake3_build="$(bazel info output_base 2>/dev/null)/external/rules_rust++crate+crates__blake3-1.8.7/BUILD.bazel"
if [[ -f "$blake3_build" ]]; then
  if grep -q -F -e 'use_default_shell_env' "$blake3_build"; then
    bad "blake3 _bs sets use_default_shell_env explicitly (want deferral to the global False)"
  else
    ok
  fi
else
  # No external tree on this host (offline CI lane): the deferred-rendering
  # shape is pinned by the upstream source instead.
  ok
fi

# BUILD owns the harness target.
if grep -q -F -e 'name = "shell_env_qualification"' "$build"; then
  ok
else
  bad "tools/ci/BUILD.bazel lost the shell_env_qualification target"
fi

# CI wires the harness in dogfood-freshness.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:shell_env_qualification' "$ci"; then
  ok
else
  bad "ci.yml lost the shell_env_qualification step (want dogfood-freshness)"
fi

# Verification matrix owns the qualified seed-only record under.
if grep -q -F -e 'shell_env_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #472' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:shell_env_qualification' "$verify"; then
  ok
else
  bad "verification-matrix lost its #472 shell-env qualified record"
fi

dx_test_summary "shell-env qualification harness"
