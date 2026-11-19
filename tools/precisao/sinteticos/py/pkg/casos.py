from pkg.models import Cliente, Repo, Filho, Canal, fabrica, livre, unica
from pkg.models import Cliente as C


def n0():
    livre(1)
    unica()


def n1():
    A = Cliente("a")
    B = Cliente("b")
    A.salvar()


def n2(a: Cliente):
    a.salvar()


def n3():
    c = fabrica()
    c.salvar()


class Servico:
    def __init__(self):
        self.repo = Repo()

    def gravar(self):
        self.repo.salvar(1)


def n5(f: Filho):
    f.herdado()


def n6(canais: list[Canal]):
    for ch in canais:
        ch.notify(1)


def notify(x):
    return x


def n7(executor, x):
    executor.submit(notify, x)


def n8():
    c = C("x")
    c.salvar()
