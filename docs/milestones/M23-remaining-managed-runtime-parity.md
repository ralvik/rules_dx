# M23: Additional Managed Foundations And Runtime Parity

## Outcome

Admitted complete managed-runtime foundations and remaining quality integrations are adapter-tested;
managed quality tools require no consumer installer or tool lock.

## Scope

Complete private Python and Node tools/plugins, complete JVM distributions, exact .NET and PowerShell tool packages,
Ruby assembled tool closures, and the Scala managed route selected by M22 when applicable.
Own the additional managed-runtime foundation cohort under first-release admission; admit/defer/exclude
outcomes live in [ADR 0019](../decisions/0019-first-release-additional-foundations.md).
Java, Kotlin, C#, and F# foundations
are admitted to v1 scope by [ADR 0019](../decisions/0019-first-release-additional-foundations.md). Ruby and PowerShell application
foundations are deferred beyond v1 by [ADR 0019](../decisions/0019-first-release-additional-foundations.md); only their quality-tool cohorts
stay in this milestone. Deliver admitted complete low-cost
upstream foundations after O46 freezes language-specific contracts, provider/generation mappings,
effort evidence, and work packages. Quality-tool runtime acquisition is not an application dependency
or foundation implementation.

## Contract References

- [Tools](../tools/), [quality](../quality/), [tested platforms](../decisions/0014-tested-platform-release-stack.md), [testing](../testing/), and open decision [O31](../open-decisions.md).
- [V1 admission](../product/scope.md#first-release-admission), [support inventory](../product/support-matrix.md), [generation](../generation/), [environments](../environments/), and [O46](../open-decisions.md).

## Deliverables

- Managed-runtime adapters, private locks, shared runtime metadata, complete distributions, and assembled bundle artifacts.
- Reproducible maintainer-only lock/bundle generation with licenses, SBOM, and provenance where publication applies.
- Admitted foundation wrappers, authoritative dependencies, generation adapters, and target
  environment/IDE plans with language-specific test and coverage integration.

## Work Packages

1. Review and prove each upstream foundation/provider route, including any language packaging,
   source build, or focused patch permitted by the admission policy, before integrating admitted
   foundations and dependent tools. Complete Python and Node private-graph members and curated
   plugins; a foundation deferral does not remove a baseline tool.
2. Prove and add JVM, .NET, and PowerShell cohorts with shared runtimes.
3. Build and consume the Ruby exceptional bundle within the O46-approved packaging-effort boundary
   and, when M22 selected it, implement the Scala managed route.
4. Run per-member external-consumer, no-install, runtime-sharing, platform, and performance suites.

## Milestone-Specific Evidence

- Managed quality-tool consumers perform no solve, install, compilation, lifecycle script, or ambient runtime discovery.
- Every private-lock member and bundle has focused capability evidence; representative tools do not stand in for a cohort.
- Every admitted foundation has per-capability generation, dependency, test, environment, platform,
  and clean external-consumer evidence rather than borrowing its quality adapter's claim.

## Out Of Scope

- Native/toolchain cohorts, parity closure, replacement language stacks, and support promotion.

## Completion Report Additions

- Provide the managed-runtime adapter-test matrix, lock members, shared runtimes, bundle reproducibility,
  licenses, per-route effort/review evidence, and consumption of the M22 Scala route decision.
  Distinguish recorded additional-foundation deferrals and their evidence from required-core
  blockers and other unresolved required cells; account separately for retained quality-tool obligations.
