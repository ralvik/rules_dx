# ADR 0032: Admit Ruby and PowerShell Foundations, Re-select Bandit, Swift Spike Keeps Exclusion

## Status

Date: 2026-09-22.

Accepted. Scope decision for issue #970 under
[ADR 0019](0019-first-release-additional-foundations.md) and the
[first-release admission](../product/scope.md#first-release-admission).
It supersedes the ADR 0019 Ruby/PowerShell deferral plus Swift/Bandit
exclusions, and the ADR 0030 Ruby plus ADR 0031 PowerShell continued
deferrals.

## Context

[ADR 0019](0019-first-release-additional-foundations.md) deferred the Ruby
and PowerShell application foundations beyond v1 and excluded
Swift/SwiftFormat plus Bandit as evidence-backed v1 scope decisions.
[ADR 0030](0030-ruby-foundation-reconsideration.md) (issue #777) and
[ADR 0031](0031-powershell-foundation-reconsideration.md) (issue #778)
re-reviewed both foundations on 2026-09-21 and kept the deferrals on
fragmented Ruby ruleset plus gem/bundler effort and on the single young
execution-only PowerShell upstream with unproven generation, dependency,
environment, and IDE stories. Direction is now to implement, not defer.
Per the reversal rule a material reversal needs a new superseding record
with links, not an edit in place.

Re-review of upstream as of 2026-09-22 (documentation/source
observations, not executed qualification):

- Ruby: canonical ruleset has consolidated on
  [bazel-contrib/rules_ruby](https://github.com/bazel-contrib/rules_ruby)
  (Bzlmod-published `rules_ruby` 0.28.0, Bazel 8 tested; the former
  `bazelruby/rules_ruby` points there and is retired). Hermetic toolchain
  still has platform gaps:
  [portable Ruby](https://github.com/bazel-contrib/portable-ruby) covers
  Linux/macOS x86_64/arm64 only; Windows falls back to RubyInstaller.
  Bundler packaging still has open gaps:
  [`rb_bundle_fetch` fails on git gems](https://github.com/bazel-contrib/rules_ruby/issues/62)
  (open), ignores the Bundler `CHECKSUMS` section in favor of a separate
  `gem_checksums` attribute
  ([issue #63](https://github.com/bazel-contrib/rules_ruby/issues/63),
  open). No Ruby Gazelle extension ships with the ruleset or in
  `gazelle/`; no env plan, editor driver, or `Gemfile.lock` fail-closed
  plus declared-usage checker mapping exists here. RSpec runs through the
  existing `rb_test` shape with `main = "@bundle//bin:rspec"`, so the
  runner itself needs no new rule kind, only the lock story above.
- PowerShell: single young execution-only upstream remains:
  [periareon/rules_powershell](https://github.com/periareon/rules_powershell)
  (BCR-published `rules_powershell` 0.2.0, April 2026). It ships only
  `pwsh_binary`, `pwsh_library`, `pwsh_test` plus a `PwshInfo` provider
  and a `pwsh` toolchain download extension (default 7.5.4). No
  PowerShell Gazelle extension, no Gallery/PSResourceGet lock with
  fail-closed repin, no env plan, and no qualified test-runner mapping
  beyond `pwsh_test` script execution exist here. The portable `pwsh`
  runtime route (7.6.5 LTS observed) already decided for PSScriptAnalyzer
  in [tool acquisition](../tools/tool-acquisition.md#delivery-classes)
  extends directly to the foundation. Pester runs through `pwsh_test`
  script execution, so the runner needs no new rule kind, only the lock
  story.
- Bandit: [PyCQA/bandit](https://github.com/PyCQA/bandit) (1.8.x line,
  Apache-2.0) is an AST-based Python security linter with JSON/SARIF
  output over a PyPI wheel. It fits the decided private wheel-only Python
  graph plus shared managed Python runtime route (same class as flake8,
  pylint, djlint, yamllint), with no sdist fallback and no installer on
  the consumer path. It complements, not replaces, the Ruff S
  (flake8-bandit) selection qualified seed-only under closed #801.
- Swift: [rules_swift](https://github.com/bazelbuild/rules_swift) ships a
  standalone toolchain extension downloading from swift.org for `xcode`
  (macOS Apple-silicon plus Intel, `.pkg` extractable on a macOS host
  only) plus `ubuntu22.04`, `ubuntu24.04`, `debian12`, `fedora39`,
  `amazonlinux2`, `ubi9` (each plus `aarch64` variants), with explicit
  per-distribution toolchain registration and `CC=clang` plus host deps
  (ICU, Clang) on Linux. There is no Windows (x86_64/arm64) hermetic
  route, no cross-distro auto-selection, and the
  default remains the host toolchain/Xcode. SwiftFormat v2.8.0 needs a
  host Swift toolchain, which stays forbidden. The spike finds no
  hermetic toolchain covering all
  [required hosts](0014-tested-platform-release-stack.md#required-platforms)
  per #965.

## Decision

Ruby and PowerShell application foundations are admitted to v1. This
supersedes the ADR 0019 deferral and the ADR 0030/0031 continued
deferrals. Provisional upstreams are `rules_ruby` 0.28.0 and
`rules_powershell` 0.2.0; exact versions, mappings, and adapter work stay
provisional pending qualification. Delivery splits into a Ruby track plus
a PowerShell track as parallel follow-ups.

RSpec (Ruby) and Pester (PowerShell) are confirmed as the provisional
test runners. Each must pass the same hermetic/Bzlmod/pinning bar:
Bzlmod-published ruleset, `Gemfile.lock` / Gallery lock fail-closed with
declared-usage checking and offline proof, lazy fetch with no consumer
installer, and every required host including MSVC-compatible
Windows.

Bandit exclusion is revoked. Bandit is selected as a Python source-audit
tool alongside Ruff S via the private wheel-only Python graph plus
shared managed Python runtime route. Exact version, ruleset, and adapter
wiring stay provisional pending qualification owned by issue #801
(successor to closed #613); the curated-audit empty set stays until that
wiring lands.

Swift/SwiftFormat exclusion stands, re-evidenced by this conditional
spike. Admission in this record required a hermetic toolchain covering
all required hosts per #965; the spike found none, so no admission is
made here. Reconsideration requires a new evidence-backed scope decision.

Every new foundation must meet the all-hosts required bar (#965) and the
thin-CLI/facade/laziness fit (#957-959). Unresolved cells block
qualification. There is no post-v1 bucket: everything lands v1 or gets
an evidence-backed exclusion.

## Consequences

- No `ruby/`, `powershell/`, or `swift/` foundation dirs, wrappers,
  Gazelle extensions, env plans, hello builds, or `MODULE.bazel` deps
  land here; admission is scope (`Planned`), not delivery. The
  [support matrix](../product/support-matrix.md)
  records the new dispositions; other docs link there instead of copying
  them.
- Ruby track must qualify wrappers plus providers, Gazelle plus naming,
  env plans, hello builds, `Gemfile.lock` fail-closed plus
  declared-usage checking with offline proof, RSpec mapping, quality
  adapters, and every required platform before any promotion claim.
- PowerShell track must qualify wrappers plus providers, Gazelle plus
  naming, env plans, hello builds, Gallery/PSResource lock fail-closed
  plus declared-usage checking with offline proof, Pester mapping,
  quality adapters, and every required platform before any promotion
  claim.
- RuboCop/StandardRB stay v1 scope via the release-assembled Ruby
  closure route; PSScriptAnalyzer stays v1 scope via the exact-module
  plus portable-`pwsh` route. Foundation admission extends those routes;
  it selects no second runtime.
- Python source-audit wiring, fixtures, and promotion evidence stay owned
  by issue #801; taxonomy stays taxonomy-only under closed #512.

## Rejected Alternatives

- Keep the ADR 0019 deferral/exclusions as re-affirmed by ADR 0030/0031:
  rejected; implement-now direction.
- Admit Swift/SwiftFormat on the standalone toolchain: rejected; the
  spike shows Windows plus cross-distro plus host-extract gaps
  against the all-required-hosts bar.
- Leave Bandit excluded with Ruff S alone: rejected; Bandit fits the
  decided wheel-only route and complements Ruff S.
- Admit the foundations without confirming RSpec/Pester: rejected; the
  runners must clear the same hermetic/Bzlmod/pinning bar before
  qualification.
