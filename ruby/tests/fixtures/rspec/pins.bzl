"""RSpec 3.13.0 version pins.

Contract: `docs/product/support-matrix.md#provisional-default-test-runners`,
`docs/generation/README.md#language-mapping-qualification`.
"""

# Upstream ruleset pin (MODULE.bazel plus MODULE.bazel.lock).
RULES_RUBY_VERSION = "0.28.0"

# Toolchain pin (ruby.toolchain in MODULE.bazel).
RUBY_VERSION = "3.4.9"

# Runner pin: RSpec 3.13.0 line (latest 3.x; 4.x not stable).
RSPEC_VERSION = "3.13.0"
RSPEC_CORE_VERSION = "3.13.0"
RSPEC_EXPECTATIONS_VERSION = "3.13.0"
RSPEC_MOCKS_VERSION = "3.13.0"
RSPEC_SUPPORT_VERSION = "3.13.1"
DIFF_LCS_VERSION = "1.5.0"

# Bundle coordinates resolved by rb_bundle_fetch from the shared
# Gemfile.lock (single lock via gemfile plus gemfile_lock in
# third_party/ruby pins over the shared `@bundle` hub).
RSPEC_GEMS = [
    "rspec:3.13.0",
    "rspec-core:3.13.0",
    "rspec-expectations:3.13.0",
    "rspec-mocks:3.13.0",
    "rspec-support:3.13.1",
    "diff-lcs:1.5.0",
]

# Runner mapping: `ruby_test` with `main = "@bundle//bin:rspec"` plus
# `args` naming the spec file and `deps` on the spec helper plus `@bundle`
# (no new rule kind; the runner itself needs no new rule, only the lock
# story above).
RSPEC_MAIN = "@bundle//bin:rspec"
RSPEC_KIND = "ruby_test"

# Live proof labels (wrapper consumers over the pinned hub).
RSPEC_FIXTURE_LIB = "//ruby/tests/fixtures/rspec:greeter_lib"
RSPEC_FIXTURE_TEST = "//ruby/tests/fixtures/rspec:greeter_spec"

# Rejected: unpinned runner (floating version, living at head, implicit
# `rspec` without the Bundler lock).
RSPEC_REJECTED = "unpinned runner rejected: no floating version or head"

# Currency recheck (issue #932): directives below were verified current on
# this date (rspec 3.13.0 per ADR 0008 latest-stable). Refresh the date with
# each dependency-currency pass; pin_consistency.sh fails when absent or stale.
RSPEC_CURRENCY_RECHECK = "2026-09-22"
