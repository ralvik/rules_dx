"""Gemfile.lock wiring pins.

Contract: `docs/product/support-matrix.md#provisional-default-dependency-locks`,
`docs/generation/README.md#language-mapping-qualification`.
"""

# Upstream ruleset pin (MODULE.bazel plus MODULE.bazel.lock).
RULES_RUBY_VERSION = "0.28.0"

# Toolchain pin (ruby.toolchain in MODULE.bazel).
RUBY_VERSION = "3.4.9"

# Lock authority (maintainer-owned, committed, never hand-edited beyond
# the documented seed-host regeneration).
GEMFILE = "//third_party/ruby:Gemfile"
GEMFILE_LOCK = "//third_party/ruby:Gemfile.lock"
BUNDLE_HUB = "@bundle"
BUNDLE_BIN_RSPEC = "@bundle//bin:rspec"

# Hub wiring: every entry carries its version in Gemfile.lock so the
# Bundler fetch verifies each artifact (fail-closed on stale lock).
GEMFILE_LOCK_ATTR = "gemfile_lock"

# Generation contract: consumes the Gemfile files, never writes them.
GEMS_GENERATION = "consumes never writes"

# Live proof labels (wrapper consumers over the pinned hub).
GEMS_FIXTURE_HELLO = "//ruby/tests/fixtures/hello:hello_lib"
GEMS_FIXTURE_RSPEC = "//ruby/tests/fixtures/rspec:greeter_spec"

# Rejected: consumer Bundler runs on the consumer path; generation writes
# the lock; git gems per rules_ruby issue 62; CHECKSUMS section per issue 63
# in favor of the separate `gem_checksums` attribute).
GEMS_REJECTED = "consumer Bundler rejected: lock plus depcheck only; git gems rejected per rules_ruby#62"

# this date (rspec 3.13.0 per ADR 0008 latest-stable). Refresh the date with
# each dependency-currency pass; pin_consistency.sh fails when absent or stale.
GEMS_CURRENCY_RECHECK = "2026-09-22"
