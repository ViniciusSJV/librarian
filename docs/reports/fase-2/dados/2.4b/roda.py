# Conferencia com o Python de verdade (3.11, o do Blender): que metodo roda em cada caso do sintetico.
#   python -I -B roda.py
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "sintetico"))
from pkg.heranca import *  # noqa
from pkg import heranca as h  # noqa

n = Neto()
casos = [
    ("c_cadeia", lambda: c_cadeia(n)),
    ("c_generica", lambda: c_generica(CaixaDe())),
    ("c_apelido", lambda: c_apelido(PorApelido())),
    ("Filho.passo (super)", n.passo),
    ("Filho.passo_velho (super(C, self))", n.passo_velho),
    ("Filho.proprio (self)", n.proprio),
    ("Filho.fabrica (cls)", Neto.fabrica),
    ("c_sobrescrita com Raiz()", lambda: c_sobrescrita(Raiz())),
    ("c_fora_antes", lambda: c_fora_antes(ForaAntes())),
    ("c_fora_depois", lambda: c_fora_depois(ForaDepois())),
    ("c_so_fora", lambda: c_so_fora(SoFora())),
    ("c_diamante", lambda: c_diamante(DD())),
    ("Mistura.ir (self) em Junta", lambda: Junta().ir()),
    ("CoopA.mm (super) em CoopA", lambda: CoopA().mm()),
    ("CoopA.mm (super) em CoopD", lambda: CoopD().mm()),
    ("MistSuper.mm (super) em MistJunta", lambda: MistJunta().mm()),
    ("c_atributo", lambda: c_atributo(NomeDaClasse())),
    ("c_instancia", lambda: c_instancia(NomeDaInstancia())),
    ("c_getattr", lambda: c_getattr(ComGetattr())),
    ("c_condicional", lambda: c_condicional(PorCond())),
    ("c_derruba", lambda: c_derruba(n)),
    ("c_prop", lambda: c_prop(Canal())),
    ("c_prop_herdada", lambda: c_prop_herdada(SubCanal())),
    ("c_prop_sem_tipo", lambda: c_prop_sem_tipo(Canal())),
    ("c_construtor", c_construtor),
    ("Neto() rodou Raiz.__init__", lambda: hasattr(n, "nome")),
    ("Dados() rodou Raiz.__init__", lambda: hasattr(c_dataclass(), "nome") ),
    ("ComMeta() rodou Raiz.__init__", lambda: hasattr(c_meta(), "nome")),
    ("ComNew() rodou Raiz.__init__", lambda: hasattr(c_new(), "nome")),
]
for nome, f in casos:
    try:
        r = f()
    except Exception as e:
        r = type(e).__name__
    print(nome, "=>", r)
for c in (DD, CoopD, MistJunta, ForaAntes, ForaDepois):
    print("MRO", [k.__name__ for k in c.__mro__])
try:
    type("Ruim", (DA, DB), {})
except TypeError as e:
    print("class Ruim(DA, DB) =>", type(e).__name__)
