package mixed

import "path"

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
