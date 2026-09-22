package demo

import org.junit.Test

class DemoTest {
  @Test
  fun testGreet() {
    assert(Demo.greet("world") == "hello world")
  }
}
