# Documentation Instructions

- Put cross-domain docs in `docs/`. Use the existing domain folders.
- Edit in place. One doc per fact; link, don't copy. Link-don't-copy applies to code comments and docstrings too: `.bzl` headers carry one-line purpose plus owning-contract link, function docs keep only non-obvious invariants, Rust keeps only why-not-obvious notes with owning issue/ADR link.
- In-code docs rule (enforceable, audited under #710): `.bzl` headers are one-line purpose plus `Contract:` link; function docs keep only non-obvious invariants with a `See:` link. Rust keeps `//!`/`///` only for why-not-obvious (protocol, safety/ordering, error-mapping) with a `See:` or `Owning contract:` link; crate roots link the owning `docs/` contract once and never restate file lists. Python/Starlark/shell keep docstrings only where they add signal beyond the def name. `LCOV_EXCL_*` reasons stay short (`policy: docs/testing/README.md#coverage`); full rationale lives once in `docs/testing/README.md#coverage`. No bare `Issue #` in non-test source: `rg "Issue #|issue #" --glob '*.rs' --glob '*.bzl' cli/ quality/ tools/` must show only lines containing `See:` or `Owning contract:` or inside test pins (`pins.bzl`, `*_tests*.rs`/`*_tests*.bzl`, `tests/`, `testdata/`). Per-area keep/shrink/delete list: `docs/documentation/in-code-docs-audit.md`.
- One H1 per file. Short paragraphs, relative links.
- Label accepted vs provisional vs open clearly.
- Keep repo facts separate from future plans.
- Update related docs together.
- On conflicts: pick the smallest provisional fix, flag for review. Don't present provisional as accepted.
- No verified claims without evidence per `testing/README.md`.
- No milestone system: do not create `Mxx` specs or an `Oxx` register. Planned work is listed in `roadmap.md`; docs describe as-built behavior only.
- Prototypes stay as marked scratch outside product paths.
- After edits, report changed files, design changes, and open items.
- User-prose style: no `issue #` numbers or closed-issue genealogy in user-facing prose (`README.md`, `docs/README.md`, landing pages, `examples/README.md`); link the roadmap once instead. The [support matrix](product/support-matrix.md) is the single status source; other docs link to it instead of copying lifecycle claims.
