package demo

import org.junit.Test

class HelperTest {
  @Test
  fun testGreet() {
    assert(Main.greet("world") == "entry hello world")
  }
}
