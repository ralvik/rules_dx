// Seed C# xUnit fixture library (issue #477): xUnit v3 4.0.0 mapping.
namespace XunitFixture;

public static class Greeter
{
    public static string Greet(string name)
    {
        return "hello " + name;
    }
}
