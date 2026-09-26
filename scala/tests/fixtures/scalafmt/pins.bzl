"""Scalafmt check plus fix wiring."""

SCALAFMT_VERSION = "3.11.4"

SCALAFMT_ARTIFACT = "compatible JVM artifact over the shared managed JDK"

SCALAFMT_CHECK = "scalafmt --check plus --config when hinted (exit 0 clean, exit 1 with unified diff when dirty)"
SCALAFMT_FIX = "scalafmt in-place rewrite (re-read on exit 0, keep input otherwise)"
SCALAFMT_CONFIG_POLICY = "checked-in .scalafmt.conf required to change policy; upstream built-in defaults without config, native interpretation with config"

SCALA_FIXTURE_HELLO = "//scala/tests/fixtures/hello:hello_test"

SCALAFMT_REJECTED = "IN_PLACE patching outside sandbox rejected; ambient .scalafmt.conf discovery rejected; auto-supplied preset rejected"
