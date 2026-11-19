package sample;

class Person {
    String name;
    int age;
    String email;

    String greet() {
        return "hi " + name;
    }
}

class Box<T> {
    T value;

    T get() {
        return value;
    }
}
