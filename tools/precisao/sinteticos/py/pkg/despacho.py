from abc import ABC, abstractmethod
from threading import Thread


def solta(*a):
    return "solta"


class Veiculo:
    def andar(self):
        return "Veiculo.andar"

    def parar(self):
        return "Veiculo.parar"

    def __motor(self):
        return "Veiculo.__motor"

    def ligar(self):
        return self.__motor()

    def rodar(self):
        return self.andar()

    def frear(self):
        return self.parar()

    @classmethod
    def montar(cls):
        return cls.nome()

    @classmethod
    def nome(cls):
        return "Veiculo.nome"

    def pelo_nome(self):
        return Veiculo.andar(self)


class Carro(Veiculo):
    def andar(self):
        return "Carro.andar"

    @classmethod
    def nome(cls):
        return "Carro.nome"

    def __motor(self):
        return "Carro.__motor"

    def pelo_super(self):
        return super().andar()


class Jipe(Carro):
    pass


class Moto(Veiculo):
    def andar(self):
        return "Moto.andar"


class Irma(Veiculo):
    def rodar(self):
        return "Irma.rodar"

    def andar(self):
        return "Irma.andar"


class Ficha:
    def abrir(self):
        return "Ficha.abrir"

    def usar(self):
        return self.abrir()


class MisturaFicha:
    def abrir(self):
        return "MisturaFicha.abrir"


class FichaMisturada(MisturaFicha, Ficha):
    pass


class Porta:
    def trancar(self):
        return "Porta.trancar"


class PortaAtributo(Porta):
    trancar = solta


class Motor:
    def girar(self):
        return "Motor.girar"


class MotorFio(Thread, Motor):
    pass


class Forma(ABC):
    @abstractmethod
    def area(self):
        return 0


class FormaMeio(Forma):
    def area(self):
        raise NotImplementedError


class Quadrado(FormaMeio):
    def area(self):
        return "Quadrado.area"


class Solida(Quadrado):
    def area(self):
        return "Solida.area"


class ComNovo:
    def __new__(cls):
        return object.__new__(ComNovoFilha)

    def agir(self):
        return "ComNovo.agir"


class ComNovoFilha(ComNovo):
    def agir(self):
        return "ComNovoFilha.agir"


class Garagem:
    def __init__(self):
        self.carro = Carro()
        self.veiculo = Veiculo()

    @property
    def atual(self) -> Veiculo:
        return Moto()

    def do_campo(self):
        return self.veiculo.andar()

    def da_propriedade(self):
        return self.atual.andar()


def novo_veiculo() -> Veiculo:
    return Moto()


def d_parametro(x: Veiculo):
    return x.andar()


def d_meio(x: Carro):
    return x.andar()


def d_irma(x: Moto):
    return x.andar()


def d_sem_sobrescrita(x: Veiculo):
    return x.parar()


def d_construtor():
    x = Veiculo()
    return x.andar()


def d_construtor_direto():
    return Veiculo().andar()


def d_construtor_anotado():
    x: Veiculo = Carro()
    return x.andar()


def d_retorno():
    x = novo_veiculo()
    return x.andar()


def d_abstrato(x: Forma):
    return x.area()


def d_abstrato_meio(x: Quadrado):
    return x.area()


def d_mistura(x: Ficha):
    return x.abrir()


def d_atributo(x: Porta):
    return x.trancar()


def d_fora_antes(x: Motor):
    return x.girar()


def d_novo():
    x = ComNovo()
    return x.agir()
