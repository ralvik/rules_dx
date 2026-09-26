# `dx migrate`

```text
dx migrate --from <version> --to <version> [scope...]
```

Rewrites breaking changes. Both versions are required. Target must be newer than source. No manifests exist yet, so live runs fail closed. `--dry-run` only prints the plan.
