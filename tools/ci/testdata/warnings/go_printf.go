// Warning fixture: vet printf mismatch (go vet printf check).
package warning

import "fmt"

func WarningPrintf() {
	fmt.Printf("%d", "not-a-number")
}
