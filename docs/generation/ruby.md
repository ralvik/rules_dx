# Ruby Generation Contract

Ruby follows the [common contract](common.md). Admitted to v1 by
[ADR 0032](../decisions/0032-ruby-powershell-bandit-swift.md) on the
provisional `rules_ruby` upstream; no switch is approved here.

Wrappers in `ruby/rules/defs.bzl` preserve `RubyFilesInfo` and add
`QualitySourcesInfo`. The extension lives in `gazelle/ruby/` with
parser/naming/lang fixtures plus focused tests. Hello builds in
`ruby/tests/fixtures/hello/` consume the wrappers.

## Sources And Ownership

One directory holds one reusable `ruby_library` named after the directory
basename with sorted non-test `.rb` sources. `*_spec.rb` and `*_test.rb`
files are never library sources; handwritten `ruby_test` owns them. Thin
`ruby_binary` entries are never inferred, and a directory mixing a library
with an executable-guard source (`__FILE__ == $0`) fails so the owner
splits it first. Exact naming, test, and guard rules live in
`gazelle/ruby/naming.go` and `gazelle/ruby/parser.go`, pinned by their
fixtures.

## Resolution

Interpreter-bundled requires filter via `IsStdLib`. Every other literal
`require` uses strict common resolution over the local index, the shared
Bundler `third_party/ruby/Gemfile.lock` scope, and user-authored mappings
or exact `dx_ignore_import` exceptions. `require_relative` stays inside
the package and never becomes an edge. The extension never selects
versions or edits manifests or locks.

## Evidence

Accepted. Pinned by `bazel run //tools/ci:foundation_maps` with the
[test matrix](../testing/generation.md#generate-command-and-gazelle-extensions).
RSpec runner mapping is qualified seed-only via
`bazel run //tools/ci:rspec_qualification`; Ruby deps via
`bazel run //tools/ci:gems_qualification`. No `Supported` claim until
platform plus consumer plus release evidence passes per the
[support matrix](../product/support-matrix.md).
