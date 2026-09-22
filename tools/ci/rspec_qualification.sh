#!/usr/bin/env bash
# RSpec wiring qualification harness.
#
# Qualifies the owned gap from ADR 0032: provisional RSpec via rules_ruby,
# Ruby track owner only. Godeps-equivalent lock wiring lives in
# `gems_qualification`; this owns the runner mapping only.
# - pinned: rules_ruby 0.28.0 plus Ruby 3.4.9 in MODULE.bazel recorded in
#   `ruby/tests/fixtures/rspec/pins.bzl`; Ruby hello is stdlib-only with no
#   bundle dep.
# - runner: `ruby_test` with `main = "@bundle//bin:rspec"` plus `args`
#   naming the spec plus `deps` on the spec helper plus `@bundle` (no new
#   rule kind). Unpinned runner stays rejected.
# - open owned gaps: platform plus consumer plus release evidence, no
#   `Supported` claim. Compatibility is test mapping only.
#
# Versioned here, run by CI via `bazel run //tools/ci:rspec_qualification`,
# following //tools/ci:gotest_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="ruby/tests/fixtures/rspec/pins.bzl"
rspec_build="ruby/tests/fixtures/rspec/BUILD.bazel"
hello_build="ruby/tests/fixtures/hello/BUILD.bazel"
spec="ruby/tests/fixtures/rspec/greeter_spec.rb"
wrapper="ruby/rules/defs.bzl"
module="MODULE.bazel"
matrix="docs/product/support-matrix.md"
gen_readme="docs/generation/README.md"

if [[ -f "$pins" && -f "$rspec_build" && -f "$hello_build" && -f "$spec" ]]; then
  ok
else
  bad "rspec fixture missing (want $pins plus $rspec_build plus $hello_build plus $spec)"
fi

if grep -q -F -e 'RULES_RUBY_VERSION = "0.28.0"' "$pins" &&
  grep -q -F -e 'RUBY_VERSION = "3.4.9"' "$pins" &&
  grep -q -F -e 'RSPEC_VERSION = "3.13.0"' "$pins" &&
  grep -q -F -e 'RSPEC_MAIN = "@bundle//bin:rspec"' "$pins"; then
  ok
else
  bad "pins.bzl lost its rules_ruby plus Ruby plus RSpec mapping"
fi

if grep -q -F -e 'bazel_dep(name = "rules_ruby", version = "0.28.0")' "$module" &&
  grep -q -F -e 'ruby.toolchain(' "$module" &&
  grep -q -F -e 'version = "3.4.9"' "$module"; then
  ok
else
  bad "MODULE.bazel lost its rules_ruby 0.28.0 plus Ruby 3.4.9 pins"
fi

if grep -q -F -e 'RubyFilesInfo' "$wrapper" &&
  grep -q -F -e 'QualitySourcesInfo' "$wrapper" &&
  grep -q -F -e 'def ruby_test' "$wrapper" &&
  grep -q -F -e 'rules_ruby 0.28.0' "$wrapper"; then
  ok
else
  bad "ruby/rules/defs.bzl lost its ruby_test provider mapping"
fi

if grep -q -F -e '@bundle//bin:rspec' "$rspec_build" &&
  grep -q -F -e 'ruby_test' "$rspec_build" &&
  grep -q -F -e ':spec_helper' "$rspec_build"; then
  ok
else
  bad "ruby/tests/fixtures/rspec lost its wrapper ruby_test plus bundle mapping"
fi

if grep -q -F -e 'RSpec.describe' "$spec" &&
  grep -q -F -e 'expect(' "$spec"; then
  ok
else
  bad "ruby/tests/fixtures/rspec/greeter_spec.rb lost its RSpec shape"
fi

if grep -q -F -e 'RSpec via `rb_test`' "$matrix" &&
  grep -q -F -e 'RSpec 3.13.0' "$gen_readme"; then
  ok
else
  bad "docs lost their RSpec mapping record"
fi

dx_test_summary "rspec qualification"
