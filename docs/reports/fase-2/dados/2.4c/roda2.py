class A:
    def m(self): return "A.m"
class Meio(A):
    def m(self): pass
class Fim(Meio):
    def m(self): return "Fim.m"
from abc import ABC, abstractmethod
class AbsMeio(A, ABC):
    @abstractmethod
    def m(self): return "AbsMeio.m"
class AbsFim(AbsMeio):
    def m(self): return "AbsFim.m"
for c in (A, Meio, Fim): print("x.m() com", c.__name__, "=>", c().m(), "| corpo:", type(c()).m.__qualname__)
try: AbsMeio()
except TypeError as e: print("AbsMeio() =>", type(e).__name__)
print("AbsFim().m() =>", AbsFim().m())
