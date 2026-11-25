# Passo 2.4d-0: uma chamada por função; o nome diz o caso.
import pkg.classes
import pkg.classes as modulo
from pkg import classes
from pkg.classes import Balde, Base, ComAtributo, ComMeta, Contrato, Filha, HerdeiraDeOutra, Neta, Outra, Religada, Sem
from pkg.classes import Base as Alias
from pkg.fora import Dic, Misto, MistoDepois, Tarefa, TarefaPropria

Apelido = Filha


class Local:
    @staticmethod
    def somar(a=0):
        return "Local.somar"

    def de_dentro(self):
        return Local.somar()

    class Conf:
        @staticmethod
        def somar(a=0):
            return "Local.Conf.somar"

        @staticmethod
        def de_dentro():
            return Local.Conf.somar()


# como o nome chega
def c_importada():
    return Base.somar()


def c_mesmo_modulo():
    return Local.somar()


def c_alias_import():
    return Alias.somar()


def c_alias_modulo():
    return Apelido.somar()


def c_alias_local():
    k = Filha
    return k.somar()


def c_modulo():
    return classes.Base.somar()


def c_modulo_as():
    return modulo.Filha.somar()


def c_modulo_pontos():
    return pkg.classes.Filha.somar()


def c_aninhada():
    return Outra.Dentro.somar()


def c_aninhada_modulo():
    return modulo.Outra.Dentro.somar()


def c_aninhada_herdada():
    return HerdeiraDeOutra.Dentro.somar()


def c_import_local():
    from pkg.classes import Neta as N

    return N.rotulo(N())


# item 2
def r_classmethod():
    return Base.criar().quem


def r_staticmethod():
    return Filha.somar()


def r_instancia():
    return Base.rotulo(Neta())


def r_instancia_filha():
    return Filha.rotulo(Neta())


def r_herdado():
    return Neta.herdado(Neta())


def r_herdado_meio():
    return Neta.somar()


def r_solta():
    return Balde.autorizar()


def r_sombra_variavel():
    Base = Outra()
    return Base.rotulo()


def r_sombra_parametro(Base):
    return Base.herdado()


def r_sombra_for():
    for Base in [Outra]:
        return Base.herdado()


def r_sombra_condicional(x=True):
    Base2 = Base
    if x:
        Base2 = Outra
    return Base2.herdado()


def r_atributo():
    return ComAtributo.tarefa()


def r_atributo_apelido():
    return ComAtributo.liberar()


def r_religada():
    return Religada.tarefa()


def r_protocolo():
    return Contrato.rotulo(Neta())


def r_ninguem():
    return Sem.so_outra()


def r_metaclasse():
    return ComMeta.fabricar()


def r_retorno():
    x = Base.criar()
    return x.rotulo()


def r_retorno_herdado():
    x = Neta.criar()
    return x.liberar()


def r_init_direto():
    n = Neta.__new__(Neta)
    Base.__init__(n)
    return n.quem


def r_init_herdado():
    n = Neta.__new__(Neta)
    Neta.__init__(n)
    return n.extra


def r_init_ninguem():
    s = Sem()
    Sem.__init__(s)
    return type(s).__name__


# base de fora
def f_so_de_fora():
    return Tarefa.run(Tarefa())


def f_propria():
    return TarefaPropria.rodar_fora()


def f_antes():
    return Misto.rotulo(Misto())


def f_depois():
    return MistoDepois.rotulo(MistoDepois())


def f_classmethod_de_fora():
    return Dic.fromkeys([1])


# guardas: não são a classe pelo nome
def g_objeto():
    return Base().rotulo()


def g_tipo(x: Base):
    return x.rotulo()


def g_type_self(x: Base):
    return type(x).rotulo(x)


def g_religada_objeto(x: Religada):
    return x.tarefa


def g_religada_chamada(x: Religada):
    return x.tarefa()


def r_sombra_compreensao():
    return [Base.herdado() for Base in [Outra]][0]


def r_condicional(x=True):
    if x:
        from pkg.classes import Base as Z
    else:
        from pkg.classes import Filha as Z
    return Z.somar()


def r_objeto_por_nome():
    return Neta.rotulo.__name__
