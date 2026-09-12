// Package typescript implements the first-party TypeScript Gazelle extension
// (M16, ADR 0013).
//
// Narrow M16 slice: one-source dx_ts_project ownership, strict imports,
// merge, and stale cleanup. Test sources (`*_test.ts/tsx/mts/cts`) generate
// dx_ts_project targets in this slice; dedicated dx_ts_test execution
// wrappers arrive in a later slice (see typescript/rules/defs.bzl). Thin
// binaries for entries are deferred until the execution wrapper lands;
// entries currently own only their library. No shared helper layer with the
// JavaScript extension: each extension owns its facts.
package typescript
