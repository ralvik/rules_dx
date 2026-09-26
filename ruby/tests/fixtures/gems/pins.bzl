RULES_RUBY_VERSION = "0.28.0"

RUBY_VERSION = "3.4.9"

GEMFILE = "//third_party/ruby:Gemfile"
GEMFILE_LOCK = "//third_party/ruby:Gemfile.lock"
BUNDLE_HUB = "@bundle"
BUNDLE_BIN_RSPEC = "@bundle//bin:rspec"

GEMFILE_LOCK_ATTR = "gemfile_lock"

GEMS_GENERATION = "consumes never writes"

GEMS_FIXTURE_HELLO = "//ruby/tests/fixtures/hello:hello_lib"
GEMS_FIXTURE_RSPEC = "//ruby/tests/fixtures/rspec:greeter_spec"

GEMS_REJECTED = "consumer Bundler rejected: lock plus depcheck only; git gems rejected per rules_ruby#62"

GEMS_CURRENCY_RECHECK = "2026-09-22"
