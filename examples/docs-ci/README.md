# Docs CI Example

```sh
cp caller.yml .github/workflows/docs.yml
```

Starter caller for the reusable docs workflow. Pin to a reviewed commit.

```python
bazel_dep(name = "rules_dx", version = "0.0.0")
```

Enable Pages with source GitHub Actions before the first publishing run.
