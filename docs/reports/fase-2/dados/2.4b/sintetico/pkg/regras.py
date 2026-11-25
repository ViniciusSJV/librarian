from abc import ABC, abstractmethod
from threading import Thread
from unittest.mock import Mock

from pkg.apoio import Alheia


class Envio:
    def mandar(self):
        """Abstrato."""
        raise NotImplementedError()

    def vazio(self):
        pass

    def usar(self):
        return self.mandar()


class EnvioA(Envio):
    def mandar(self):
        return "EnvioA.mandar"

    def vazio(self):
        return "EnvioA.vazio"


class EnvioB(EnvioA):
    pass


class EnvioC(Envio):
    def mandar(self):
        return "EnvioC.mandar"


class EnvioSuper(Envio):
    def vazio(self):
        return super().vazio()


class SemSobrescrita:
    def ninguem(self):
        raise NotImplementedError()


class ComAbc(ABC):
    @abstractmethod
    def mandar(self):
        return 1


class ComAbcFilha(ComAbc):
    def mandar(self):
        return "ComAbcFilha.mandar"


class Fio(Thread):
    def topo_de_fora(self):
        return self.enviar()


class Procurador(Mock):
    pass


class Delegado:
    def __init__(self, alvo):
        self._alvo = alvo

    def __getattr__(self, n):
        return getattr(self._alvo, n)


class SuperDelegado(Delegado):
    def mm(self):
        return super().mm()


def fabrica_de_base(k):
    return type("Base", (Thread,), {"mm": k.mm})


class DeBaseDinamica(fabrica_de_base(Alheia)):
    pass


class MetaPoe(type):
    def __new__(m, n, b, d):
        d["mm"] = Alheia.mm
        return type.__new__(m, n, b, d)


class DeMeta(Thread, metaclass=MetaPoe):
    pass


def poe(c):
    c.mm = Alheia.mm
    return c


@poe
class DeDecorador(Thread):
    pass


class DeFora(Thread):
    pass


def r_abstrato(x: Envio):
    return x.mandar()


def r_abstrato_meio(x: EnvioA):
    return x.mandar()


def r_vazio(x: Envio):
    return x.vazio()


def r_sem_sobrescrita(x: SemSobrescrita):
    return x.ninguem()


def r_abc(x: ComAbc):
    return x.mandar()


def r_construtor():
    e = Envio()
    return e.mandar()


def r_fora(x: DeFora):
    return x.mm()


def r_base_dinamica(x: DeBaseDinamica):
    return x.mm()


def r_meta(x: DeMeta):
    return x.mm()


def r_decorador(x: DeDecorador):
    return x.mm()


def r_posto_de_fora(x: DeFora):
    x.mm = Alheia().mm
    return x.mm()
