"""Structured quality defaults pins.

Contract: `docs/product/support-matrix.md#provisional-default-quality-tools`,
`docs/quality/native-configuration.md#authority`.
"""

# Pinned upstream versions (initial artifact research observations, now
# qualified seed-only under; living at head rejected).
BUF_VERSION = "1.72.0"
QMLFORMAT_QT_OBSERVED = "Qt 6.11.2"
QMLLINT_QT_OBSERVED = "Qt 6.11.1"

# Upstream distribution identities (checksummed native/self-contained
# artifact route for buf as self-contained per-platform binaries with
# published checksums, no target compiler context, execution-platform lazy;
# authoritative-toolchain route for qmlformat/qmllint from the qualified Qt
# distribution, Qt-last ordering decided; exact Qt distribution identity,
# licensing, and platform artifact qualification plus all digests stay owned
# , never reconstructed from modules).
BUF_ARTIFACT = "self-contained per-platform binaries with published checksums"
QMLFORMAT_ARTIFACT = "qualified Qt distribution tool targets"
QMLLINT_ARTIFACT = "qualified Qt distribution tool targets"
QT_COUPLING = "Authoritative Qt distribution: qmlformat plus qmllint version follows the qualified Qt distribution pin, no separate acquisition"

# Rejected lines (never pinned here).
REJECTED_BUF_HEAD = "newer than 1.72.0 observed, not pinned; living at head rejected"

# Native-configuration sole policy (no hidden presets). Without an applicable
# checked-in native config the pinned tool uses its upstream built-in
# behavioral defaults; with a config the tool interprets it natively.
NATIVE_CONFIG_POLICY = "native-configuration sole policy: no hidden presets"

# Qualified rule-set resolutions (upstream built-in defaults, not rules_dx presets).
BUF_POLICY = "STANDARD is the upstream built-in default lint set, not a rules_dx preset; without checked-in buf.yaml the pinned buf uses STANDARD, with buf.yaml it interprets natively"
QMLFORMAT_POLICY = "upstream built-in defaults without checked-in .qmlformat.ini, native interpretation with .qmlformat.ini; no auto-supplied ini preset"
QMLLINT_POLICY = "upstream built-in defaults without checked-in .qmllint.ini, native interpretation with .qmllint.ini; no auto-supplied ini preset"
BEYOND_DEFAULT_REJECTED = "beyond-default switches rejected: COMMENTS plus UNARY_RPC opt-in maxima, --enable=all-style opt-in maxima"

# Live proof shape (no protobuf/qml hello bazel test exists: Protocol Buffer
# plus QML foundations are not admitted as build/test targets, so the fixture
# pair plus grep contract checks plus bazel build of the fixture is the live
# proof; quality adapters claim nothing yet).
STRUCTURED_PROOF = "bazel build //quality/tests/fixtures/structured_quality:corpus_starlark"

# Rejected: hidden presets plus unpinned versions.
STRUCTURED_REJECTED = "hidden presets rejected: no auto-supplied buf.yaml preset, no auto-supplied qml ini preset; unpinned versions rejected: no floating version or head"
