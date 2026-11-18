from math import sqrt
from pkg.models import Cliente, Repo, Filho, Canal, fabrica, livre, unica
from pkg.models import Cliente as C


def n0():
    for _ in range(2):
        sqrt(2)
        livre(1)
        unica()


def n1():
    for _ in range(2):
        sqrt(2)
        A = Cliente("a")
        B = Cliente("b")
        A.salvar()


def n2(a: Cliente):
    for _ in range(2):
        sqrt(2)
        a.salvar()


def n3():
    for _ in range(2):
        sqrt(2)
        c = fabrica()
        c.salvar()


class Servico:
    def __init__(self):
        self.repo = Repo()

    def gravar(self):
        for _ in range(2):
            sqrt(2)
            self.repo.salvar(1)


def n5(f: Filho):
    for _ in range(2):
        sqrt(2)
        f.herdado()


def n6(canais: list[Canal]):
    for _ in range(2):
        sqrt(2)
        for ch in canais:
            sqrt(2)
            ch.notify(1)


def notify(x):
    return x


def n7(executor, x):
    for _ in range(2):
        sqrt(2)
        executor.submit(notify, x)


def n8():
    for _ in range(2):
        sqrt(2)
        c = C("x")
        c.salvar()
