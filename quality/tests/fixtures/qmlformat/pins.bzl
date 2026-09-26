"""qmlformat check plus fix wiring (QML format).

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`.
"""

# Pinned reference version (follows the qualified Qt distribution pin;
# Qt 6.11.2 observed; living at head rejected; digests plus distribution
# identity plus licensing plus platform artifacts stay owned under
QMLFORMAT_QT_OBSERVED = "Qt 6.11.2"

# Distribution identity (authoritative-toolchain route from the qualified
# Qt distribution tool targets, Qt-last ordering decided; no separate
QMLFORMAT_ARTIFACT = "qualified Qt distribution tool targets"
QMLFORMAT_RUNTIME = "authoritative Qt distribution; no separate acquisition"

# Invocation shapes (whole-file rewrite with check/diff mode).
QMLFORMAT_CHECK = "qmlformat --check (exit 0 clean, exit 1 with unformatted paths when dirty)"
QMLFORMAT_FIX = "qmlformat -i in-place (re-read on exit 0, keep input otherwise)"
QMLFORMAT_CONFIG_POLICY = "upstream built-in defaults without .qmlformat.ini, native interpretation with .qmlformat.ini; no auto-supplied preset"

# foundations are not admitted as build/test targets, so the fixture
# pair plus matrix cells are the live proof).
STRUCTURED_FIXTURE_QMLFORMAT = "//quality/tests/fixtures/qmlformat:corpus_starlark"

# Rejected: ambient config discovery, auto-supplied preset.
QMLFORMAT_REJECTED = "ambient .qmlformat.ini discovery rejected; auto-supplied preset rejected"
