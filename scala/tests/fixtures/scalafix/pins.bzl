"""Scalafix console + semanticdb-classpath wiring decision."""

SCALAFIX_VERSION = "0.14.7"

SCALAFIX_ARTIFACT = "semantic-rule artifacts over the shared managed JDK"

SCALAFIX_DIAGNOSTICS = "wire via ScalafixMainCallback library API; console-parse rejected"
SCALAFIX_CONSOLE_PARSE_REJECTED = "console-parse rejected: rewritable rules emit patch-only diff with no rule attribution (console_rewrite.txt), interleaved patch plus diagnostic sections, missing diagnostics; fail-closed parsing unprovable"
SCALAFIX_WIRE_REQUIRED = "custom Java entrypoint binding scalafix.interfaces.ScalafixMainCallback over semantic-rule artifacts for per-file rule IDs plus positions plus patches"

SCALAFIX_SEMANTICDB_WIRING = "target-coupled: --classpath plus --sourceroot plus --semanticdb-targetroots from authoritative scala_* target context; syntactic-only runs need no target context; missing semantic context fails closed"
SCALAFIX_SYNTACTIC_NOTE = "syntactic rules run on source without compilation; semantic rules require target sources compiled with semanticdb-scalac plus full classpath"
SCALAFIX_FIX_FLOW = "sandbox-apply-and-diff with declared per-target outputs as unified patches; IN_PLACE mutation rejected under sandboxing"

SCALAFIX_CONFIG_POLICY = "checked-in .scalafix.conf required for OrganizeImports plus RemoveUnused; upstream built-in defaults without config, native interpretation with config"

SCALA_FIXTURE_HELLO = "//scala/tests/fixtures/hello:hello_test"

SCALAFIX_REJECTED = "silent console parse rejected: must record; IN_PLACE patching rejected; ambient classpath inference rejected; auto-supplied OrganizeImports plus RemoveUnused preset rejected"
