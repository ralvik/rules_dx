# Code style

Accepted. One table for indent policy plus the shellcheck record.
Formatters own enforcement; this doc only records the choice.

## Indent

| Scope | Tool | Indent | Source |
| --- | --- | --- | --- |
| JavaScript, TypeScript, JSON | Biome | tab | `biome.json` |
| Shell | shfmt | 2 spaces (`-i 2 -ci`) | `.shellcheckrc` |
| Rust | rustfmt | defaults, edition 2021 | `rustfmt.toml` |
| Python | Ruff | defaults | `ruff.toml` |
| Starlark | Buildifier | defaults | pinned action |

Biome tab versus shfmt spaces is intentional: each scope follows its
tool default, never a repo-wide indent. See the
[local workflows](local-workflows.md) for the check commands.

## Shellcheck

Accepted. The nine global disables in `.shellcheckrc` are exhaustive:
each carries its reason inline next to the disable. Narrowing further
would re-litigate settled harness constraints (runfiles bootstrap,
intentional single-quote patterns, `$var` style). See `.shellcheckrc`.
