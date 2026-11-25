from dataclasses import dataclass
from threading import Thread
from typing import Generic, Protocol, TypeVar

from pkg.apoio import Alheia, Canal, Raiz, SubCanal, Transporte
from pkg.apoio import Raiz as Apelido

T = TypeVar("T")


class Filho(Raiz):
    def passo(self):
        return super().passo()

    def passo_velho(self):
        return super(Filho, self).passo()

    def proprio(self):
        return self.topo()

    @classmethod
    def fabrica(cls):
        return cls.criar()


class Neto(Filho):
    pass


class Caixa(Raiz, Generic[T]):
    pass


class CaixaDe(Caixa[int]):
    pass


class PorApelido(Apelido):
    pass


class ForaAntes(Thread, Raiz):
    pass


class ForaDepois(Raiz, Thread):
    pass


class SoFora(Thread):
    pass


class DA:
    def mm(self):
        return "DA.mm"


class DB(DA):
    pass


class DC(DA):
    def mm(self):
        return "DC.mm"


class DD(DB, DC):
    pass


class Vizinha:
    def vizinho(self):
        return "Vizinha.vizinho"


class Mistura:
    def ir(self):
        return self.vizinho()


class Junta(Mistura, Vizinha):
    pass


class CoopBase:
    def mm(self):
        return "CoopBase.mm"


class CoopA(CoopBase):
    def mm(self):
        return super().mm()


class CoopB(CoopBase):
    def mm(self):
        return "CoopB.mm"


class CoopD(CoopA, CoopB):
    pass


class MistSuper:
    def mm(self):
        return super().mm()


class MistJunta(MistSuper, DA):
    pass


def solta(*a):
    return "solta"


class BaseEsc:
    def escondido(self):
        return "BaseEsc.escondido"


class NomeDaClasse(BaseEsc):
    escondido = solta


class NomeDaInstancia(BaseEsc):
    def __init__(self):
        self.escondido = solta


class ComGetattr(Raiz):
    def __getattr__(self, n):
        return solta


class Proto(Protocol):
    def mm(self): ...


class DeProto(Proto):
    pass


if len(__name__) > 3:
    Cond = Raiz
else:
    Cond = Alheia


class PorCond(Cond):
    pass


@dataclass
class Dados(Raiz):
    x: int = 1


class Meta(type):
    def __call__(cls, *a):
        return object.__new__(cls)


class ComMeta(Raiz, metaclass=Meta):
    pass


class ComNew(Raiz):
    def __new__(cls, *a):
        return "novo"


def c_cadeia(x: Neto):
    return x.topo()


def c_generica(x: CaixaDe):
    return x.topo()


def c_apelido(x: PorApelido):
    return x.topo()


def c_sobrescrita(x: Raiz):
    return x.passo()


def c_fora_antes(x: ForaAntes):
    return x.topo()


def c_fora_depois(x: ForaDepois):
    return x.topo()


def c_so_fora(x: SoFora):
    return x.topo()


def c_diamante(x: DD):
    return x.mm()


def c_atributo(x: NomeDaClasse):
    return x.escondido()


def c_instancia(x: NomeDaInstancia):
    return x.escondido()


def c_getattr(x: ComGetattr):
    return x.topo()


def c_protocolo(x: DeProto):
    return x.mm()


def c_condicional(x: PorCond):
    return x.topo()


def c_derruba(x: Neto):
    return x.inexistente()


def c_prop(x: Canal):
    return x.transporte.enviar()


def c_prop_herdada(x: SubCanal):
    return x.transporte.enviar()


def c_prop_sem_tipo(x: Canal):
    return x.sem_tipo.enviar()


def c_construtor():
    n = Neto()
    return n.topo()


def c_dataclass():
    return Dados()


def c_meta():
    return ComMeta()


def c_new():
    return ComNew()
