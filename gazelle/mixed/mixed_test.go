package mixed

import (
	"reflect"
	"testing"
)

func TestOwnerPartition(t *testing.T) {
	cases := []struct {
		name  string
		files []string
		want  map[string][]string
	}{
		{
			"empty",
			nil,
			nil,
		},
		{
			"unsupportedOnly",
			[]string{"notes.txt", "README.md", "data.json"},
			nil,
		},
		{
			"mixedPackage",
			[]string{"Hello.vue", "Hello.svelte", "Hello.astro", "Hello.mdx", "helper.js", "notes.txt"},
			map[string][]string{
				"astro":      {"Hello.astro"},
				"javascript": {"helper.js"},
				"mdx":        {"Hello.mdx"},
				"svelte":     {"Hello.svelte"},
				"vue":        {"Hello.vue"},
			},
		},
		{
			"typescriptOwned",
			[]string{"widget.ts", "helper.tsx", "types.d.ts", "main.mts", "lib.cts"},
			map[string][]string{
				"typescript": {"helper.tsx", "lib.cts", "main.mts", "types.d.ts", "widget.ts"},
			},
		},
		{
			"javascriptVariants",
			[]string{"a.js", "b.jsx", "c.mjs", "d.cjs"},
			map[string][]string{
				"javascript": {"a.js", "b.jsx", "c.mjs", "d.cjs"},
			},
		},
		{
			"caseSensitive",
			[]string{"Hello.VUE", "Hello.MDX"},
			nil,
		},
		{
			"noDoubleOwner",
			[]string{"demo.vue", "demo.svelte", "demo.astro", "demo.mdx"},
			map[string][]string{
				"astro":  {"demo.astro"},
				"mdx":    {"demo.mdx"},
				"svelte": {"demo.svelte"},
				"vue":    {"demo.vue"},
			},
		},
	}
	for _, tc := range cases {
		if got := Owners(tc.files); !reflect.DeepEqual(got, tc.want) {
			t.Errorf("%s: Owners = %v, want %v", tc.name, got, tc.want)
		}
	}
}

func TestOwnerSingle(t *testing.T) {
	owned := map[string]string{
		"Hello.vue":    "vue",
		"Hello.svelte": "svelte",
		"Hello.astro":  "astro",
		"Hello.mdx":    "mdx",
		"helper.js":    "javascript",
		"widget.ts":    "typescript",
		"pkg/demo.vue": "vue",
	}
	for file, want := range owned {
		if got := Owner(file); got != want {
			t.Errorf("Owner(%q) = %q, want %q", file, got, want)
		}
	}
	unowned := []string{"", "notes.txt", "README.md", "data.json", "style.css", "Hello.VUE", "a.foo", "dir/"}
	for _, file := range unowned {
		if got := Owner(file); got != "" {
			t.Errorf("Owner(%q) = %q, want empty", file, got)
		}
	}
}

// Per-adapter shared-edge agreement (each concrete parser extracts the
// mixed helper edge while fenced/markup regions stay inert) is covered by
// the existing per-adapter parser suites plus the //mixed/hello:hello_test
// runtime edge; this package owns only the disjoint partition.
