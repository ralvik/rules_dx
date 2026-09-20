# Documentation

Accepted v1 direction; execution open (see Contracts). No `Supported`
claim; no working site claimed.

Accepted design: native language tooling extracts API semantics; thin per-language adapters
normalize into one versioned [documentation IR](doc-ir.md); one [site
build](site.md) renders prose plus API reference with a single theme,
navigation, URL model, and search. SCIP code intelligence is out of v1; it is not needed for docs.

## Pipeline

```text
native machine output (rustdoc JSON, TypeDoc JSON, Griffe, Doclet, ...)
        ↓
thin language adapter (one package per language, pinned inputs)
        ↓
versioned documentation IR (generated Bazel outputs, cached by Bazel)
        ↓
site build (Bazel-cached actions: extract → aggregate → render)
        ↓
mdBook-compatible prose + generated API pages + one search index
```

Generated IR stays in Bazel outputs, not beside source files or in Git. The `.proto`
schema and `documentation_ir` codec tests are checked in; adapter golden
fixtures remain open. Ordinary API changes require no IR snapshot update.

## Contracts

- [Documentation IR](doc-ir.md): common symbol model, language extensions,
  validation, fixtures, and drift policy. Authoritative for IR facts.
- [Site build](site.md): Bazel cache-friendly action design, determinism
  rules, laziness, generated-artifact lifecycle, and the decided mdBook renderer.
  Authoritative for build facts.
- Command surface removed;
  reintroduction open, tracked under issue #421 (live successor to closed #310)
  (see the [`dx docs` stub](../cli/commands/docs.md)).
  [ADR 0006](../decisions/0006-cli-command-surface.md) records build versus
  validation-only check; exact mappings are tracked under
  issue #421. Docs pipeline gaps stay open under issue #421 (per-language adapter runs
  with pins and mappings, renderer and site execution, byte-identical rebuild proof,
  link and reference completeness, guide-step CI wiring, first-hour timing proof, and
  per-release pin-bump plus drift process; no working site claimed).

Rust uses pinned nightly `rustdoc --output-format json`; Scala needs a
Scaladoc/TASTy proof spike; Astro/MDX are prose-only with no API surface.
Accepted scope covers thirteen adapter scopes. Per-language input pins,
mappings, adapter runs, renderer/site-build execution, and every other
#421 item above remain open; no adapter execution exists today.
