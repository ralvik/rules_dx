# Adopt Python example

Foreign Python tree adopted without upstream changes: an `app` package with a
same-package local edge (`envelopes` -> `handlers`), a manifest-free `solo`
directory (stdlib-only), and a conventional `tests/` directory proving the
`_test`-suffix / `test_`-prefix classification negatives (`test_login.py` and
`test_helpers.py` stay libraries). The tree arrived with no `MODULE.bazel` and
no `BUILD` files; the `*/BUILD.bazel` files are generator-owned
(`//gazelle/python:gazelle` output, see below).

```sh
bazel run //dx/cli:dx -- init //examples/adopt-python/...
bazel run //gazelle/python:gazelle -- update examples/adopt-python/app examples/adopt-python/solo examples/adopt-python/tests
bazel build //examples/adopt-python/...
bazel test //examples/adopt-python/...
```

Evidence: `dx init` writes nothing inside the tree (absent-only). Generation
emits one `python_library` per `.py`, links same-package basename imports
(`:envelopes` -> `:handlers`, tests -> siblings), drops stdlib imports
(`json`, `dataclasses`, `math`, `hashlib`) with no edges, and classifies only
`*_test.py` as `python_test`. The three tests carry user-owned runtime edges
(`dep_group = "hello"` plus `# keep` `@pypi//pytest` / `@pypi//coverage`,
entries precedent: no separate lock) because the wrapper runs pytest; those
lines survive regeneration. Build covers 15 targets; all 3 tests pass
(`handlers_test`, `pure_test`, `login_test`).

Scope notes: the root `pyproject.toml` pins the manifest shape (extras and
dependency groups) but generation does not enforce lock scope yet, so the
example reuses the hello uv graph. Python is not wired into `dx generate`
yet (that target runs the Rust extension only); regenerate with the
`//gazelle/python:gazelle` command above until the dx wiring lands.
