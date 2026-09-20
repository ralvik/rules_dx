"""Scalafix console + semanticdb-classpath wiring decision.

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`,
`docs/product/support-matrix.md#provisional-adapter-input-notes`.
"""

# Pinned reference version (qualified seed-only under; living at head
# rejected; digests stay owned).
SCALAFIX_VERSION = "0.14.7"

# Distribution identity (managed-JVM route over the shared managed JDK plus
# the Scala Maven-lock story; digests stay owned).
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
# nothing yet, decision recorded).
SCALA_FIXTURE_HELLO = "//scala/tests/fixtures/hello:hello_test"

# Rejected: silent console parse plus IN_PLACE plus ambient wiring.
SCALAFIX_REJECTED = "silent console parse rejected: must record; IN_PLACE patching rejected; ambient classpath inference rejected; auto-supplied OrganizeImports plus RemoveUnused preset rejected"
