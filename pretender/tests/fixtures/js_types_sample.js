class Person {
  name;
  age;
  email;

  greet() {
    return `hi ${this.name}`;
  }
}

class Pair {
  first;
  second;

  swap() {
    return [this.second, this.first];
  }
}

function standalone(person) {
  return person.age;
}
