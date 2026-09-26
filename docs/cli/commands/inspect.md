# Inspect Wrappers

```text
dx owners <scope>...
dx deps <scope>...
dx why <file> <label>
```

`dx owners` lists owning targets. `dx deps` lists dependencies. `dx why` shows one path from a file owner to a label. `--configured` uses `cquery` instead of `query`.
