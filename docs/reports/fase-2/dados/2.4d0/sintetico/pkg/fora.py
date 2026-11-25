# Passo 2.4d-0: base de fora na cadeia.
from collections import UserDict
from threading import Thread

from pkg.classes import Base


class Tarefa(Thread):
    """`run` vem da Thread; o projeto tem outro `run`."""


class TarefaPropria(Thread):
    @staticmethod
    def rodar_fora():
        return "TarefaPropria.rodar_fora"


class Misto(Thread, Base):
    """A base de fora vem antes; `rotulo` só existe na Base."""


class MistoDepois(Base, Thread):
    """A Base vem antes da de fora."""


class Corredor:
    @staticmethod
    def run(x=None):
        return "Corredor.run"

    @staticmethod
    def fromkeys(x=None):
        return "Corredor.fromkeys"


class Dic(UserDict):
    pass
