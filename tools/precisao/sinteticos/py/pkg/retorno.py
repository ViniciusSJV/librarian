from datetime import datetime

from pkg.requisicao import Requisicao


class Pedido:
    def fechar(self):
        return 1

    def nota(self):
        return Nota()

    def proximo(self):
        return self


class Nota:
    def fechar(self):
        return 2

    def nota(self):
        return self

    def proximo(self):
        return self


class Fabrica:
    def pedido(self) -> Pedido:
        return Pedido()


def novo_pedido() -> Pedido:
    return Pedido()


def opcional() -> Pedido | None:
    return None


def agora() -> datetime:
    return datetime.now()


def pedidos() -> list[Pedido]:
    return []


def sem_anotacao():
    return Pedido()


def duplo() -> Pedido:
    return Pedido()


def mesmo():
    p = novo_pedido()
    p.fechar()


def metodo(fab: Fabrica):
    p = fab.pedido()
    p.fechar()


def talvez():
    p = opcional()
    p.fechar()


def campo(r: Requisicao):
    r.pedido.fechar()


def fora():
    d = agora()
    d.fechar()


def generico():
    ps = pedidos()
    ps.fechar()


def sem_tipo():
    p = sem_anotacao()
    p.fechar()


def dois_alvos():
    d = duplo()
    d.fechar()


def cadeia():
    p = novo_pedido()
    n = p.nota()
    n.fechar()


def laco():
    x = x.proximo()
    x.fechar()
