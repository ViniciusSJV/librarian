class Raiz:
    def __init__(self, nome="r"):
        self.nome = nome

    def topo(self):
        return "Raiz.topo"

    def passo(self):
        return "Raiz.passo"

    @classmethod
    def criar(cls):
        return "Raiz.criar"


class Alheia:
    def topo(self):
        return "Alheia.topo"

    def passo(self):
        return "Alheia.passo"

    def criar(self):
        return "Alheia.criar"

    def enviar(self):
        return "Alheia.enviar"

    def mm(self):
        return "Alheia.mm"

    def vizinho(self):
        return "Alheia.vizinho"

    def escondido(self):
        return "Alheia.escondido"

    def inexistente(self):
        return "Alheia.inexistente"


class Transporte:
    def enviar(self):
        return "Transporte.enviar"


class Rapido(Transporte):
    def enviar(self):
        return "Rapido.enviar"


class Canal:
    @property
    def transporte(self) -> Transporte:
        return Rapido()

    @property
    def sem_tipo(self):
        return Transporte()


class SubCanal(Canal):
    pass
