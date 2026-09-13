# M23 Completion Report: Additional Managed Foundations And Runtime Parity

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
the integrations are adapter-tested first-party implementation under test, not a
product support claim. O31 stays open for its owner; this report supplies the
milestone-specific evidence (upstream stack, supported targets, provider
mappings, lock members, shared runtimes, route proofs, platforms, limitations).
No quality adapter claims any M23 class yet; adapter qualification moves to M24
under O32, recorded below as deferred scope rather than a silent drop.

## WP1: Upstream Review, Python/Node Private Graphs

Upstream stack (pinned in `MODULE.bazel`): `rules_java 9.7.0` with the
remote-JDK toolchain configs (hermetic `--java_runtime_version` selection open
under O31), `rules_kotlin 2.4.10` over the same JVM toolchain and Maven-lock
story (kotlinc acquisition and worker behavior open), `rules_scala 7.3.0` with
Scala `2.13.18` on the managed route frozen by the M22 O30 decision
(Coursier-fetched toolchains share Java's Maven lock; semanticdb plus
classpath wiring for Scalafix open), and `rules_dotnet 0.22.1` (plus
`bazel_lib 3.7.0`) covering both admitted .NET languages with SDK `10.0.201`
and default target framework `net10.0` (per-platform SDK acquisition and the
xUnit/NUnit runner selection open). Decision: reuse the pinned upstream
rulesets through thin wrappers rather than replacement compiler rules;
rationale: the narrowest API that preserves upstream providers while adding
the one new `QualitySourcesInfo` fact quality aspects gate on.

Python and Node private-graph members (WP1 "complete" clause) already landed
under their owning milestones and are consumed unchanged here: the private
wheel-only Python tool graph (`quality/tools/python`: `pyproject.toml` plus
`uv.lock`, `pypi` hub) from M15 and the private pure-JavaScript tool graph
(`quality/tools/javascript`: `package.json` plus `pnpm-lock.yaml`,
`npm_tools` hub) from M17, each sharing its managed runtime cohort. No M23
change to either lock was needed; a foundation deferral removes no baseline
tool.

## WP2: JVM, .NET, And PowerShell Cohorts With Shared Runtimes

`java/rules/defs.bzl` (`dx_java_library`, `dx_java_binary`, `dx_java_test`),
`kotlin/rules/defs.bzl` (`dx_kotlin_library`, `dx_kotlin_binary`,
`dx_kotlin_test`), `scala/rules/defs.bzl` (`dx_scala_library`,
`dx_scala_binary`, `dx_scala_test`), `csharp/rules/defs.bzl`
(`dx_csharp_library`, `dx_csharp_binary`, `dx_csharp_test`), and
`fsharp/rules/defs.bzl` (`dx_fsharp_library`, `dx_fsharp_binary`,
`dx_fsharp_test`): one private `<name>_dx_upstream` target plus one public
forwarding target each, preserving the upstream providers (`JavaInfo`,
`DefaultInfo`, `InstrumentedFilesInfo`; Scala keeps `ScalaInfo` readable from
the private target, never duplicated) unchanged and adding
`QualitySourcesInfo(direct_sources = {<class>: <direct sources>})`.
Kotlin and Scala own same-compilation-unit mixed `.java` sources alongside
(classified `java`). `main_class`/`test_class` always pass through with no
invented default; out-of-scope upstream surface (`java_import`,
`scala_import`, compiler plugins, `scala_junit_test`, extra target kinds)
fails closed to direct upstream loads. The five `hello` fixtures prove the
boundaries (library/binary/test over the wrappers,
`bazel test //java/hello:hello_test //kotlin/hello:hello_test
//scala/hello:hello_test //csharp/hello:hello_test
//fsharp/hello:hello_test` pass).

Locks: one shared Maven lock for the JVM cohort
(`third_party/jvm/maven_install.json` via `rules_jvm_external 7.1`,
`fail_if_repin_required`, `known_contributing_modules = ["protobuf"]`; seed
member junit `4.13.2` only, JUnit 5/6 runner selection open) and one shared
Paket lock for the .NET cohort (`third_party/dotnet/paket.dependencies` plus
`paket.lock` via `paket2bazel` into the `paket.main` hub; seed member
FSharp.Core only, xUnit/NUnit runner selection open).

Generation and env: `gazelle/java`, `gazelle/kotlin`, `gazelle/scala`,
`gazelle/csharp`, `gazelle/fsharp` mirror the `gazelle/vue`
strict-resolution shape (source-only goldens, self-deps dropped, stdlib
skipped, errors abort before emission, stale sweep, `dx_ignore_import` with
inheritance and stale-fail-closed); `java/env`, `kotlin/env`, `scala/env`,
`csharp/env`, `fsharp/env` (`plan.bzl` over the preserved upstream info plus
`QualitySourcesInfo`, deterministic JSON plus `DxSubjectInfo`) each carry a
pinned plan test. Classification: `java`, `kotlin`, `scala`, `csharp`,
`fsharp` families in `quality/adapters.bzl`, each keeping its existing
ownership with no adapter claim yet (classification only, no supported
claim). O31 qualifies exact adapters later.

Focused route proofs (O31 order: Python, Node, JVM including the
Scala/Scalafix managed route, .NET, PowerShell, Ruby), recorded in the
[support matrix](../product/support-matrix.md#provisional-default-quality-tools)
and [tool acquisition](../tools/tool-acquisition.md#first-release-tool-routing),
all frozen 2026-09-13:

- JVM: google-java-format, Checkstyle, PMD, SpotBugs, ktfmt, and ktlint take
  the complete-upstream-artifact plus shared-JDK route (no Maven-module
  reconstruction, no consumer installer/solver/compiler).
- Scala managed: Scalafmt and Scalafix take the managed JVM route over the
  same shared JDK and Maven-lock story; Scalafix semantic rules additionally
  need semanticdb plus classpath wiring.
- .NET: CSharpier and Fantomas take the exact-upstream-package plus
  shared-.NET-runtime route (declared-DLL execution, no `dotnet tool
  install`).
- PowerShell: PSScriptAnalyzer takes the exact-module plus
  portable-`pwsh`-runtime route (explicit-path import; application
  foundation stays deferred beyond v1, tool cohort only).
- Ruby: RuboCop and StandardRB take the release-assembled Ruby closure route
  (the exceptional bundle within the O46-approved packaging-effort boundary;
  application foundation stays deferred beyond v1, tool cohort only).
  This closes the O31 order.

## WP3: Ruby Bundle And The Scala Managed Route

The Ruby exceptional bundle is route-frozen (contents, lock inputs, and
adapter qualification pending, owned by M24 under O32); no bundle bytes are
assembled here, matching the M22 treatment of Buf/Qt (route frozen, assets
pending). The Scala managed route selected by M22 is implemented here: the
`dx_scala_*` wrappers, hello fixture, env plan, Gazelle adapter, shared
Maven-lock reuse (no new lock members), and classification above, consuming
the M22 O30 decision. Coursier toolchain acquisition and per-target
`scala_version` selection stay open under O31.

## WP4: Per-Member Suites, Deferred Adapters

Deferred scope (evidence-backed, owned by M24 under O32, not silent): no M23
quality adapter (google-java-format, Checkstyle, PMD, SpotBugs, Error Prone,
ktfmt, ktlint, detekt, Scalafmt, Scalafix, CSharpier, Roslyn analyzers,
Fantomas, FSharpLint, PSScriptAnalyzer, RuboCop, StandardRB) is implemented
here; exact artifact versions, digests, rule sets, config mappings,
runtime-compatibility bounds, and common-runner exceptions close in M24
parity closure. Per-member external-consumer, no-install, runtime-sharing,
platform, and performance suites are unproven (same gap class as
M00/M12/M14/M16/M17/M22).

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (623 targets).
- `bazel test //...`: 142/142 pass, including `//java/hello:hello_test`,
  `//kotlin/hello:hello_test`, `//scala/hello:hello_test`,
  `//csharp/hello:hello_test`, `//fsharp/hello:hello_test`, the five
  `*/env:env_plan_tests`, and the ten `//gazelle/{java,kotlin,scala,csharp,
  fsharp}:{*_test,generation_test}` suites.
- `bazel coverage //gazelle/scala:scala_test //gazelle/csharp:csharp_test
  //gazelle/fsharp:fsharp_test`: pass.
- `bazel run //dx:generate_check`: clean.
- Coverage inventory registers the `gazelle/java`, `gazelle/kotlin`,
  `gazelle/scala`, `gazelle/csharp`, and `gazelle/fsharp` paths;
  `//tools/coverage:coverage_test` passes in the full run.

## Changed Components

- `MODULE.bazel` (`rules_java 9.7.0`, `rules_kotlin 2.4.10`, `rules_scala
  7.3.0` with Scala `2.13.18`, `rules_dotnet 0.22.1` with SDK `10.0.201`,
  `bazel_lib 3.7.0`, `rules_jvm_external 7.1` with the shared Maven lock,
  the shared Paket lock extension).
- `java/rules/`, `kotlin/rules/`, `scala/rules/`, `csharp/rules/`,
  `fsharp/rules/` (`defs.bzl`, `BUILD.bazel`); the five `hello/` fixtures;
  the five `env/` plans (`plan.bzl`, `plan_tests.bzl`, `BUILD.bazel`).
- `gazelle/java/`, `gazelle/kotlin/`, `gazelle/scala/`, `gazelle/csharp/`,
  `gazelle/fsharp/` (adapters, parsers, naming, tests, goldens).
- `third_party/jvm/maven_install.json`, `third_party/dotnet/`
  (`paket.dependencies`, `paket.lock`, `deps/`).
- `quality/adapters.bzl` (`java`/`kotlin`/`scala`/`csharp`/`fsharp` plus
  tool-only `ruby`/`powershell` classification).
- `tools/coverage/inventory.txt` (new Gazelle paths).
- `docs/product/support-matrix.md` (O31 cohort tables, route proofs, runner
  and lock tables), `docs/open-decisions.md` (O31), `docs/tools/
  tool-acquisition.md` (O31 route proofs), this report.

## Open Items

- O31 stays open for its owner: per-platform JDK/SDK acquisition, kotlinc
  worker behavior, Coursier toolchain details, per-target Scala version
  selection, semanticdb/classpath wiring for Scalafix, Paket/NuGet lock
  wiring, test-runner selections (JUnit 5/6, xUnit/NUnit, ScalaTest
  wiring), Ruby bundle contents and lock inputs, PowerShell console-parse
  versus library-API binding, and any adapter that cannot use the common
  runner.
- M23 quality adapters deferred to M24 under O32 (see WP4); M24 reconciles
  the full baseline.
- Non-Linux hosts, remote execution, and clean external-consumer
  evidence are unproven (same gap class as M00/M12/M14/M16/M17/M22).
- No supported claim; out-of-scope upstream surface stays unresolved and
  fails closed.
