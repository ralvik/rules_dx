# PowerShell Generation Contract

PowerShell follows the [common contract](common.md). Admitted to v1 by
[ADR 0032](../decisions/0032-ruby-powershell-bandit-swift.md) (superseding the
[ADR 0019](../decisions/0019-first-release-additional-foundations.md) deferral)
on the provisional `rules_powershell` 0.2.0 upstream; no switch is approved here.

Wrappers in `powershell/rules/defs.bzl` preserve `PwshInfo` and add
`QualitySourcesInfo`. Hello builds in `powershell/tests/fixtures/hello/`
consume the wrappers.

## Upstream Assessment

Single young execution-only upstream remains:
[periareon/rules_powershell](https://github.com/periareon/rules_powershell)
(BCR-published `rules_powershell` 0.2.0, April 2026). It ships only
`pwsh_binary`, `pwsh_library`, `pwsh_test` plus a `PwshInfo` provider
(`imports`/`srcs` for `PSModulePath` setup) and a `pwsh` toolchain download
extension (default 7.5.4, covering `linux_x64`, `linux_arm64`, `osx_arm64`,
`osx_x64`, `win_x64`, `win_arm64`). That is script execution plus a portable
runtime, not a complete application foundation: no PowerShell Gazelle
extension, no Gallery/PSResourceGet lock, no env plan, and no qualified
test-runner mapping beyond `pwsh_test` script execution ship upstream.
The portable `pwsh` runtime route (7.6.5 LTS observed, 7.5.4 pinned) already
decided for PSScriptAnalyzer in
[tool acquisition](../tools/tool-acquisition.md#delivery-classes) extends
directly to the foundation.

## Sources And Ownership

One directory holds one reusable `pwsh_library` named after the directory
basename with sorted `.ps1`/`.psm1`/`.psd1` sources (at most one `.psd1`
manifest per library, enforced upstream). `*Test.ps1` and
`*.Tests.ps1` files are never library sources; handwritten `pwsh_test`
owns them. Thin `pwsh_binary` entries are never inferred; a directory
mixing a library module with an entry script fails so the owner splits it
first. All three extensions map to the single `powershell` semantic class.

## Generation Story

No PowerShell Gazelle extension ships with the ruleset or in `gazelle/`;
the published upstream docs cover only the three execution rules plus
toolchain registration. The foundation takes the explicitly scoped
alternative: handwritten wrappers only, no first-party generator. Directory
libraries, entry binaries, and tests are authored directly against
`//powershell/rules:defs.bzl`; generation consumes the Gallery lock and
never writes it. A future Gazelle extension needs a new evidence-backed
decision; it is not assumed here.

## Resolution

Every literal identity uses strict common resolution over the local index,
the Gallery `third_party/powershell/PSGallery.lock.json` scope
(`PSGallery.requirements.psd1` manifest vs lock, fail-closed), and
user-authored mappings or exact `dx_ignore_import` exceptions. The wrappers
never select versions, run `Install-Module`/`Install-PSResource`, or edit
manifests or locks. Module imports arrive via explicit-path `deps` on
wrapper libraries (`PwshInfo` `imports` for `PSModulePath` setup); ambient
module discovery is rejected.

## Evidence

Provisional. Pinned by `bazel run //tools/ci:foundation_maps` with the
[test matrix](../testing/generation.md#generate-command-and-gazelle-extensions).
Pester runner mapping is qualified seed-only via the
`powershell/tests/fixtures/pester/pins.bzl` plus hello fixture shape
(`Describe`/`It` sources plus `Invoke-Pester` entry over the pinned
Gallery lock; unpinned runner rejected); Gallery lock via
`powershell/tests/fixtures/gallery/pins.bzl` over
`third_party/powershell/PSGallery.lock.json` (consumes never writes,
fail-closed). No `Supported` claim until platform plus consumer plus
release evidence passes per the [support matrix](../product/support-matrix.md).
