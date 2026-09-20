package typescript

import (
	"strings"
	"testing"
)

func TestNormalize(t *testing.T) {
	cases := []struct{ in, want string }{
		{"hello", "hello"},
		{"hello_test", "hello_test"},
		{"_private", "private"},
		{"my-mod", "my_mod"},
		{"my mod", "my_mod"},
		{"UPPER123", "UPPER123"},
		{"déjà", "d_j"},
	}
	for _, tc := range cases {
		if got, err := Normalize(tc.in); err != nil || got != tc.want {
			t.Errorf("Normalize(%q) = %q, %v; want %q", tc.in, got, err, tc.want)
		}
	}
}

func TestNormalizeEmptyFails(t *testing.T) {
	for _, in := range []string{"", "---", "___"} {
		if got, err := Normalize(in); err == nil {
			t.Errorf("Normalize(%q) = %q, want error", in, got)
		}
	}
}

func TestIsDeclaration(t *testing.T) {
	for _, name := range []string{"foo.d.ts", "pkg/foo.d.mts", "foo.d.cts"} {
		if !IsDeclaration(name) {
			t.Errorf("declaration not recognized: %q", name)
		}
	}
	for _, name := range []string{"foo.ts", "foo.tsx", "foo.mts", "foo.cts", "foo.d.js", "d.ts"} {
		if IsDeclaration(name) {
			t.Errorf("non-declaration recognized: %q", name)
		}
	}
}

func TestIsTestFile(t *testing.T) {
	cases := []struct {
		name string
		want bool
	}{
		{"helper_test.ts", true},
		{"pkg/helper_test.tsx", true},
		{"helper_test.mts", true},
		{"helper_test.cts", true},
		{"helper.ts", false},
		{"helper.d.ts", false},
		{"helper_test.d.ts", false},
		{"helper.test.ts", false},
		{"helper_test.js", false},
		{"_test.ts", true},
	}
	for _, tc := range cases {
		if got := IsTestFile(tc.name); got != tc.want {
			t.Errorf("IsTestFile(%q) = %v, want %v", tc.name, got, tc.want)
		}
	}
}

func TestTargetName(t *testing.T) {
	cases := []struct {
		name string
		want string
	}{
		{"demo.ts", "demo"},
		{"pkg/my-mod.tsx", "my_mod"},
		{"helper_test.mts", "helper_test"},
		{"entry.cts", "entry"},
		{"plain", "plain"},
		{"demo.js", "demo_js"},
	}
	for _, tc := range cases {
		if got, err := TargetName(tc.name); err != nil || got != tc.want {
			t.Errorf("TargetName(%q) = %q, %v; want %q", tc.name, got, err, tc.want)
		}
	}
	if _, err := TargetName("---.ts"); err == nil {
		t.Error("TargetName(---.ts) succeeded, want error")
	}
}

func TestModuleName(t *testing.T) {
	cases := []struct {
		name string
		want string
	}{
		{"demo.ts", "demo"},
		{"pkg/my-mod.tsx", "my-mod"},
		{"entry.mts", "entry"},
		{"plain", "plain"},
		{"demo.js", "demo.js"},
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
	single := []Claimant{{Name: "a", Source: "a.ts"}, {Name: "b", Source: "b.tsx"}}
	if err := CheckCollisions(single); err != nil {
		t.Errorf("unique collisions = %v", err)
	}
	dupes := []Claimant{{Name: "a_b", Source: "a-b.ts"}, {Name: "a_b", Source: "a_b.tsx"}}
	err := CheckCollisions(dupes)
	collision, ok := err.(*CollisionError)
	if !ok {
		t.Fatalf("collisions = %v (%T), want *CollisionError", err, err)
	}
	if collision.Name != "a_b" || len(collision.Claimants) != 2 {
		t.Errorf("collision = %+v", collision)
	}
	message := collision.Error()
	if !strings.Contains(message, "a-b.ts") || !strings.Contains(message, "a_b.tsx") {
		t.Errorf("collision message omits a claimant: %s", message)
	}
}

func TestIsEntryFile(t *testing.T) {
	cases := []struct {
		name string
		want bool
	}{
		{"main.ts", true},
		{"pkg/main.tsx", true},
		{"main.mts", true},
		{"main.cts", true},
		{"main_test.ts", false},
		{"main.d.ts", false},
		{"helper.ts", false},
		{"main.js", false},
		{"main", false},
		// Other layouts are an explicit wont-fix: only the
		// exact `main` basename is an entry. Manifest-declared names
		// (`package.json` `main`/`bin`) are never inferred.
		{"index.ts", false},
		{"app.ts", false},
		{"cli.tsx", false},
		{"pkg/index.mts", false},
		{"bin.cts", false},
		{"src/app.ts", false},
	}
	for _, tc := range cases {
		if got := IsEntryFile(tc.name); got != tc.want {
			t.Errorf("IsEntryFile(%q) = %v, want %v", tc.name, got, tc.want)
		}
	}
}

func TestEntryBinaryName(t *testing.T) {
	if got := EntryBinaryName("main"); got != "main_bin" {
		t.Errorf("EntryBinaryName(main) = %q, want main_bin", got)
	}
	if got := EntryBinaryName("a_b"); got != "a_b_bin" {
		t.Errorf("EntryBinaryName(a_b) = %q, want a_b_bin", got)
	}
}

func TestEntryPointName(t *testing.T) {
	cases := []struct {
		name string
		want string
	}{
		{"main.ts", "main.js"},
		{"pkg/main.tsx", "main.js"},
		{"main.mts", "main.mjs"},
		{"main.cts", "main.cjs"},
	}
	for _, tc := range cases {
		if got := EntryPointName(tc.name); got != tc.want {
			t.Errorf("EntryPointName(%q) = %q, want %q", tc.name, got, tc.want)
		}
	}
}
