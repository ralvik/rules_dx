// Foreign C# test: handwritten owner of the test-owned source. The
// generator never emits csharp_test; `*Test.cs` files stay out of the
// generated library and this rule survives regeneration unchanged.
using System;

public static class GreetTestMain
{
    public static int Main(string[] args)
    {
        var got = Greet.Greeter.SayHello("world");
        if (got != "hello world")
        {
            Console.WriteLine("FAIL: got '" + got + "'");
            return 1;
        }
        if (Greet.Helper.Suffix() != " world")
        {
            Console.WriteLine("FAIL: suffix mismatch");
            return 1;
        }
        Console.WriteLine("PASS");
        return 0;
    }
}
