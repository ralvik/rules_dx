"""File-family quality defaults pins (issue #489).

Contract: `docs/product/support-matrix.md#provisional-default-quality-tools`,
`docs/quality/native-configuration.md#authority`.

Qualifies the provisional file-family format plus lint defaults against the
native-configuration contract with no hidden presets. Versions below are the
pinned upstream releases from the initial artifact research; rule-sets are
upstream built-in defaults unless an applicable checked-in native config
supplies policy. Adapters add only transport/hermetic settings. Exact
artifact digests plus adapter mappings stay owned under issue #420.
`protobuf`/`qml` stay owned by issue #488/#419 and are cross-linked here,
never double-claimed. Applicability is provider-class owned by the registry,
never inferred from a file suffix.
"""

# Pinned upstream versions (initial artifact research observations, now
# qualified seed-only under #489; living at head rejected).
CUE_VERSION = "v0.17.1"
JSONNETFMT_VERSION = "v0.22.0"
PKL_VERSION = "0.32.1"
TERRAFORM_VERSION = "v1.16.1"
DJLINT_VERSION = "v1.45.0"
STYLELINT_VERSION = "17.14.1"
PRETTIER_VERSION = "3.9.6"
PRETTIER_PLUGIN_SQL_VERSION = "0.15.1"
YAMLFMT_VERSION = "v0.21.0"
YAMLLINT_VERSION = "1.38.0"
KEEP_SORTED_VERSION = "v0.10.0"

# Upstream distribution identities (frozen delivery-class routes; digests
# stay owned under issue #420, never reconstructed or ambient-resolved).
CUE_ARTIFACT = "standalone checksummed release artifact; cue fmt whole-file rewrite with check/diff mode"
JSONNETFMT_ARTIFACT = "standalone checksummed release artifact; go-jsonnet whole-file rewrite with check/diff mode"
PKL_ARTIFACT = "standalone checksummed release artifact with published checksums"
TERRAFORM_ARTIFACT = "standalone checksummed release artifact; terraform fmt whole-file rewrite with check/diff mode, fmt ships with the CLI"
DJLINT_ARTIFACT = "private wheel-only Python graph member over the shared managed Python runtime, no sdist fallback"
STYLELINT_ARTIFACT = "private pure-JavaScript graph member over the shared managed Node runtime"
PRETTIER_PLUGIN_GHERKIN_ARTIFACT = "private pure-JavaScript graph member (named plugin closure with Prettier over the shared managed Node runtime)"
PRETTIER_PLUGIN_SQL_ARTIFACT = "private pure-JavaScript graph member (named plugin closure with Prettier over the shared managed Node runtime)"
PRETTIER_PLUGIN_XML_ARTIFACT = "private pure-JavaScript graph member (named plugin closure with Prettier over the shared managed Node runtime)"
MODFMT_ARTIFACT = "standalone checksummed release-artifact candidate for go.mod formatting; whole-file rewrite with check/diff mode"
YAMLFMT_ARTIFACT = "standalone checksummed release artifact with cosign-signed checksums; -lint check mode"
YAMLLINT_ARTIFACT = "private wheel-only Python graph member over the shared managed Python runtime, no sdist fallback"
KEEP_SORTED_ARTIFACT = "standalone checksummed release artifact; check-only with sandbox-apply-and-diff"

# Rejected and pending lines (never pinned here).
REJECTED_JSONNET_CPP = "C++ jsonnet v0.21.0 observed, not pinned; go-jsonnet v0.22.0 is the pinned line; living at head rejected"
REJECTED_MODFMT_PENDING = "modfmt upstream identity pending under issue #420, not pinned here; maintainer must establish exact upstream plus byte identity before adapter qualification"
REJECTED_GHERKIN_XML_VERSION = "prettier-plugin-gherkin plus prettier-plugin-xml observed without a stable version line, not separately pinned; they ride Prettier 3.9.6 in the private graph, recheck latest stable at implementation"
REJECTED_HEAD = "living at head rejected; unpinned versions rejected"

# Native-configuration sole policy (no hidden presets). Without an applicable
# checked-in native config the pinned tool uses its upstream built-in
# behavioral defaults; with a config the tool interprets it natively.
NATIVE_CONFIG_POLICY = "native-configuration sole policy: no hidden presets"

# Qualified rule-set resolutions (upstream built-in defaults, not rules_dx presets).
CUE_POLICY = "cue fmt is whole-file rewrite with check/diff mode; no rule-set selection, upstream built-in defaults without config, native interpretation with config"
JSONNETFMT_POLICY = "jsonnetfmt is whole-file rewrite with check/diff mode; no rule-set selection, upstream built-in defaults"
PKL_POLICY = "pkl uses upstream built-in defaults without config, native interpretation with config; no auto-supplied preset"
TERRAFORM_POLICY = "terraform fmt is whole-file rewrite with check/diff mode; upstream built-in defaults, no auto-supplied preset"
DJLINT_POLICY = "djlint uses upstream built-in defaults without config, native interpretation with config; --lint versus --reformat are upstream modes, no auto-supplied preset"
STYLELINT_POLICY = "stylelint uses upstream built-in defaults without config, native interpretation with config; no auto-supplied config preset; --formatter json is transport"
PRETTIER_PLUGIN_POLICY = "prettier-plugin closures are whole-file rewrite with check/diff mode over Prettier 3.9.6; no auto-supplied plugin preset"
MODFMT_POLICY = "modfmt is whole-file rewrite with check/diff mode; upstream identity plus rule-set pending under issue #420, no hidden preset here"
YAMLFMT_POLICY = "yamlfmt -lint is the upstream built-in check mode; config discovery is native interpretation, no auto-supplied preset"
YAMLLINT_POLICY = "default ruleset is the upstream built-in default ruleset, not a rules_dx preset"
KEEP_SORTED_POLICY = "keep-sorted is check-only with sandbox-apply-and-diff; no rule-set selection"
SUFFIX_POLICY = "suffix inference rejected: registry owns applicability, never inferred from a file suffix"
BEYOND_DEFAULT_REJECTED = "beyond-default switches rejected: auto presets, --enable=all-style opt-in maxima, all-rules maxima"

# Structured cross-link (never double-claimed here).
STRUCTURED_CROSSLINK = "protobuf plus qml stay owned by issue #488 plus issue #419, cross-linked here never double-claimed"

# Live proof shape (no file-family hello bazel test exists for the deferred
# families: cue/jsonnet/pkl/css/html_template/gherkin/sql/xml/go_module/
# terraform/yaml/text foundations are feasibility or N/A, so the fixture pair
# plus grep contract checks plus bazel build of the fixture is the live proof;
# quality adapters claim nothing yet under issue #420).
FILE_FAMILY_PROOF = "bazel build //quality/tests/fixtures/file_family_quality:corpus_starlark"

# Rejected: hidden presets plus unpinned versions plus suffix inference.
FILE_FAMILY_REJECTED = "hidden presets rejected: no auto-supplied config, no preset ruleset/enablement, no suffix-inferred applicability; unpinned versions rejected: no floating version or head"
