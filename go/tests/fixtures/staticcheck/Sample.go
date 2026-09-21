// Seed Go lint fixture.
package staticcheck

func Greet(name string) string {
	x := 1
	_ = x
	return "hello " + name
}
