"""Scalafix console + semanticdb-classpath wiring decision (issue #490).

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`,
`docs/product/support-matrix.md#provisional-adapter-input-notes`.

Decides the open Scalafix adapter-input risk with fixture evidence, recorded
explicitly here and in the owning docs, never silently dropped. Silent console
parsing is rejected per the issue alternatives.

Upstream facts (observations, not pins; recheck latest stable at
implementation):
- Scalafix CLI has no machine-readable output upstream (open upstream issue
  scalacenter/scalafix#2459; report-API request scalacenter/scalafix#951).
  Console mixes interleaved patch plus diagnostic sections.
- Lint-only rules emit a parseable line
  (`path:line:col: severity: [RuleId] message`, see `console_lint.txt`
  modeled on `DisableSyntax.var`).
- Rewritable rules emit only a unified diff with no rule attribution (see
  `console_rewrite.txt` modeled on `ProcedureSyntax`); the rule that
  produced the rewrite cannot be inferred from the console. A console parser
  would therefore miss or misattribute findings and can never prove complete,
  fail-closed diagnostic parsing.
- The upstream-recommended integration point is the Java library API
  `scalafix.interfaces.ScalafixMainCallback` (diagnostics plus patches per
  file), not the console. A custom Java entrypoint over the semantic-rule
  artifacts is the wire route.
- Semantic rules need semanticdb plus classpath wiring: target sources must
  be compiled with the semanticdb-scalac plugin, plus full `--classpath`
  plus `--sourceroot` plus `--semanticdb-targetroots` (dependencies need not
  carry semanticdb). Syntactic rules run without compilation. CLI reference:
  `scalafix --help` (`--classpath`, `--sourceroot`,
  `--semanticdb-targetroots`, `--check`, `--rules`, `--config`).
- Source rewriting sits uneasily with immutable action outputs: `IN_PLACE`
  patching breaks under sandboxing.

Decision (adapter-only, no `scala` claim yet; cohort stays owned by #417):
- Console-parse REJECTED for diagnostics. No fail-closed text parser is
  approved; unrecognized, truncated, patch-only, or ambiguously attributed
  console output must fail the action, never become findings or an empty
  successful result. Requalification of a console grammar on every tool
  update is not sufficient here because the gap is lossiness (missing rule
  attribution), not grammar drift.
- Wire REQUIRED: structured diagnostics plus patches via a custom Java
  entrypoint binding `ScalafixMainCallback` over the semantic-rule artifacts
  on the shared managed JDK plus the Scala Maven-lock story
  (`maven_install.json` plus `fail_if_repin_required`). The entrypoint
  reports per-file rule IDs plus positions plus patches; the runner
  normalizes them to the result contract.
- Target-coupled semanticdb plus classpath wiring REQUIRED for semantic
  rules: the adapter resolves `--classpath` plus `--sourceroot` plus
  `--semanticdb-targetroots` from the authoritative `scala_*` target context
  (never ambient, never inferred from the filesystem). Syntactic-only runs
  need no target context. Without required semantic context the adapter
  fails closed; no silent check-only fallback that would miss semantic
  findings.
- Fix flow is sandbox-apply-and-diff with declared per-target outputs,
  rendered as unified patches (never `IN_PLACE` mutation of immutable
  inputs). `--check` unified diff is the CLI check shape, never the
  diagnostic source.
- Native config is sole policy: checked-in `.scalafix.conf` required for
  OrganizeImports plus RemoveUnused (see `example.scalafix.conf`); without
  config the pinned tool uses upstream built-in defaults, with a config it
  interprets natively; adapters add only transport/hermetic settings.
"""

# Pinned reference version (qualified seed-only under #486; living at head
# rejected; digests stay owned under issue #417).
SCALAFIX_VERSION = "0.14.7"

# Distribution identity (managed-JVM route over the shared managed JDK plus
# the Scala Maven-lock story; digests stay owned under issue #417).
SCALAFIX_ARTIFACT = "semantic-rule artifacts over the shared managed JDK"

# Explicit parse-vs-wire record (silent console parse rejected).
SCALAFIX_DIAGNOSTICS = "wire via ScalafixMainCallback library API; console-parse rejected"
SCALAFIX_CONSOLE_PARSE_REJECTED = "console-parse rejected: rewritable rules emit patch-only diff with no rule attribution (console_rewrite.txt), interleaved patch plus diagnostic sections, missing diagnostics; fail-closed parsing unprovable"
SCALAFIX_WIRE_REQUIRED = "custom Java entrypoint binding scalafix.interfaces.ScalafixMainCallback over semantic-rule artifacts for per-file rule IDs plus positions plus patches"

# Explicit semanticdb plus classpath wiring record.
SCALAFIX_SEMANTICDB_WIRING = "target-coupled: --classpath plus --sourceroot plus --semanticdb-targetroots from authoritative scala_* target context; syntactic-only runs need no target context; missing semantic context fails closed"
SCALAFIX_SYNTACTIC_NOTE = "syntactic rules run on source without compilation; semantic rules require target sources compiled with semanticdb-scalac plus full classpath"
SCALAFIX_FIX_FLOW = "sandbox-apply-and-diff with declared per-target outputs as unified patches; IN_PLACE mutation rejected under sandboxing"

# Native-config sole policy (no hidden preset; adapters transport-only).
SCALAFIX_CONFIG_POLICY = "checked-in .scalafix.conf required for OrganizeImports plus RemoveUnused; upstream built-in defaults without config, native interpretation with config"

# Live proof labels (foundation consumers stay green; quality adapters claim
# nothing yet under issue #417, decision recorded under issue #490).
SCALA_FIXTURE_HELLO = "//scala/tests/fixtures/hello:hello_test"

# Rejected: silent console parse plus IN_PLACE plus ambient wiring.
SCALAFIX_REJECTED = "silent console parse rejected: must record; IN_PLACE patching rejected; ambient classpath inference rejected; auto-supplied OrganizeImports plus RemoveUnused preset rejected"
