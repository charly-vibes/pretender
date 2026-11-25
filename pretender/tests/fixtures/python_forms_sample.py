"""Forms fixture (pretender-1v8k.4): Python adapter forms the types-fixture
round trip does not cover. Each form is an assert-guard pinning current
adapter behavior; the test file carries the assertions."""

import asyncio

# expected_cyclomatic: fetch=1 — async def captured as one unit, the
# async keyword adds nothing.
async def fetch(url):
    return url

# expected_cyclomatic: make_greeter=1 — the nested greet is reported as its
# own unit AND excluded from the parent's body metrics (greet's branch and
# call do not leak into make_greeter). greet=2 on its own.
def make_greeter(prefix):
    def greet(name):
        if not name:
            raise ValueError("empty")
        return prefix + name
    return greet

class Widget:
    # expected_cyclomatic: size=1, tag=1 — decorated defs are captured and
    # decorators add no complexity; @property/@staticmethod resolve to
    # kind Method like any class member that is not a constructor.
    @property
    def size(self):
        return self._size

    @staticmethod
    def tag():
        return "widget"

    async def pulse(self):
        await asyncio.sleep(0)

    # expected_cyclomatic: shield=3 — except* groups are counted per group
    # as catch branches.
    def shield(self):
        try:
            return asyncio.gather(self.pulse())
        except* ValueError:
            return 1
        except* TypeError:
            return 2

    # expected_cyclomatic: sieve=6 — comprehensions: the for clause of each
    # comprehension is a loop branch, each if filter is a comprehension
    # filter (slices 3.1/3.2).
    def sieve(self, pairs):
        kept = {k: v for k, v in pairs if k > 0 if v}
        flattened = (x for row in sorted(kept) for x in row)
        return kept, flattened
