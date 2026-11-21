class Person {
  name: string;
  age: number;
  email: string;

  greet(): string {
    return `hi ${this.name}`;
  }
}

class Pair<T> {
  first: T;
  second: T;

  swap(): [T, T] {
    return [this.second, this.first];
  }
}

function standalone(person: Person): number {
  return person.age;
}
