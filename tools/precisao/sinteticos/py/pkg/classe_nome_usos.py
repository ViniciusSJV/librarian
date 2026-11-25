# Passo 2.4d-0: uma chamada por função; o nome diz o caso.
import pkg.classe_nome
import pkg.classe_nome as modulo
from pkg import classe_nome
from pkg.classe_nome import CnBalde, CnBase, CnComAtributo, CnComMeta, CnContrato, CnFilha, CnHerdeiraDeOutra, CnNeta, CnOutra, CnReligada, CnSem, CnTrocada
from pkg.classe_nome import CnBase as Alias
from pkg.classe_nome_fora import CnReexportada as Reexp
from pkg.classe_nome_fora import CnDic, CnMisto, CnMistoDepois, CnTarefa, CnTarefaPropria

Apelido = CnFilha


class CnLocal:
    @staticmethod
    def somar_cn(a=0):
        return "CnLocal.somar_cn"

    def de_dentro_cn(self):
        return CnLocal.somar_cn()

    class CnConf:
        @staticmethod
        def somar_cn(a=0):
            return "CnLocal.CnConf.somar_cn"

        @staticmethod
        def de_dentro_cn():
            return CnLocal.CnConf.somar_cn()


# como o nome chega
def cn_c_importada():
    return CnBase.somar_cn()


def cn_c_mesmo_modulo():
    return CnLocal.somar_cn()


def cn_c_alias_import():
    return Alias.somar_cn()


def cn_c_alias_modulo():
    return Apelido.somar_cn()


def cn_c_alias_local():
    k = CnFilha
    return k.somar_cn()


def cn_c_modulo():
    return classe_nome.CnBase.somar_cn()


def cn_c_modulo_as():
    return modulo.CnFilha.somar_cn()


def cn_c_modulo_pontos():
    return pkg.classe_nome.CnFilha.somar_cn()


def cn_c_aninhada():
    return CnOutra.CnDentro.somar_cn()


def cn_c_aninhada_modulo():
    return modulo.CnOutra.CnDentro.somar_cn()


def cn_c_aninhada_herdada():
    return CnHerdeiraDeOutra.CnDentro.somar_cn()


def cn_c_import_local():
    from pkg.classe_nome import CnNeta as N

    return N.rotulo_cn(N())


def cn_c_aninhada_trocada():
    return CnTrocada.CnDentro.somar_cn()


def cn_c_reexportada():
    return Reexp.somar_cn()


# item 2
def cn_r_classmethod():
    return CnBase.criar_cn().quem


def cn_r_staticmethod():
    return CnFilha.somar_cn()


def cn_r_instancia():
    return CnBase.rotulo_cn(CnNeta())


def cn_r_instancia_filha():
    return CnFilha.rotulo_cn(CnNeta())


def cn_r_herdado():
    return CnNeta.herdado_cn(CnNeta())


def cn_r_herdado_meio():
    return CnNeta.somar_cn()


def cn_r_solta():
    return CnBalde.autorizar_cn()


def cn_r_sombra_variavel():
    CnBase = CnOutra()
    return CnBase.rotulo_cn()


def cn_r_sombra_parametro(CnBase):
    return CnBase.herdado_cn()


def cn_r_sombra_for():
    for CnBase in [CnOutra]:
        return CnBase.herdado_cn()


def cn_r_sombra_condicional(x=True):
    Base2 = CnBase
    if x:
        Base2 = CnOutra
    return Base2.herdado_cn()


def cn_r_atributo():
    return CnComAtributo.tarefa_cn()


def cn_r_atributo_apelido():
    return CnComAtributo.liberar_cn()


def cn_r_religada():
    return CnReligada.tarefa_cn()


def cn_r_protocolo():
    return CnContrato.rotulo_cn(CnNeta())


def cn_r_ninguem():
    return CnSem.so_outra_cn()


def cn_r_metaclasse():
    return CnComMeta.fabricar_cn()


def cn_r_retorno():
    x = CnBase.criar_cn()
    return x.rotulo_cn()


def cn_r_retorno_herdado():
    x = CnNeta.criar_cn()
    return x.liberar_cn()


def cn_r_init_direto():
    n = CnNeta.__new__(CnNeta)
    CnBase.__init__(n)
    return n.quem


def cn_r_init_herdado():
    n = CnNeta.__new__(CnNeta)
    CnNeta.__init__(n)
    return n.extra


def cn_r_init_ninguem():
    s = CnSem()
    CnSem.__init__(s)
    return type(s).__name__


# base de fora
def cn_f_so_de_fora():
    return CnTarefa.run(CnTarefa())


def cn_f_propria():
    return CnTarefaPropria.rodar_fora_cn()


def cn_f_antes():
    return CnMisto.rotulo_cn(CnMisto())


def cn_f_depois():
    return CnMistoDepois.rotulo_cn(CnMistoDepois())


def cn_f_classmethod_de_fora():
    return CnDic.fromkeys([1])


# guardas: não são a classe pelo nome
def cn_g_objeto():
    return CnBase().rotulo_cn()


def cn_g_tipo(x: CnBase):
    return x.rotulo_cn()


def cn_g_type_self(x: CnBase):
    return type(x).rotulo_cn(x)


def cn_g_religada_objeto(x: CnReligada):
    return x.tarefa_cn


def cn_g_religada_chamada(x: CnReligada):
    return x.tarefa_cn()


def cn_r_sombra_compreensao():
    return [CnBase.herdado_cn() for CnBase in [CnOutra]][0]


def cn_r_condicional(x=True):
    if x:
        from pkg.classe_nome import CnBase as Z
    else:
        from pkg.classe_nome import CnFilha as Z
    return Z.somar_cn()


def cn_r_objeto_por_nome():
    return CnNeta.rotulo_cn.__name__
