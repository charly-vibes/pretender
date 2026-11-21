"""Type-capture fixture for the python adapter round-trip test."""

from dataclasses import dataclass


@dataclass
class Person:
    """Annotated attributes become typed fields; methods stay callables."""

    name: str
    age: int = 0
    email: str = "unknown"

    def greet(self) -> str:
        greeting: str = f"hello {self.name}"
        return greeting


class Account:
    """Second class: __init__ self-assignments are not fields."""

    balance: float
    owner: str = ""

    def __init__(self, owner: str) -> None:
        self.owner = owner
        self.balance = 0.0
