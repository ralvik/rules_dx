# Consumer CI example

```sh
cp caller.yml .github/workflows/ci.yml
```

Starter caller for the reusable consumer workflow. Pin to a reviewed commit.

```python
bazel_dep(name = "rules_dx", version = "0.0.0")
```

Set approval policy, branch protection, and least-privilege permissions manually.
