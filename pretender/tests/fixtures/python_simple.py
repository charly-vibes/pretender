# expected_complexity: simple=1, with_branch=2, complex_func=6, __init__=1, method_simple=1, method_with_loop=3

def simple(x):
    return x + 1


def with_branch(x):
    if x > 0:
        return x
    return -x


def complex_func(a, b, c):
    if a > 0:
        if b > 0:
            return a + b
        elif c > 0:
            return a + c
    for i in range(10):
        if i % 2 == 0:
            print(i)
    return 0


class MyClass:
    def __init__(self, value):
        self.value = value

    def method_simple(self):
        return self.value

    def method_with_loop(self, items):
        result = []
        for item in items:
            if item > 0:
                result.append(item)
        return result
