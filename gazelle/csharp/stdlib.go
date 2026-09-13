// Stdlib lists the .NET import roots treated as standard library by the
// C# Gazelle extension (M23, O31). Generation treats these imports as
// SDK-provided without an edge, a manifest, or a lockfile. The set covers
// the `System.*` and `Microsoft.*` roots; it is revisited when O31 freezes
// the SDK baseline.
package csharp

import "strings"

// stdlibRoots are the dotted-path roots treated as SDK-provided.
var stdlibRoots = []string{
	"System.",
	"Microsoft.",
}

// IsStdLib reports whether a dotted import path is SDK-provided.
func IsStdLib(spec string) bool {
	if spec == "System" || spec == "Microsoft" {
		return true
	}
	for _, root := range stdlibRoots {
		if strings.HasPrefix(spec, root) {
			return true
		}
	}
	return false
}
