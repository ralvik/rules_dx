# Adopt Ruby example

Foreign Ruby tree adopted without upstream changes: a `greet` package with
an RSpec spec plus a stdlib-only `solo` package. The tree arrived with no
`MODULE.bazel` and no `BUILD` files; the `*/BUILD.bazel` files are
generator-owned (`dx generate` output).

```sh
bazel run //cli/cli:dx -- init //examples/adopt-ruby/...
bazel run //cli/cli:dx -- generate //examples/adopt-ruby/...
bazel build //examples/adopt-ruby/...
bazel test //examples/adopt-ruby/...
```

Evidence: `dx init` writes nothing inside the tree (absent-only). Generation
emits one `ruby_library` per directory (named after the directory basename);
`greet_spec.rb` stays handwritten `ruby_test` with
`main = "@bundle//bin:rspec"` over the shared Bundler lock. The `greet`
library carries one pinned bundle dep (`rspec 3.13.0` via `Gemfile` plus
`Gemfile.lock` plus `@bundle` through an exact `# gazelle:resolve`
mapping where needed); `RSpec.describe` plus `expect` prove build and test
over the lock. Depcheck `locks` consistency (`Gemfile` require matches
`Gemfile.lock`), quality (`QualitySourcesInfo`), and coverage
(`InstrumentedFilesInfo`) flow through the shared wrappers. Build covers
4 targets; both tests pass (`greet_spec`, `solo_test`).

Scope notes: cross-package Ruby requires via full require paths resolve only
through an exact `# gazelle:resolve` mapping today. Regeneration is the
composed `dx generate` run.
