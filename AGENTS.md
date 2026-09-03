This repository uses Bazel.

Code rules:

- Run the repository's formatter and linter after code changes.
- Before adding a helper or abstraction, search for and reuse an existing repository library or shared module. Extend the narrowest suitable API instead of creating a parallel implementation.
- Keep domain-specific shared code near its domain. Move code into a general-purpose library only when there is a concrete cross-package use case.
- Treat warnings as errors.
- Update focused tests and documentation when behavior or user-facing workflows change.
- Do not edit files marked as generated; use the documented generator command instead.
- Keep README files short and concise; put substantial examples in the `examples/` folder.

