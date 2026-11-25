# Passo 2.4d-0: base de fora na cadeia.
from collections import UserDict
from threading import Thread

from pkg.classe_nome import CnBase
from pkg.classe_nome import CnFilha as CnReexportada


class CnTarefa(Thread):
    """`run` vem da Thread; o projeto tem outro `run`."""


class CnTarefaPropria(Thread):
    @staticmethod
    def rodar_fora_cn():
        return "CnTarefaPropria.rodar_fora_cn"


class CnMisto(Thread, CnBase):
    """A base de classe_nome_fora vem antes; `rotulo_cn` só existe na CnBase."""


class CnMistoDepois(CnBase, Thread):
    """A CnBase vem antes da de classe_nome_fora."""


class CnCorredor:
    @staticmethod
    def run(x=None):
        return "CnCorredor.run"

    @staticmethod
    def fromkeys(x=None):
        return "CnCorredor.fromkeys"


class CnDic(UserDict):
    pass
