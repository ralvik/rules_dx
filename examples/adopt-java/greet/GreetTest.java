// Foreign Java test: handwritten owner of the test-owned source. The
// generator never emits java_test; `*Test.java` files stay out of the
// generated library and this rule survives regeneration unchanged.
package greet;

import static org.junit.Assert.assertEquals;
import org.junit.Test;

public class GreetTest {
  @Test
  public void testGreet() {
    assertEquals("hello world", Greet.greet("world"));
  }

  @Test
  public void testSuffix() {
    assertEquals(" world", Helper.suffix());
  }

  @Test
  public void testGuavaJoin() {
    assertEquals("hello,world", Guava.join());
  }
}
