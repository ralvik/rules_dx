// Package typescript implements the first-party TypeScript Gazelle extension
// (ADR 0013).
//
// Narrow slice: one-source typescript_project ownership, strict imports,
// merge, and stale cleanup. Test sources (`*_test.ts/tsx/mts/cts`) generate
// typescript_test targets over the tsc-compiled output (see
// typescript/rules/defs.bzl). A
// recognized `main.ts/tsx/mts/cts` entry owns one reusable project library
// plus one thin `javascript_binary` over the compiled output (execution
// reuses the JavaScript wrappers; there is no `typescript_binary`). Only
// the exact `main` basename is an entry; other layouts are an explicit
// wont-fix. No shared helper layer with the JavaScript
// extension: each extension owns its facts.
package typescript
