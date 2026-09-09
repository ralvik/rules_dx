# M01: Starlark Testing And Tested-Stack Metadata

## Outcome

Project-owned real-Bazel Starlark testing and generated tested-stack metadata protect subsequent APIs.

## Scope

Implement load, unit, and analysis modes, expectations, phase-aware failures, standard Bazel test
reporting, Starlark coverage evidence metadata, and the initial tested-stack manifest.
Enforce the central coverage policy for the Starlark implementation and testing capability itself.

## Contract References

- [Starlark testing decision](../decisions/0009-starlark-testing.md), [tested-stack decision](../decisions/0014-tested-platform-release-stack.md), and [testing](../testing/).
- [Mandatory coverage policy](../testing/README.md#coverage), with measurement mechanics resolved
  there (O47 resolved).

## Deliverables

- Public experimental `starlark_test` facade and project-owned `expect` API.
- Generated exact-version tested-stack and Starlark coverage evidence metadata, clearly distinguishing
  instrumented line reports from any permitted behavioral matrix fallback.
- Custom Starlark tooling implementing the resolved coverage instrumentation, including framework internals
  and equivalent reasoned-ignore support when instrumented. Only documented infeasibility permits a
  complete behavioral inventory linked to meaningful passing assertions as the fallback; retain M00's
  gate for other implementation languages.

## Work Packages

1. Implement phase-correct load, unit, analysis, expectation, and negative-test behavior.
2. Integrate ordinary Bazel logs, XML, filtering, retries, and suites.
3. Carry forward the resolved coverage instrumentation under pinned real Bazel before
   implementing a behavioral fallback, following the [central policy](../testing/README.md#coverage).
4. Generate and validate tested-stack and coverage evidence metadata in CI, reconciling inventory
   changes with implementation and validating line reports and reasoned ignores when instrumented,
   or meaningful assertion evidence rather than mappings alone for the permitted fallback.

## Milestone-Specific Evidence

- Conformance fixtures prove real Bazel phase behavior and deterministic accumulated mismatches.
- Metadata regeneration is deterministic and fails on untracked stack or coverage changes.
- Instrumented conformance rejects missing line reports, uncovered non-ignored executable lines,
  invalid ignore directives, and ignores without nearby explanatory reasons; valid reasoned ignores
  affect only their intended lines under the resolved coverage mechanics.
- If instrumentation is infeasible, link the pinned-real-Bazel investigation evidence and prove matrix
  conformance rejects missing inventory items, absent mappings, and mappings without meaningful passing
  assertions. Label this fallback as behavioral completeness, never line coverage, and report it separately.

## Out Of Scope

- Product providers, quality actions, CLI behavior, and support promotion.

## Completion Report Additions

- Record tested modes, subject types, negative phases, metadata schema, and coverage evidence gaps.
- Link central coverage-gate results, instrumentation investigation, and line-report/ignore-validation
  evidence or the documented infeasibility and inventory-to-assertion evidence permitting the explicitly
  labeled behavioral fallback. Identify any unmet resolved coverage requirements.
