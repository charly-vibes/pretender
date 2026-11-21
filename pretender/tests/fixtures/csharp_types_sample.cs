namespace Sample;

class Person
{
    public string Name;
    public int Age;
    public string Email;

    public string Greet()
    {
        return "hi " + Name;
    }
}

class Box<T>
{
    public T Value;

    public T Get()
    {
        return Value;
    }
}
