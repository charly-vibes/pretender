struct Person {
    char *name;
    int age;
    double height;
};

union Value {
    int as_int;
    double as_double;
};

int person_age(const struct Person *person) {
    return person->age;
}
