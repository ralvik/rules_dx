# ADR 0030: Ruby Foundation Reconsideration Keeps Deferred Beyond V1

## Status

Date: 2026-09-21.

Accepted. Reconsideration scope decision for issue #777 under
[ADR 0019](0019-first-release-additional-foundations.md) and the
[first-release admission](../product/scope.md#first-release-admission).
It affirmed the Ruby deferral; it superseded nothing. Superseded by
[ADR 0032](0032-ruby-powershell-bandit-swift.md), which admits the Ruby
foundation to v1 for issue #970.

## Context

[ADR 0019](0019-first-release-additional-foundations.md) deferred the Ruby
application foundation beyond v1 on fragmented ruleset maintenance
ownership plus gem/bundler packaging effort exceeding the low-cost
hermetic bar, while keeping RuboCop/StandardRB in v1 scope via the
release-assembled closure. The support matrix tracks Ruby reconsideration
under issue #777. This record is that decision.

Re-review of upstream as of 2026-09-21 (documentation/source
observations, not executed qualification):

- Canonical ruleset has consolidated on
  [bazel-contrib/rules_ruby](https://github.com/bazel-contrib/rules_ruby)
  (Bzlmod-published `rules_ruby` 0.28.0, Bazel 8 tested; the former
  [bazelruby/rules_ruby](https://github.com/bazelruby/rules_ruby) points
  there and is retired). Single-maintainer BCR ownership remains.
- Hermetic toolchain still has platform gaps:
  [portable Ruby](https://github.com/bazel-contrib/portable-ruby) covers
  Linux/macOS x86_64/arm64 only; Windows falls back to RubyInstaller and
  `portable_ruby` has no effect there or on JRuby/TruffleRuby. No
  static-musl or MSVC-compatible hermetic story meets the
  [required platforms](0014-tested-platform-release-stack.md#required-platforms).
- Bundler packaging still exceeds the bar:
  [`rb_bundle_fetch` fails on git gems](https://github.com/bazel-contrib/rules_ruby/issues/62)
  (open), ignores the Bundler `CHECKSUMS` section in favor of a separate
  `gem_checksums` attribute
  ([issue #63](https://github.com/bazel-contrib/rules_ruby/issues/63),
  open), and platform-conditional gem resolution plus private-git
  credentials remain unresolved. Native-extension and JRuby jar edges
  needed dedicated fixes (for example jar fetch in early 2026).
- No low-cost generation/environment/IDE/dependency story: no Ruby
  Gazelle extension ships with the ruleset or in `gazelle/`; no env plan,
  editor driver, `Gemfile.lock` fail-closed plus declared-usage checker
  mapping, or qualified test-runner mapping exists here. Admitted
  foundations each carry such a lock plus toolchain story; Ruby does not.

## Decision

Ruby application foundation stays deferred beyond v1. This is a continued
deferral, not an exclusion: a future admission needs a new evidence-backed
scope decision with a qualification plan, not automatic promotion.

RuboCop and StandardRB stay v1 scope via the decided release-assembled
Ruby closure route in
[tool acquisition](../tools/tool-acquisition.md#delivery-classes); this
deferral removes no baseline tool.

## Consequences

- No `ruby/` foundation dirs, wrappers, Gazelle extensions, env plans,
  hello builds, or `MODULE.bazel` deps; `ruby` class stays classified
  with no adapter claim (`quality/adapters.bzl` plus `PARITY_DEFERRED`).
- The [support matrix](../product/support-matrix.md)
  records this reconsideration; other docs link there instead of copying it.
- Future admission must qualify wrappers plus providers, Gazelle plus
  naming, env plans, hello builds, `Gemfile.lock` fail-closed plus
  declared-usage checking with offline proof, quality adapters, and every
  required platform including musl and MSVC-compatible Windows, before any
  promotion claim.

## Rejected Alternatives

- Admit Ruby to v1 now: rejected; toolchain plus bundler plus
  generation/env/IDE gaps above still exceed the low-cost hermetic bar,
  and admission would open per-host qualification without a lock story.
- Exclude Ruby (and RuboCop/StandardRB) from v1: rejected; the tool
  closure route is viable and the foundation has a plausible future path,
  so deferral with retained tools is the proportionate disposition.
