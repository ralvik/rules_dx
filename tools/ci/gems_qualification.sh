#!/usr/bin/env bash
# Gemfile.lock wiring qualification harness.
#
# Qualifies the owned gap from ADR 0032: provisional Bundler lock via
# rb_bundle_fetch, Ruby track owner only. RSpec mapping lives in
# `rspec_qualification`; this owns the lock wiring only.
# - wired: `third_party/ruby/Gemfile` plus `Gemfile.lock` (rspec 3.13.0)
#   via `ruby.bundle_fetch(name = "bundle", ...)` into `@bundle` carrying
#   Bundler verification, loaded from MODULE.bazel. Single-lock layout:
#   generation consumes never writes. Stale locks fail closed.
# - fixtures: `ruby/tests/fixtures/rspec/` library plus test consume
#   `@bundle//bin:rspec` via handwritten deps, depcheck testdata proves
#   offline lockfile-consistency plus usage authority (`Gemfile` plus
#   `Gemfile.lock`).
# - rejected: consumer Bundler runs, generation writes
#   the lock, git gems per rules_ruby#62.
# - scope: lock only. Per-platform acquisition, platform plus consumer plus
#   release evidence stay open; no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:gems_qualification`,
# following //tools/ci:godeps_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="ruby/tests/fixtures/gems/pins.bzl"
pins_build="ruby/tests/fixtures/gems/BUILD.bazel"
gemfile="third_party/ruby/Gemfile"
lock="third_party/ruby/Gemfile.lock"
module="MODULE.bazel"
matrix="docs/product/support-matrix.md"
gen_readme="docs/generation/foundation-qualification.md"
checker="tools/depcheck/src/lib.rs"

if [[ -f "$pins" && -f "$pins_build" && -f "$gemfile" && -f "$lock" ]]; then
  ok
else
  bad "gems pins fixture missing (want $pins plus $pins_build plus $gemfile plus $lock)"
fi

if grep -q -F -e 'RULES_RUBY_VERSION = "0.28.0"' "$pins" &&
  grep -q -F -e 'GEMFILE = "//third_party/ruby:Gemfile"' "$pins" &&
  grep -q -F -e 'GEMFILE_LOCK = "//third_party/ruby:Gemfile.lock"' "$pins"; then
  ok
else
  bad "pins.bzl lost its rules_ruby plus Gemfile wiring"
fi

if grep -q -F -e 'ruby.bundle_fetch(' "$module" &&
  grep -q -F -e 'gemfile = "//third_party/ruby:Gemfile"' "$module" &&
  grep -q -F -e 'gemfile_lock = "//third_party/ruby:Gemfile.lock"' "$module"; then
  ok
else
  bad "MODULE.bazel lost its ruby.bundle_fetch Gemfile wiring"
fi

if grep -q -F -e 'gem "rspec", "3.13.0"' "$gemfile" &&
  grep -q -F -e 'rspec (3.13.0)' "$lock"; then
  ok
else
  bad "third_party/ruby lock lost its rspec 3.13.0 pin"
fi

if [[ -f "tools/depcheck/testdata/ruby/ok_used/Gemfile" &&
  -f "tools/depcheck/testdata/ruby/ok_used/Gemfile.lock" ]] &&
  grep -q -F -e 'Gemfile' "$checker" &&
  grep -q -F -e 'Gemfile.lock' "$checker"; then
  ok
else
  bad "depcheck lost its ruby lock authority fixtures plus parser"
fi

if grep -q -F -e 'Gemfile.lock' "$matrix" &&
  grep -q -F -e 'Gemfile' "$gen_readme"; then
  ok
else
  bad "docs lost their Gemfile.lock record"
fi

dx_test_summary "gems qualification"
