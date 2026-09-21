"""Docs adapter pins plus mappings.

Contract: `docs/documentation/doc-ir.md`.
Fixture: `docs/adapters/testdata/` via `bazel test //docs/adapters/...`.
Thin per-language runs over pinned native inputs normalizing into the
versioned IR; seed-only, no working-site claim.
"""

# Pinned native inputs, rechecked 2026-09-21 at implementation.
RUST_RUSTDOC = "nightly-2026-09-01 with rustdoc JSON format_version 30 via -Z unstable-options --output-format json"
PYTHON_GRIFFE = "griffe==2.2.0 with griffe dump --full plus JSON schema"
TYPESCRIPT_TYPEDOC = "typedoc@0.28.20 with --json --emit none plus schemaVersion"
JAVA_JDK = "JDK 25 with custom Javadoc Doclet plus owned JSON contract"
KOTLIN_DOKKA = "Kotlin 2.2.20 plus Dokka 2.2.0 with custom plugin owned JSON"
GO_XTOOLS = "Go SDK 1.26.6 plus golang.org/x/tools v0.36.0 via go/packages"
CPP_DOXYGEN = "Doxygen 1.18.0 XML generator with Clang-assisted comparison"
CSHARP_ROSLYN = ".NET SDK 10.0.201 with assembly plus /doc XML join plus owned inheritdoc resolver"
FSHARP_SERVICE = ".NET SDK 10.0.201 plus FSharp.Compiler.Service 43.9.200 with XML join"
VUE_DOCGEN = "vue-docgen-api@4.79.2 with parseMulti plus arrays-only props/events/slots/methods"
SVELTE_SVELD = "sveld@0.37.3 with compiler plus svelte2tsx comparison"
SCALA_TASTY = "Scala 3.3.6 with TASTy Inspector spike plus Scaladoc bridge comparison"
ASTROMDX_PROSE = "None; prose-only with no API surface"

# Rejected substitutes.
REJECTED_STABLE_RUSTDOC = "stable rustdoc without -Z unstable-options JSON is rejected"
REJECTED_HTML_SCRAPE = "Dokka HTML scraping is rejected; custom plugin owned JSON only"
REJECTED_CONSOLE_PARSE = "console-parse for extractor outputs is rejected"

NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #779"
OWNED_GAP = "renderer plus site plus rebuild plus link plus guide plus timing plus drift stay owned gaps under #780-#785"
