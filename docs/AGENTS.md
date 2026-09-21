# Documentation Instructions

- Put cross-domain docs in `docs/`. Use the existing domain folders.
- Edit in place. One doc per fact; link, don't copy. Link-don't-copy applies to code comments and docstrings too: `.bzl` headers carry one-line purpose plus owning-contract link, function docs keep only non-obvious invariants, Rust keeps only why-not-obvious notes with owning issue/ADR link.
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
