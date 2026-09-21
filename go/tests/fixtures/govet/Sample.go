// Seed Go vet fixture.
package govet

import "fmt"

func Greet(name string) string {
	fmt.Printf(name)
	return "hello " + name
}
