# Nomes proprios deste arquivo (sufixo _cn, prefixo Cn): nao mudam a contagem dos niveis anteriores.
# Passo 2.4d-0: as classes nomeadas. Cada método devolve o próprio nome.
from typing import Protocol


class CnBase:
    def __init__(self):
        self.quem = "CnBase.__init__"

    @classmethod
    def criar_cn(cls) -> "CnBase":
        return cls()

    @staticmethod
    def somar_cn(a=0):
        return "CnBase.somar_cn"

    def rotulo_cn(self):
        return "CnBase.rotulo_cn"

    def herdado_cn(self):
        return "CnBase.herdado_cn"

    def liberar_cn(self):
        return "CnBase.liberar_cn"


class CnFilha(CnBase):
    def __init__(self):
        CnBase.__init__(self)
        self.extra = 1

    @staticmethod
    def somar_cn(a=0):
        return "CnFilha.somar_cn"

    def rotulo_cn(self):
        return CnBase.rotulo_cn(self) + "+CnFilha"


class CnNeta(CnFilha):
    def rotulo_cn(self):
        return "CnNeta.rotulo_cn"


class CnTrocada:
    class CnDentro:
        @staticmethod
        def somar_cn(a=0):
            return "CnTrocada.CnDentro.somar_cn"

    CnDentro = CnFilha


class CnOutra:
    class CnDentro:
        @staticmethod
        def somar_cn(a=0):
            return "CnOutra.CnDentro.somar_cn"

        @staticmethod
        def rotulo_cn():
            return "CnOutra.CnDentro.rotulo_cn"

    @staticmethod
    def criar_cn():
        return CnOutra()

    @staticmethod
    def herdado_cn():
        return "CnOutra.herdado_cn"

    def rotulo_cn(self=None):
        return "CnOutra.rotulo_cn"

    @staticmethod
    def tarefa_cn():
        return "CnOutra.tarefa_cn"

    @staticmethod
    def liberar_cn(x=None):
        return "CnOutra.liberar_cn"

    @staticmethod
    def so_outra_cn():
        return "CnOutra.so_outra_cn"

    @staticmethod
    def rodar_fora_cn():
        return "CnOutra.rodar_fora_cn"


class CnHerdeiraDeOutra(CnOutra):
    pass


def solta_tarefa_cn():
    return "solta_tarefa_cn"


class CnComAtributo:
    tarefa_cn = solta_tarefa_cn
    liberar_cn = CnOutra.liberar_cn


class CnReligada:
    def tarefa_cn():
        return "CnReligada.tarefa_cn (def)"

    tarefa_cn = solta_tarefa_cn


class CnContrato(Protocol):
    def rotulo_cn(self):
        return "CnContrato.rotulo_cn"


class CnSem:
    pass


class CnPrivada:
    @staticmethod
    def __cofre_cn():
        return "CnPrivada.__cofre_cn"

    @staticmethod
    def abrir_cn():
        return CnPrivada.__cofre_cn()


class CnCofre2:
    @staticmethod
    def __cofre_cn():
        return "CnCofre2.__cofre_cn"

    @staticmethod
    def abrir_cn():
        try:
            return CnPrivada.__cofre_cn()
        except AttributeError:
            return "AttributeError"


class CnMeta(type):
    def fabricar_cn(cls):
        return "CnMeta.fabricar_cn"


class CnComMeta(metaclass=CnMeta):
    pass


class CnFabrica:
    @staticmethod
    def fabricar_cn():
        return "CnFabrica.fabricar_cn"


def liberar_cn():
    return "liberar_cn solta"


def autorizar_cn(f=None):
    return "autorizar_cn solta"


class CnBalde:
    @staticmethod
    def autorizar_cn(v=None):
        return "CnBalde.autorizar_cn"

    @staticmethod
    def por_ip_cn():
        return CnBalde.autorizar_cn("ip")
