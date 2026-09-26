package fsharp

import "strings"

var stdlibRoots = []string{
	"System.",
	"Microsoft.",
}

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
