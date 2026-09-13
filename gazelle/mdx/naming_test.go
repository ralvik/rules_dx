package mdx

import (
	"strings"
	"testing"
)

func TestNormalize(t *testing.T) {
	cases := []struct{ in, want string }{
		{"hello", "hello"},
		{"Hello", "Hello"},
		{"_private", "private"},
		{"trailing_", "trailing"},
		{"my-mod", "my_mod"},
		{"my--mod", "my_mod"},
		{"my mod", "my_mod"},
		{"a.b+c", "a_b_c"},
		{"UPPER123", "UPPER123"},
		{"123", "123"},
		{"déjà", "d_j"},
	}
	for _, tc := range cases {
		if got, err := Normalize(tc.in); err != nil || got != tc.want {
			t.Errorf("Normalize(%q) = %q, %v; want %q", tc.in, got, err, tc.want)
		}
	}
}

func TestNormalizeEmptyFails(t *testing.T) {
	for _, in := range []string{"", "---", "___", "…"} {
		if got, err := Normalize(in); err == nil {
			t.Errorf("Normalize(%q) = %q, want error", in, got)
		}
	}
}

func TestTargetName(t *testing.T) {
	cases := []struct {
		name string
		want string
	}{
		{"demo.mdx", "demo"},
		{"pkg/my-mod.mdx", "my_mod"},
		{"Hello.mdx", "Hello"},
		{"plain", "plain"},
	}
	for _, tc := range cases {
		if got, err := TargetName(tc.name); err != nil || got != tc.want {
			t.Errorf("TargetName(%q) = %q, %v; want %q", tc.name, got, err, tc.want)
		}
	}
	if _, err := TargetName("---.mdx"); err == nil {
		t.Error("TargetName(---.mdx) succeeded, want error")
	}
}

func TestModuleName(t *testing.T) {
	cases := []struct {
		name string
		want string
	}{
		{"demo.mdx", "demo"},
		{"pkg/my-mod.mdx", "my-mod"},
		{"Hello.mdx", "Hello"},
		{"archive.tar.mdx", "archive.tar"},
		{"plain", "plain"},
	}
	for _, tc := range cases {
		if got := ModuleName(tc.name); got != tc.want {
			t.Errorf("ModuleName(%q) = %q, want %q", tc.name, got, tc.want)
		}
	}
}

func TestCheckCollisions(t *testing.T) {
	if err := CheckCollisions(nil); err != nil {
		t.Errorf("empty collisions = %v", err)
	}
	single := []Claimant{{Name: "a", Source: "a.mdx"}, {Name: "b", Source: "b.mdx"}}
	if err := CheckCollisions(single); err != nil {
		t.Errorf("unique collisions = %v", err)
	}
	dupes := []Claimant{{Name: "a_b", Source: "a-b.mdx"}, {Name: "a_b", Source: "a_b.mdx"}}
	err := CheckCollisions(dupes)
	collision, ok := err.(*CollisionError)
	if !ok {
		t.Fatalf("collisions = %v (%T), want *CollisionError", err, err)
	}
	if collision.Name != "a_b" || len(collision.Claimants) != 2 {
		t.Errorf("collision = %+v", collision)
	}
	message := collision.Error()
	if !strings.Contains(message, "a-b.mdx") || !strings.Contains(message, "a_b.mdx") {
		t.Errorf("collision message omits a claimant: %s", message)
	}
}
