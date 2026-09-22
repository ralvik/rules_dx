# Documentation IR

The documentation intermediate representation (IR) is the versioned contract
between per-language extractors, aggregation, and the site renderer.
Generated shards stay in Bazel outputs and are never committed. Open work is
tracked in GitHub issues.

The checked-in [`docs/ir/doc_ir.proto`](../ir/doc_ir.proto) is the schema
source of truth. The [`documentation_ir` codec crate](../ir/ir/src/lib.rs)
implements validate/encode/decode with roundtrip, rejection-parity,
ordering, and forward-compat tests. `--check` validates without rendering;
normal build validates then renders.

## Versioning

Every IR document carries `schema_major`/`schema_minor`. Minor versions are
additive-only; breaking changes increment the major version with a recorded
migration. Unknown extension data is preserved verbatim so extractors can
advance without a core schema redesign. Generated IR action outputs use
binary Protobuf with deterministic serialization, and human-readable review
uses textproto against the same schema. Exact field and enum numbers and
reserved ranges live in the `.proto` file.

IR shards are generated and cached by Bazel like other action outputs. They
are not committed, written beside source files, or copied into a
source-tree projection. Adapter golden fixtures are pinned native inputs,
not snapshots that consumers must refresh when their APIs change.

Determinism requires reproducible bytes for the same pinned producer and
declared inputs, not canonical bytes across different serializers or
upgrades.

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

Symbol IDs are `language:package:qualified_name`, with overloads
disambiguated by normalized parameter-type list. Source paths are
workspace-relative. Only public API enters the IR; visibility filtering
follows each language's native semantics, not a universal heuristic.

## Language Extensions

The renderer handles the common model uniformly and uses language-specific
components where semantics genuinely differ (Rust trait implementations,
C++ template metadata, TypeScript type metadata, JVM/.NET member kinds).
Extension payloads live under `extensions` and are never silently dropped.

## Machine Inputs

Accepted scope covers the adapter families below, including the nightly
rustdoc route. Each adapter runs over pinned native inputs with golden
fixtures per scope. Recheck every pin and schema version at implementation;
research observations are not pins.

| Family | Pinned input | Note |
| --- | --- | --- |
| Rust | rustdoc JSON output | Narrow currency exception; no ambient or stable-output fallback |
| Python | Griffe dump/load API, published JSON schema | Documentation-oriented; map kind, members, docstring, expressions |
| TypeScript/JavaScript | TypeDoc JSON output | Versioned with a narrow compatibility window; map reflections/signatures |
| Java | Custom Javadoc Doclet emitting owned JSON | Doclet API is built for this; owned JSON contract |
| Kotlin | Dokka custom plugin owned JSON | Custom plugin over HTML scraping; lockstep Dokka-to-Kotlin pin |
| Go | `go/packages` plus doc-comment AST over `go list -json` | Thin project-owned extractor |
| C/C++ | Doxygen XML first candidate; libclang declarations/types plus comments as comparison | Compare built-in and Clang-assisted Doxygen |
| C# | Assembly metadata plus compiler `/doc` XML | XML comments alone are not the API model |
| F# | FSharp.Compiler.Service metadata plus XML docs | Same join as C#; compiler-service API may change |
| Vue | `vue-docgen-api` JSON | Adapter owns the contract |
| Svelte | `sveld` JSON versus first-party compiler extraction | Maintenance and native-semantics qualification |
| Scala | TASTy Inspector spike versus Scaladoc internal-model bridge | Missing TASTy fails closed |
| Astro/MDX | None; prose-only | No library API surface to extract |

XML or JSON comment dumps alone do not satisfy an adapter where the language
separates comments from symbols (notably C#/F#): the adapter joins metadata
with documentation.

### Extractor Research

Read-only upstream research narrows candidates, not implementation or support
claims. Qualify exact-pin mapping, member-count reconciliation, golden files,
and pre-render validation per adapter. Prefer owned JSON contracts over HTML
scraping, thin project-owned extractors over generic fallbacks, and
compiler-derived inventories over comment-only dumps. A live upstream manual
is not evidence for a selected release asset.

The inventory above has thirteen rows but only twelve extraction-family rows;
splitting JavaScript/TypeScript and C/C++ yields fourteen API identities,
with Astro/MDX as additional prose-only identities.

### Adapter Reconciliation

Each extraction-family row maps to one adapter scope; one adapter may cover
two API identities where the input pipeline is shared.

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
| — | Astro/MDX | Astro/MDX prose identities (no extractor) |

## Validation And Fixtures

Each adapter ships golden fixtures covering representative and difficult
constructs for its language (generics, overloads, re-exports, inheritance,
deprecation), not just minimal APIs. Checks verify that IR output decodes
against the checked-in schema, a symbol-count inventory detects silent
omissions, symbol IDs and cross-links are stable across reruns, selected
pages match native-tool output, upgrades run old and new extractors against
the same fixtures with explicit review of semantic changes, and
same-producer rebuilds are byte-identical while cross-version compatibility
compares decoded semantics.

The dangerous failure is a successful build that silently drops or
misclassifies APIs; the fixture suite exists to make that failure loud.

## Drift Policy

`rules_dx` pins every extractor and toolchain input, and users never upgrade
those pins themselves. Upgrades arrive only through `rules_dx` releases: each
release bumps pins, runs drift testing (contract suite, golden fixtures,
determinism evidence) against the new inputs, and ships only when everything
is green plus explicitly reviewed. An upstream format or toolchain change can
therefore turn release preparation red, but never a user's build.

Pins live in three places and bump together: the adapter pins file, the
adapter source constants, and the machine-inputs table above. Drift detection
reuses the codec gates: roundtrip, rejection parity, symbol and extension
ordering, and minor forward-compat. Ordinary API changes require no IR
snapshot update: generated shards stay in Bazel outputs and adapter golden
fixtures stay pinned native inputs, not snapshots.

## Related issues

Tracking lives in GitHub issues.
