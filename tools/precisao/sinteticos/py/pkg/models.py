class Cliente:
    def __init__(self, nome):
        self.nome = nome

    def salvar(self):
        return self.nome


class Fornecedor:
    def salvar(self):
        return 2


class Repo:
    def salvar(self, x):
        return x


class Arquivo:
    def salvar(self, x):
        return x


class Base:
    def herdado(self):
        return 1


class Filho(Base):
    def proprio(self):
        return self.herdado()


class Outro:
    def herdado(self):
        return 2


class Canal:
    def notify(self, x):
        return x


class Email(Canal):
    def notify(self, x):
        return x


class Sms(Canal):
    def notify(self, x):
        return x


class Fila:
    def notify(self, x):
        return x


def fabrica() -> Cliente:
    return Cliente("a")


def livre(x):
    return x


def unica():
    return 1
