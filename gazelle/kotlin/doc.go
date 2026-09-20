// Package kotlin implements the first-party Kotlin Gazelle extension (ADR 0019).
// It is implemented directly against Gazelle over the pinned rules_kotlin
// package boundary. There is no shared helper layer: helpers are extracted
// only after a later managed foundation proves concrete reuse.
package kotlin
