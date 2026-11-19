pub struct Person {
    pub name: String,
    pub age: u8,
    pub email: String,
}

pub struct Pair<T> {
    pub first: T,
    pub second: T,
}

impl Person {
    pub fn greet(&self) -> String {
        format!("hi {}", self.name)
    }
}

pub fn standalone(person: &Person) -> u8 {
    person.age
}
