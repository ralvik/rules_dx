package rust

import (
	"strings"
	"testing"
)

func TestNormalize(t *testing.T) {
	cases := []struct{ in, want string }{
		{"parser", "parser"},
		{"user_profile", "user_profile"},
		{"user-profile", "user_profile"},
		{"user.profile", "user_profile"},
		{"user--profile", "user_profile"},
		{"a b_c", "a_b_c"},
		{"login_test", "login_test"},
		{"UPPER", "UPPER"},
		{"a1_2b", "a1_2b"},
		{"1abc", "1abc"},
		{"-leading", "leading"},
		{"trailing-", "trailing"},
		{"--both--", "both"},
		{"a/b", "a_b"},
		{"a.b.c", "a_b_c"},
		// Non-ASCII bytes join the pending run: one underscore total.
		{"caf\xc3\xa9", "caf"},
		{"\xc3\xa9", ""}, // error case below
		{"a\xc3\xa9b", "a_b"},
		{"under_score_kept", "under_score_kept"},
		{"___", ""}, // error case below
	}
	for _, c := range cases {
		if c.want == "" {
			continue
		}
		got, err := Normalize(c.in)
		if err != nil {
			t.Errorf("Normalize(%q) unexpected error: %v", c.in, err)
			continue
		}
		if got != c.want {
			t.Errorf("Normalize(%q) = %q, want %q", c.in, got, c.want)
		}
	}
}

func TestNormalizeEmptyFails(t *testing.T) {
	for _, in := range []string{"", "---", "___", "...", "\xc3\xa9", "   "} {
		if got, err := Normalize(in); err == nil {
			t.Errorf("Normalize(%q) = %q, want error", in, got)
		}
	}
}

func TestMustNormalizePanics(t *testing.T) {
	defer func() {
		if recover() == nil {
			t.Error("MustNormalize(\"---\") did not panic")
		}
	}()
	MustNormalize("---")
}

func TestCheckCollisions(t *testing.T) {
	if err := CheckCollisions(nil); err != nil {
		t.Fatalf("empty input: %v", err)
	}
	ok := []Claimant{
		{Name: "parser", Source: "src/lib.rs"},
		{Name: "parser_bin", Source: "src/main.rs"},
	}
	if err := CheckCollisions(ok); err != nil {
		t.Fatalf("distinct names: %v", err)
	}
	dup := []Claimant{
		{Name: "user_profile", Source: "user-profile.rs"},
		{Name: "other", Source: "other.rs"},
		{Name: "user_profile", Source: "user_profile.rs"},
	}
	err := CheckCollisions(dup)
	if err == nil {
		t.Fatal("collision not reported")
	}
	msg := err.Error()
	if !strings.Contains(msg, "user_profile") ||
		!strings.Contains(msg, "user-profile.rs") ||
		!strings.Contains(msg, "user_profile.rs") {
		t.Errorf("collision error must name the target and every claimant, got: %s", msg)
	}
}

func TestDerivedNames(t *testing.T) {
	if got := IntegrationTestName("login"); got != "login_test" {
		t.Errorf("IntegrationTestName(login) = %q", got)
	}
	// No duplicated suffix when the basename already ends in _test.
	if got := IntegrationTestName("login_test"); got != "login_test" {
		t.Errorf("IntegrationTestName(login_test) = %q", got)
	}
	if got := UnitTestName("parser"); got != "parser_test" {
		t.Errorf("UnitTestName(parser) = %q", got)
	}
	if got := BinaryName("parser"); got != "parser_bin" {
		t.Errorf("BinaryName(parser) = %q", got)
	}
	if got := ExampleName("demo-app"); got != "demo_app_example" {
		t.Errorf("ExampleName(demo-app) = %q", got)
	}
	if got := ExampleTestName("demo_app_example"); got != "demo_app_example_test" {
		t.Errorf("ExampleTestName = %q", got)
	}
	if got := BenchName("bench.fast"); got != "bench_fast_bench" {
		t.Errorf("BenchName = %q", got)
	}
	if got := BuildScriptName("my-pkg"); got != "my_pkg_build_script" {
		t.Errorf("BuildScriptName = %q", got)
	}
}
