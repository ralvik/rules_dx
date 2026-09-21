# Documentation IR

Implementation status: accepted v1 direction with adapter runs plus site execution plus
rebuild proof plus link/reference completeness delivered; remaining execution open (#783-#785, successors to closed #581,
live successor to closed #421). Accepted: the `dx_docs` planning library
(command dispatch removed per [ADR 0020](../decisions/0020-remove-dx-docs-placeholder.md);
reintroduction tracked under #786) —
`--check` validates without rendering, normal build validates then renders.
Accepted: the checked-in [`docs/ir/doc_ir.proto`](../ir/doc_ir.proto)
(`dx.documentation.v1`, `schema_major: 1`) and the
[`documentation_ir` codec crate](../ir/ir/src/lib.rs)
(`//docs/ir/ir:documentation_ir`: validate/encode/decode with
roundtrip, rejection-parity, extension- and symbol-ordering, and minor-forward-compat tests).
Delivered under #779 (successor to closed #581; see [Documentation](README.md#contracts)):
per-language adapter runs with pins and mappings (`//docs/adapters:docs_adapters`
over pinned native inputs with golden fixtures per scope). Same-producer byte-identical rebuild proof
for the fixture-scale site is delivered seed-only under #781. Link/reference completeness at the
pre-render boundary is delivered seed-only under #782. Open under #785 plus #783-#784
(successors to closed #581; see [Documentation](README.md#contracts) for the full list):
symbol-count inventory and native-output comparison fixtures (delivered under #779),
and per-release pin-bump plus drift process. No working docs support is claimed until
qualified site execution lands.

## Versioning

Every IR document carries `schema_major`/`schema_minor`. Minor versions are
additive-only; breaking changes increment the major version with a recorded
migration. Unknown extension data is preserved verbatim so extractors can
advance without a core schema redesign. Per the repository-wide internal
representation direction, the checked-in
[`docs/ir/doc_ir.proto`](../ir/doc_ir.proto)
is the schema source of truth: generated IR action outputs use
binary Protobuf with deterministic serialization, and human-readable review
uses textproto against the same schema. Exact field and enum numbers and
reserved ranges live in that file, validated by the
`documentation_ir` codec crate; compatibility fixtures are tracked under
#785 (successor to closed #581), following the
[Quality Result Protocol](../quality/quality-result-protocol.md) precedent.

IR shards are generated and cached by Bazel like other action outputs. They are not
committed, written beside source files, or copied into a source-tree projection.
The checked-in schema and `documentation_ir` codec tests define the format and expected
behavior; adapter golden fixtures delivered under #779 (successor to closed #581) in
`docs/adapters/testdata/` with `//docs/adapters:docs_adapters` runs. They are not snapshots that consumers must refresh when their APIs change.

Determinism requires reproducible bytes for the same pinned producer and declared inputs, not
canonical bytes across different serializers or upgrades. Follow the
[Protobuf serialization constraint](../quality/quality-result-protocol.md#transport): pin the
producer, schema, and runtime; test semantic compatibility separately from same-producer byte equality.

## Common Model

Shown as textproto for review; on the wire and in action outputs this is
binary Protobuf. Symbol IDs are stable across rebuilds:

```text
schema_major: 1
schema_minor: 0
language: "python"
package: "mylib"
symbols {
  id: "python:mylib:AccountService.create"
  kind: METHOD
  signature_text: "def create(self, input: AccountInput) -> Account"
  doc_markdown: "Creates a new account."
  params { name: "input" type: "AccountInput" doc: "Validated input." }
  returns { type: "Account" doc: "The created account." }
  examples: "```python\nsvc.create(data)\n```"
  source { file: "src/account.py" line: 42 }
  visibility: PUBLIC
  relations { member_of: "python:mylib:AccountService" }
}
```

Symbol IDs are stable across rebuilds: `language:package:qualified_name`,
with overloads disambiguated by normalized parameter-type list. The
disambiguation runs in `//docs/adapters:docs_adapters` (delivered under #779).
Source paths are
workspace-relative. Only public API enters the IR; visibility filtering
follows each language's native semantics, not a universal heuristic.

## Language Extensions

The renderer handles the common model uniformly and uses language-specific
components where semantics genuinely differ (Rust trait implementations,
C++ template metadata, TypeScript type metadata, JVM/.NET member kinds).
Extension payloads live under `extensions` and are never silently dropped.

## Machine Inputs

Accepted scope covers thirteen adapter scopes below, including the pinned
nightly rustdoc route and the delivered Scala spike. Adapter runs delivered
under #779 with exact pins in `docs/adapters/pins.bzl` and mappings in
`//docs/adapters:docs_adapters`.

| Family | Pinned input (#779) | Note |
| --- | --- | --- |
| Rust | nightly-2026-09-01 `rustdoc -Z unstable-options --output-format json` (format_version 30) | Narrow [currency exception](../decisions/0008-dependency-currency.md#rustdoc-extraction-exception); no ambient or stable-output fallback |
| Python | Griffe 2.2.0 (`griffe dump --full`, `load` API, published JSON schema) | Documentation-oriented; map `kind`/`members[]`/`docstring`/expressions |
| TypeScript/JavaScript | TypeDoc 0.28.20 (`--json`, `--emit none`, `JSONOutput.*`, `schemaVersion`) | Versioned with a narrow compatibility window; map reflections/signatures |
| Java | JDK 25 custom Javadoc Doclet emitting owned JSON (`Doclet{init, getName, getSupportedOptions, getSupportedSourceVersion, run}`, `javadoc -doclet`) | Doclet API is built for this; owned JSON contract delivered |
| Kotlin | Kotlin 2.2.20 plus Dokka 2.2.0 custom plugin owned JSON (CLI JSON configuration) | Custom plugin over HTML scraping; lockstep Dokka-to-Kotlin pin delivered |
| Go | Go 1.26.6 plus `golang.org/x/tools` v0.36.0 `go/packages` plus doc-comment AST over `go list -json` (`packages.Load`, `doc.New`, `comment.Parser`) | Thin project-owned extractor; request `NeedTypes` with `NeedSyntax` together |
| C/C++ | Doxygen 1.18.0 XML first candidate; libclang declarations/types plus comments as comparison | Compare built-in and Clang-assisted Doxygen; `CXComment` alone is not an API model |
| C# | .NET 10.0.201 assembly metadata plus compiler `/doc` XML (`<member name="T:|M:|P:|F:|E:|N:">`, `GetDocumentationCommentXml`, own `inheritdoc`/`include` resolver) | XML comments alone are not the API model |
| F# | .NET 10.0.201 plus FSharp.Compiler.Service 43.9.200 metadata plus XML docs (`FSharpChecker`, `AssemblySignature.Entities`, XML lookup by signature) | Same join as C#; compiler-service API may change |
| Vue | `vue-docgen-api` 4.79.2 JSON (`parseMulti`, arrays-only `props`/`events`/`slots`/`methods`) | Object-stable without a frozen JSON schema; adapter owns the contract |
| Svelte | `sveld` 0.37.3 JSON versus first-party compiler/`svelte2tsx`/TypeScript extraction | Maintenance and native-semantics qualification delivered |
| Scala | Scala 3.3.6 TASTy Inspector spike versus pinned Scaladoc internal-model bridge | Proof spike delivered; missing TASTy fails closed |
| Astro/MDX | None; prose-only | No library API surface to extract; confirmation delivered |

XML or JSON comment dumps alone do not satisfy an adapter where the
language separates comments from symbols (notably C#/F#): the adapter joins
metadata with documentation; that join runs in `//docs/adapters:docs_adapters`
(delivered under #779).

### Extractor Research

Read-only upstream research narrows candidates, not implementation or support claims.
Recheck every pin and schema version at implementation; research observations are not pins.

- **Python:** use [Griffe serialization](https://mkdocstrings.github.io/griffe/guide/users/serializing/)
  and the [`griffe dump` CLI](https://mkdocstrings.github.io/griffe/reference/cli/) (`dump <pkg> [-o {package}.json]
  [-f/--full]`, multi-package `{pkgName: moduleObj}`) or the
  [programmatic API](https://mkdocstrings.github.io/griffe/reference/api/) (`load`/`visit`/`inspect`,
  `Object.as_json(full=)`/`from_json`, `JSONEncoder`). Default dumps are round-trippable subsets; `full`
  preserves all fields for non-Griffe consumers. Qualify exact-pin mapping, member-count reconciliation,
  golden `--full` files, and pre-render JSON-schema validation.
- **TypeScript/JavaScript:** use [`typedoc --json <out.json>`](https://typedoc.org/documents/Options.Output.html)
  with `--emit none` for convert-and-validate without JS emit, consuming the versioned
  [`JSONOutput`](https://typedoc.org/api/modules/JSONOutput.html) model (`Serializer.toObject`,
  `DeclarationReflection`/`SignatureReflection`). Expect breaking renames across minors and record
  `schemaVersion` at extraction. Qualify exact-pin mapping, reflection-count reconciliation against the
  `tsc` export set, golden JSON, and pre-render shape validation.
- **Java:** implement a custom doclet against the
  [JEP 221 Doclet API](https://openjdk.org/groups/compiler/using-new-doclet.html)
  (`Doclet{init, getName, getSupportedOptions, getSupportedSourceVersion, run}`,
  `javadoc -doclet <fqcn>`, `getSpecifiedElements`/`getDocTrees()` composed with Language Model and
  Compiler Tree APIs). There is no blessed JSON output; the owned JSON contract is implementation work.
  Qualify the JDK pin, specified-element count reconciliation, per-pin goldens, and pre-render validation.
- **Kotlin:** the [Dokka CLI](https://kotlinlang.org/docs/dokka-cli.html) accepts a JSON configuration with
  `pluginsClasspath`/`pluginsConfiguration`, but no stable JSON model output was established; `dokka-base`
  ships HTML only and the [plugin model](https://kotlinlang.org/docs/dokka-plugins.html) expects custom
  extensions. Prefer a custom plugin emitting owned JSON over HTML scraping if pursued, with exact
  Dokka-to-Kotlin pinning, symbol-count reconciliation, per-bump goldens, and pre-render validation.
- **Go:** implement a thin extractor with [`go/packages`](https://pkg.go.dev/golang.org/x/tools/go/packages)
  (`packages.Load` with `NeedName|NeedFiles|NeedCompiledGoFiles|NeedImports|NeedTypes|NeedSyntax|NeedTypesInfo`),
  default `go list -json -e -compiled -test` driver, [`go/doc`](https://pkg.go.dev/go/doc) plus
  [`go/doc/comment`](https://pkg.go.dev/go/doc/comment) AST, `go/parser`, and build-constraint matching.
  Qualify the `x/tools` pseudo-version plus Go toolchain pin, package/file-count reconciliation, hermetic
  `GOPACKAGESDRIVER`-unset runs, golden JSON, and pre-render validation.
- **C#:** join assembly/signature enumeration with compiler `/doc` sidecar XML
  (`<member name="T:|M:|P:|F:|E:|N:">`, `cref`/generic encodings) per the
  [XML documentation](https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/xmldoc/) model.
  XML is not reflection metadata; `ISymbol.GetDocumentationCommentXml()` can return empty for metadata
  references and the compiler does not resolve `inheritdoc`/`include`. Qualify the Roslyn pin, owned
  resolver policy, documented-versus-reflected count reconciliation with unresolved-ID reporting, per-pin
  goldens, and `cref` validation.
- **F#:** join `FSharp.Compiler.Service` signatures (`FSharpChecker`, `AssemblySignature.Entities`,
  XML lookup by signature) with XML docs per the
  [F# XML documentation](https://learn.microsoft.com/en-us/dotnet/fsharp/language-reference/xml-documentation)
  model. The service API may change. Qualify the exact service pin with the same join, count, golden,
  and validation requirements as C#.
- **Vue:** consume [`vue-docgen-api`](https://vue-styleguidist.github.io/docs/Docgen.html)
  (`parse`/`parseSource`/`parseMulti`) as a JS object (`description`/`tags`/`exportName`/`props[]`/
  `events[]`/`slots[]`/`methods[]`; props/events/methods/slots are arrays). Standardize on `parseMulti`
  for multi-export files and treat versioned JSON stability as adapter-owned. Qualify the npm pin, SFC
  block-scan count reconciliation, golden `ComponentDoc` JSON, and required-array validation.
- **C/C++:** prefer the documented [Doxygen XML interface](https://www.doxygen.nl/manual/customize.html#xmlgenerator)
  as the first proof candidate. Compare its built-in parser and
  [Clang-assisted mode](https://www.doxygen.nl/manual/config.html#cfg_clang_assisted_parsing) against
  a reviewed compiler-derived inventory; the latter requires a libclang-enabled Doxygen artifact.
  Direct [libclang](https://clang.llvm.org/docs/LibClang.html) is the alternative, but its C API does
  not expose the full C++ AST. Traverse declarations/types as well as comments. Qualify target flags,
  macros, generated headers, overloads, templates/concepts, public-header static functions, and
  undocumented public symbols. A live upstream manual is not evidence for a selected release asset.
- **Svelte:** retain `sveld` as a credible packaged candidate; its
  [recent releases](https://github.com/carbon-design-system/sveld/releases) show active maintenance.
  Its [documented behavior](https://github.com/carbon-design-system/sveld#readme) still requires proof
  for imported prop types, custom-parser fidelity, observable partial failures, and runtime bounds.
  Compare against first-party [svelte2tsx](https://github.com/sveltejs/language-tools/tree/master/packages/svelte2tsx)
  plus compiler and TypeScript semantics, not a generic TS-only fallback. Cover runes and legacy
  syntax, snippets versus slots, callback props versus dispatched events, bindings, exports,
  preprocessors, and source locations. Disable or confine tool caches to declared Bazel outputs;
  native snapshot-check commands are not the planned `dx docs --check`.
- **Scala:** spike the documented [Scala 3 TASTy Inspector](https://docs.scala-lang.org/scala3/reference/metaprogramming/tasty-inspect.html)
  and a compiler-pinned Scaladoc internal-model bridge separately. TASTy supplies semantic trees,
  not a ready-made documentation model; prove retained comments, dependency classpaths, visibility,
  inheritance, synthetic filtering, and links. The reviewed
  [Scaladoc settings](https://docs.scala-lang.org/scala3/guides/scaladoc/settings.html) do not establish
  a general JSON export; a type-search database is not complete API documentation. Missing or
  incompatible TASTy and empty inputs must not produce a false successful inventory. Scala 2, if
  required by the qualified version scope, needs its own native route rather than assumed TASTy support.

The table has thirteen rows but only twelve extraction-family rows; splitting JavaScript/TypeScript
and C/C++ yields fourteen API identities, with Astro/MDX additional prose-only identities.
The provisional reconciliation below maps adapter scopes to the grouped
machine-input inventory above. No language is removed, dummy prose adapter added, or package split mandated by this count.

### Adapter Reconciliation (delivered under #779)

Each extraction-family row maps to one adapter scope; one adapter may cover
two API identities where the input pipeline is shared. Adapter packaging
(one `//docs/adapters:docs_adapters` crate covering all scopes) is delivered,
not mandated here.

| Adapter scope | Machine-input row(s) | API identities |
| --- | --- | --- |
| Rust | Rust | Rust |
| Python | Python | Python |
| TypeScript | TypeScript/JavaScript | TypeScript, JavaScript |
| Java | Java | Java |
| Kotlin | Kotlin | Kotlin |
| Go | Go | Go |
| C++ | C/C++ | C, C++ |
| C# | C# | C# |
| F# | F# | F# |
| Vue | Vue | Vue |
| Svelte | Svelte | Svelte |
| Scala | Scala | Scala |
| — | Astro/MDX | Astro/MDX prose identities (no extractor; authored markdown flows straight to the renderer) |

Twelve extraction adapters plus the prose-only path reconcile the
thirteen-row inventory with the fourteen API identities: the TypeScript and
C++ scopes each cover two identities over one shared input pipeline.

## Validation And Fixtures

Delivered under #779 (successor to closed #581): each adapter ships golden fixtures
in `docs/adapters/testdata/` covering representative and difficult
constructs for its language (generics, overloads, re-exports, inheritance,
deprecation), not just minimal APIs. Delivered checks (`//docs/adapters:docs_adapters`
plus `bazel run //tools/ci:docs_pipeline_qualification`):

- IR output decodes against the checked-in `.proto` schema; protocol
  failures fail the extraction action rather than emitting partial shards.
- A symbol-count inventory test detects silent public-API omissions.
- Symbol IDs and cross-links are stable across fixture reruns.
- Selected generated pages are compared against native-tool output.
- Upgrades run old and new extractor versions against the same fixtures;
  semantic fixture changes require explicit review.
- Same-producer rebuilds are byte-identical; cross-version compatibility compares decoded
  semantics rather than requiring identical Protobuf encodings.

The dangerous failure is a successful build that silently drops or
misclassifies APIs; the fixture suite exists to make that failure loud.

## Drift Policy

Accepted policy; adapter pins delivered under #779 (twelve extraction plus
prose-only; see `docs/adapters/pins.bzl`). rules_dx pins every extractor and toolchain
input, and users never upgrade those pins themselves. Upgrades arrive only
through rules_dx releases: each release bumps pins, runs drift testing
(contract suite, golden fixtures, determinism evidence) against the new
inputs, and ships only when everything is green. An upstream format or
toolchain change can therefore turn rules_dx CI red during release
preparation, but never a user's build — users stay on pinned, checksummed
inputs and receive working adapters with the release. The exact per-release
pin-bump and drift-test process stays open under
#785 (successor to closed #581).

## Related issues

Tracking lives in the [roadmap](../roadmap.md). IR and adapters: #779 (delivered) plus #780 (site delivered) plus #781 (rebuild delivered) plus #782 (link completeness delivered) plus #783-#785 (successors to closed #581). Reintroduction: #786.
