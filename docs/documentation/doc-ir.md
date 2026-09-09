# Documentation IR

Provisional implementation status: the pipeline direction is accepted v1
scope, but the schema, validation mechanism,
per-language inputs, and drift policy remain unqualified under
[O54](../open-decisions.md). Do not implement adapters against this prose
until those qualifications land.

## Versioning

Every IR document carries `doc_ir_version`. Minor versions are
additive-only; breaking changes increment the major version with a recorded
migration. Unknown extension data is preserved verbatim so extractors can
advance without a core schema redesign. Per the repository-wide internal
representation direction, a checked-in `.proto`
file is the schema source of truth: generated IR action outputs use
binary Protobuf with deterministic serialization, and human-readable review
uses textproto against the same schema. Exact field and enum numbers,
reserved ranges, and compatibility fixtures freeze under O54, following the
[Quality Result Protocol](../quality/quality-result-protocol.md) precedent.

IR shards are generated and cached by Bazel like other action outputs. They are not
committed, written beside source files, or copied into a source-tree projection.
The checked-in schema and reviewed golden test fixtures define the format and expected
behavior; they are not snapshots that consumers must refresh when their APIs change.

Determinism requires reproducible bytes for the same pinned producer and declared inputs, not
canonical bytes across different serializers or upgrades. Follow the
[Protobuf serialization constraint](../quality/quality-result-protocol.md#transport): pin the
producer, schema, and runtime; test semantic compatibility separately from same-producer byte equality.

## Common Model

Shown as textproto for review; on the wire and in action outputs this is
binary Protobuf. Symbol IDs are stable across rebuilds:

```text
doc_ir_version: 1
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
with overloads disambiguated by normalized parameter-type list. The exact
disambiguation scheme per language freezes under O54. Source paths are
workspace-relative. Only public API enters the IR; visibility filtering
follows each language's native semantics, not a universal heuristic.

## Language Extensions

The renderer handles the common model uniformly and uses language-specific
components where semantics genuinely differ (Rust trait implementations,
C++ template metadata, TypeScript type metadata, JVM/.NET member kinds).
Extension payloads live under `extensions` and are never silently dropped.

## Machine Inputs

v1 ships adapters for all thirteen languages below, including the pinned
nightly rustdoc route and the Scala proof spike.
Exact inputs, pins, and adapter mappings freeze under O54.

| Family | Provisional input | Note |
| --- | --- | --- |
| Rust | Pinned nightly `rustdoc -Z unstable-options --output-format json` | Narrow [currency exception](../decisions/0008-dependency-currency.md#rustdoc-extraction-exception); no ambient or stable-output fallback |
| Python | Griffe model (`griffe dump --full`, `load` API, published JSON schema) | Documentation-oriented; map `kind`/`members[]`/`docstring`/expressions; recheck exact `griffe==2.x` pin at implementation |
| TypeScript/JavaScript | TypeDoc JSON (`--json`, `--emit none`, `JSONOutput.*`, `schemaVersion`) | Versioned with a narrow compatibility window; map reflections/signatures; recheck exact pin at implementation |
| Java | Custom Javadoc Doclet emitting IR (`Doclet{init, getName, getSupportedOptions, getSupportedSourceVersion, run}`, `javadoc -doclet`) | Doclet API is built for this; owned JSON contract remains implementation work |
| Kotlin | Dokka model/plugin output (CLI JSON configuration, `dokka-base` plus plugins) | No stable JSON model output established; custom plugin likely; lockstep Dokka-to-Kotlin coupling needs qualification |
| Go | `go/packages` plus doc-comment AST over `go list -json` (`packages.Load`, `doc.New`, `comment.Parser`) | Thin project-owned extractor; request `NeedTypes` with `NeedSyntax` together |
| C/C++ | Doxygen XML first candidate; libclang declarations/types plus comments as comparison | Compare built-in and Clang-assisted Doxygen; `CXComment` alone is not an API model |
| C# | Assembly metadata plus compiler `/doc` XML (`<member name="T:|M:|P:|F:|E:|N:">`, `GetDocumentationCommentXml`, own `inheritdoc`/`include` resolver) | XML comments alone are not the API model |
| F# | Compiler-service metadata plus XML docs (`FSharpChecker`, `AssemblySignature.Entities`, XML lookup by signature) | Same join qualification as C#; compiler-service API may change |
| Vue | `vue-docgen-api` JSON (`parseMulti`, arrays-only `props`/`events`/`slots`/`methods`) | Object-stable without a frozen JSON schema; adapter owns the contract |
| Svelte | `sveld` JSON versus first-party compiler/`svelte2tsx`/TypeScript extraction | Maintenance and native-semantics qualification required |
| Scala | Scala 3 TASTy Inspector versus pinned Scaladoc internal model | No general Scaladoc JSON export established; proof spike required |
| Astro/MDX | None; prose-only | No library API surface to extract |

XML or JSON comment dumps alone do not satisfy an adapter where the
language separates comments from symbols (notably C#/F#): the adapter must
join metadata with documentation, and O54 qualifies that join.

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
  native snapshot-check commands are not `dx docs --check`.
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
Reconcile the thirteen-adapter claim with an explicit adapter-to-input table against the grouped
machine-input inventory above. No language is removed, dummy prose adapter added, or package split mandated by this count.

## Validation And Fixtures

Each adapter ships golden fixtures covering representative and difficult
constructs for its language (generics, overloads, re-exports, inheritance,
deprecation), not just minimal APIs. Required checks:

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

rules_dx pins every extractor and toolchain
input, and users never upgrade those pins themselves. Upgrades arrive only
through rules_dx releases: each release bumps pins, runs drift testing
(contract suite, golden fixtures, determinism evidence) against the new
inputs, and ships only when everything is green. An upstream format or
toolchain change can therefore turn rules_dx CI red during release
preparation, but never a user's build — users stay on pinned, checksummed
inputs and receive working adapters with the release. The exact per-release
pin-bump and drift-test process freezes under O54.
