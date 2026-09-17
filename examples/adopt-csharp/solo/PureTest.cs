// Foreign C# test: handwritten owner of the test-owned source.
using System;

public static class PureTestMain
{
    public static int Main(string[] args)
    {
        var got = Solo.Pure.SayHello("world");
        if (got != "hello world")
        {
            Console.WriteLine("FAIL: got '" + got + "'");
            return 1;
        }
        Console.WriteLine("PASS");
        return 0;
    }
}
