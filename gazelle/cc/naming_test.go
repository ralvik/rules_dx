package cc

import (
	"strings"
	"testing"
)

func TestNormalize(t *testing.T) {
	cases := map[string]string{
		"demo":      "demo",
		"demo-pkg":  "demo_pkg",
		"demo.pkg":  "demo_pkg",
		"demo  pkg": "demo_pkg",
		"a--b..c":   "a_b_c",
		"Ab3":       "Ab3",
		"under_ok":  "under_ok",
	}
	for in, want := range cases {
		got, err := Normalize(in)
		if err != nil || got != want {
			t.Errorf("Normalize(%q) = %q, %v; want %q", in, got, err, want)
		}
	}
	for _, in := range []string{"", "---", "...", "___"} {
		if _, err := Normalize(in); err == nil {
			t.Errorf("Normalize(%q) succeeded, want failure", in)
		}
	}
}

func TestIsTestSource(t *testing.T) {
	for _, name := range []string{"demo_test.cc", "pkg/demo_test.cc", "demo_test.h", "demo_test.cpp", "demo_test.cxx", "demo_test.c", "demo_test.hpp"} {
		if !IsTestSource(name) {
			t.Errorf("IsTestSource(%q) = false, want true", name)
		}
	}
	for _, name := range []string{"demo.cc", "test.cc", "contest.cc", "demo_test.cc.bak", "test_demo.cc", "demo.h", "demo_test.cc.bak"} {
		if IsTestSource(name) {
			t.Errorf("IsTestSource(%q) = true, want false", name)
		}
	}
}

func TestIsSourceIsHeader(t *testing.T) {
	for _, name := range []string{"a.c", "a.cc", "a.cpp", "a.cxx"} {
		if !IsSource(name) || IsHeader(name) {
			t.Errorf("source classification of %q wrong", name)
		}
	}
	for _, name := range []string{"a.h", "a.hh", "a.hpp", "a.hxx"} {
		if !IsHeader(name) || IsSource(name) {
			t.Errorf("header classification of %q wrong", name)
		}
	}
	for _, name := range []string{"a.cu", "a.cuh", "a.S", "notes.txt"} {
		if IsSource(name) || IsHeader(name) {
			t.Errorf("out-of-scope %q recognized, want unrecognized", name)
		}
	}
}

func TestHeaderIdentity(t *testing.T) {
	if got := HeaderIdentity("cc/tests/fixtures/hello/hello.h"); got != "hello.h" {
		t.Errorf("HeaderIdentity = %q, want hello.h", got)
	}
}

func TestDirTargetName(t *testing.T) {
	got, err := DirTargetName("pkg/demo-pkg")
	if err != nil || got != "demo_pkg" {
		t.Errorf("DirTargetName = %q, %v; want demo_pkg", got, err)
	}
}

func TestCheckCollisions(t *testing.T) {
	if err := CheckCollisions([]Claimant{{Name: "a", Source: "a.cc"}, {Name: "b", Source: "b.cc"}}); err != nil {
		t.Errorf("CheckCollisions distinct = %v", err)
	}
	err := CheckCollisions([]Claimant{{Name: "a", Source: "a.cc"}, {Name: "a", Source: "a_v2.cc"}})
	if err == nil || !strings.Contains(err.Error(), `"a"`) || !strings.Contains(err.Error(), "a.cc") || !strings.Contains(err.Error(), "a_v2.cc") {
		t.Errorf("CheckCollisions collision = %v, want every claimant", err)
	}
}
