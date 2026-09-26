package demo_test

import "testing"

func TestPlaceholder(t *testing.T) {
	t.Parallel()
	if "hello world" != "hello world" {
		t.Fatal("unreachable")
	}
}
