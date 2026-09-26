# Scope Defaults

No scope means `//...` for most commands. Use `--here` for the current dir tree. `--here` never combines with explicit scopes.

`dx run`, `dx deploy`, `dx why`, `dx bump`, and `dx completion` need their args. `dx bazel` forwards raw args. `dx deploy` takes exactly one label.
