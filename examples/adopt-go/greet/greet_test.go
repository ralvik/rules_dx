// Internal package-level test with TestMain and a shared helper.
// Package greet tests reach the library directly (same package); the
// generated go_test owns every *_test.go via embed, never per-file targets.
package greet

import (
	"os"
	"testing"
)

func TestMain(m *testing.M) {
	os.Exit(m.Run())
}

func TestHello(t *testing.T) {
	t.Parallel()
	if got, want := Hello("world"), "hello world"; got != want {
		t.Fatalf("Hello(world) = %q, want %q", got, want)
	}
	if got, want := Platform(), "linux"; got != want {
		t.Fatalf("Platform() = %q, want %q (seed host is linux)", got, want)
	}
}

func TestShout(t *testing.T) {
	t.Parallel()
	if got, want := Shout("hi"), "HI!"; got != want {
		t.Fatalf("Shout(hi) = %q, want %q", got, want)
	}
	if got := greetHelper("ab"); got != "ab-ab" {
		t.Fatalf("greetHelper(ab) = %q, want %q", got, "ab-ab")
	}
}
