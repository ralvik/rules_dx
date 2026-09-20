// Package mixed certifies the mixed-framework ownership boundary.
//
// Each v1 framework container keeps exactly one physical owner: .vue is
// vue-owned, .svelte is svelte-owned, .astro is astro-owned, .mdx is
// mdx-owned, and core .js/.jsx/.mjs/.cjs sources stay javascript-owned
// (.ts/.tsx/.mts/.cts/.d.ts stay typescript-owned). No container is owned
// twice, no region becomes a second physical source, and unknown extensions
// have no owner (no generic fallback). Unused adapters do no eager work:
// an empty directory or a directory without a supported extension yields no
// rules.
//
// The mapping is deliberately mechanical (extension switch, no regex, no
// prose scan) so each concrete adapter remains the authority for its own
// regions; this package only proves the partition is disjoint and complete
// for the closed v1 set.
package mixed

import "path"

// Owner reports the owning adapter for one source basename: vue, svelte,
// astro, mdx, javascript, typescript, or "" when no v1 adapter owns it.
// Matching is case-sensitive on the final extension (path.Ext, no
// lowercasing): `Hello.VUE` is unowned, exactly like an unknown extension.
// `.d.ts` stays typescript-owned because its final extension is `.ts`.
// Query strings and fragments never appear in checked-in paths.
func Owner(name string) string {
	base := path.Base(name)
	switch path.Ext(base) {
	case ".vue":
		return "vue"
	case ".svelte":
		return "svelte"
	case ".astro":
		return "astro"
	case ".mdx":
		return "mdx"
	case ".js", ".jsx", ".mjs", ".cjs":
		return "javascript"
	case ".ts", ".tsx", ".mts", ".cts":
		return "typescript"
	default:
		return ""
	}
}

// Owners partitions one directory listing into adapter -> sorted basenames.
// Files with no owner are dropped (no fallback). The result never contains
// an empty adapter entry, so callers can assert laziness by checking for a
// nil/empty map.
func Owners(files []string) map[string][]string {
	out := map[string][]string{}
	for _, f := range files {
		owner := Owner(f)
		if owner == "" {
			continue
		}
		base := path.Base(f)
		out[owner] = append(out[owner], base)
	}
	for k := range out {
		// Insertion order is source order; sort for determinism.
		sorted := append([]string(nil), out[k]...)
		for i := 1; i < len(sorted); i++ {
			for j := i; j > 0 && sorted[j] < sorted[j-1]; j-- {
				sorted[j], sorted[j-1] = sorted[j-1], sorted[j]
			}
		}
		out[k] = sorted
	}
	if len(out) == 0 {
		return nil
	}
	return out
}
