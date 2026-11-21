struct Person
    name::String
    age::Int
    email::String
end

mutable struct Pair{T}
    first::T
    second::T
end

function greet(person::Person)
    "hello " * person.name
end

function standalone(person::Person)
    person.age
end
