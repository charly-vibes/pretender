package sample

type Person struct {
	Name  string
	Age   uint8
	Email string
}

type Bar[T any] struct {
	First  T
	Second T
}

func (p Person) Greet() string {
	return "hi " + p.Name
}

func Standalone(person *Person) uint8 {
	return person.Age
}
