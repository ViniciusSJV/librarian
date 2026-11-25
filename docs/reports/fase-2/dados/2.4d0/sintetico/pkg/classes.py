# Passo 2.4d-0: as classes nomeadas. Cada método devolve o próprio nome.
from typing import Protocol


class Base:
    def __init__(self):
        self.quem = "Base.__init__"

    @classmethod
    def criar(cls) -> "Base":
        return cls()

    @staticmethod
    def somar(a=0):
        return "Base.somar"

    def rotulo(self):
        return "Base.rotulo"

    def herdado(self):
        return "Base.herdado"

    def liberar(self):
        return "Base.liberar"


class Filha(Base):
    def __init__(self):
        Base.__init__(self)
        self.extra = 1

    @staticmethod
    def somar(a=0):
        return "Filha.somar"

    def rotulo(self):
        return Base.rotulo(self) + "+Filha"


class Neta(Filha):
    def rotulo(self):
        return "Neta.rotulo"


class Outra:
    class Dentro:
        @staticmethod
        def somar(a=0):
            return "Outra.Dentro.somar"

        @staticmethod
        def rotulo():
            return "Outra.Dentro.rotulo"

    @staticmethod
    def criar():
        return Outra()

    @staticmethod
    def herdado():
        return "Outra.herdado"

    def rotulo(self=None):
        return "Outra.rotulo"

    @staticmethod
    def tarefa():
        return "Outra.tarefa"

    @staticmethod
    def liberar(x=None):
        return "Outra.liberar"

    @staticmethod
    def so_outra():
        return "Outra.so_outra"

    @staticmethod
    def rodar_fora():
        return "Outra.rodar_fora"


class HerdeiraDeOutra(Outra):
    pass


def solta_tarefa():
    return "solta_tarefa"


class ComAtributo:
    tarefa = solta_tarefa
    liberar = Outra.liberar


class Religada:
    def tarefa():
        return "Religada.tarefa (def)"

    tarefa = solta_tarefa


class Contrato(Protocol):
    def rotulo(self):
        return "Contrato.rotulo"


class Sem:
    pass


class Privada:
    @staticmethod
    def __cofre():
        return "Privada.__cofre"

    @staticmethod
    def abrir():
        return Privada.__cofre()


class Cofre2:
    @staticmethod
    def __cofre():
        return "Cofre2.__cofre"

    @staticmethod
    def abrir():
        try:
            return Privada.__cofre()
        except AttributeError:
            return "AttributeError"


class Meta(type):
    def fabricar(cls):
        return "Meta.fabricar"


class ComMeta(metaclass=Meta):
    pass


class Fabrica:
    @staticmethod
    def fabricar():
        return "Fabrica.fabricar"


def liberar():
    return "liberar solta"


def autorizar(f=None):
    return "autorizar solta"


class Balde:
    @staticmethod
    def autorizar(v=None):
        return "Balde.autorizar"

    @staticmethod
    def por_ip():
        return Balde.autorizar("ip")
