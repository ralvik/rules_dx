package demo

import "testing"

func TestHelper(t *testing.T) {
	t.Parallel()
	if "hello world" != "hello world" {
		t.Fatal("unreachable")
	}
}
