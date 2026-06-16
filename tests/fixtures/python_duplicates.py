# Fixture: intentional structural clones for duplication detection tests

def process_users(users):
    result = []
    for item in users:
        if item > 0:
            result.append(item)
        else:
            result.append(0)
    return result


def process_orders(orders):
    result = []
    for item in orders:
        if item > 0:
            result.append(item)
        else:
            result.append(0)
    return result


def unique_logic(x, y):
    return x * y + x - y


def another_unique(a, b, c):
    if a > b:
        return a
    return c
