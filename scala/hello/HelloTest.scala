// M23 seed Scala test; consumer of dx_scala_test (ScalaTest).
package hello

import org.scalatest.flatspec.AnyFlatSpec

class HelloTest extends AnyFlatSpec {
  "Hello.hello" should "greet by name" in {
    assert(Hello.hello("world") == "hello world")
  }
}
