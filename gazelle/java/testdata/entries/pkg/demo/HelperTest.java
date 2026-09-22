package demo;

import static org.junit.Assert.assertEquals;
import org.junit.Test;

public class HelperTest {
  @Test
  public void testGreet() {
    assertEquals("entry hello world", Main.greet("world"));
  }
}
