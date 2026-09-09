# Documentation

Unified polyglot documentation is v1 scope.

Native language tooling extracts API semantics; thin per-language adapters
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
schema and golden test fixtures are checked in; ordinary API changes require no IR snapshot update.

## Contracts

- [Documentation IR](doc-ir.md): common symbol model, language extensions,
  validation, fixtures, and drift policy. Authoritative for IR facts.
- [Site build](site.md): Bazel cache-friendly action design, determinism
  rules, laziness, generated-artifact lifecycle, and the decided mdBook renderer.
  Authoritative for build facts.
- [`dx docs`](../cli/commands/docs.md): candidate build/check/serve command
  surface. [ADR 0006](../decisions/0006-cli-command-surface.md) records build versus
  validation-only check; exact mappings remain provisional under [O54](../open-decisions.md).

Rust uses pinned nightly `rustdoc --output-format json`; Scala needs a
Scaladoc/TASTy proof spike; Astro/MDX are prose-only with no API surface.
v1 ships all thirteen adapters. Per-language input pins and mappings stay
provisional until O54 qualification.
