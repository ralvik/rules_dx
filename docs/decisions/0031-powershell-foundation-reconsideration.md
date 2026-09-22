# ADR 0031: PowerShell Foundation Reconsideration Keeps Deferred Beyond V1

## Status

Date: 2026-09-21.

Accepted. Reconsideration scope decision for issue #778 under
[ADR 0019](0019-first-release-additional-foundations.md) and the
[first-release admission](../product/scope.md#first-release-admission).
It affirmed the PowerShell deferral; it superseded nothing. Superseded
by [ADR 0032](0032-ruby-powershell-bandit-swift.md), which admits the
PowerShell foundation to v1 for issue #970.

## Context

[ADR 0019](0019-first-release-additional-foundations.md) deferred the
PowerShell application foundation beyond v1 on a single young
execution-only upstream with unproven generation, dependency,
environment, and IDE stories exceeding the low-cost hermetic bar, while
keeping PSScriptAnalyzer in v1 scope via the exact-module plus portable
`pwsh` runtime route. The support matrix tracks PowerShell
reconsideration under issue #778. This record is that decision.

Re-review of upstream as of 2026-09-21 (documentation/source
observations, not executed qualification):

- Single young execution-only upstream remains:
  [periareon/rules_powershell](https://github.com/periareon/rules_powershell)
  (BCR-published `rules_powershell` 0.2.0, April 2026; only three
  releases since 0.0.4 in September 2025, 18 commits, two maintainers,
  no stars or forks). Its stated goal is feature parity with
  `rules_shell`: it ships only `pwsh_binary`, `pwsh_library`,
  `pwsh_test` plus a `PwshInfo` provider (`imports`/`srcs` for
  `PSModulePath` setup) and a `pwsh` toolchain download extension
  (default 7.5.4). That is script execution, not a complete application
  foundation.
- No low-cost generation story: no PowerShell Gazelle extension ships
  with the ruleset or in `gazelle/`; the published docs cover only the
  three execution rules plus toolchain registration. Admitted
  foundations each carry a Gazelle plus naming story; PowerShell does
  not.
- No low-cost dependency story: no PowerShell Gallery / PSResourceGet
  lock, fail-closed repin, or declared-usage checker mapping exists in
  the ruleset or here. `PwshInfo` carries module paths only, not lock
  authority. Admitted foundations each carry such a lock plus
  toolchain story; PowerShell does not.
- No low-cost environment/IDE story: no env plan, editor driver, or
  qualified test-runner mapping beyond `pwsh_test` script execution
  exists here. The `pwsh` toolchain (`powershell.toolchain` extension,
  per-platform archives from `PowerShell/PowerShell` releases,
  execution-platform lazy) is an unqualified download route, not a
  qualified hermetic story meeting the
  [required platforms](0014-tested-platform-release-stack.md#required-platforms)
  including musl and MSVC-compatible Windows.

## Decision

PowerShell application foundation stays deferred beyond v1. This is a
continued deferral, not an exclusion: a future admission needs a new
evidence-backed scope decision with a qualification plan, not automatic
promotion.

PSScriptAnalyzer stays v1 scope via the decided exact-module plus
portable-PowerShell-runtime route in
[tool acquisition](../tools/tool-acquisition.md#delivery-classes); this
deferral removes no baseline tool.

## Consequences

- No `powershell/` foundation dirs, wrappers, Gazelle extensions, env
  plans, hello builds, or `MODULE.bazel` deps; `powershell` class stays
  classified with no adapter claim (`quality/adapters.bzl` plus
  `PARITY_DEFERRED`).
- The [support matrix](../product/support-matrix.md#deferred-beyond-v1)
  records this reconsideration; other docs link there instead of copying it.
- Future admission must qualify wrappers plus providers, Gazelle plus
  naming, env plans, hello builds, Gallery/PSResource lock fail-closed
  plus declared-usage checking with offline proof, quality adapters, and
  every required platform including musl and MSVC-compatible Windows,
  before any promotion claim.

## Rejected Alternatives

- Admit PowerShell to v1 now: rejected; execution-only plus
  generation/dependency/env/IDE gaps above still exceed the low-cost
  hermetic bar, and admission would open per-host qualification without
  a lock story.
- Exclude PowerShell (and PSScriptAnalyzer) from v1: rejected; the tool
  closure route is viable and the foundation has a plausible future
  path, so deferral with retained tools is the proportionate disposition.
