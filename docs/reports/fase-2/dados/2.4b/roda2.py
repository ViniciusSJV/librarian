# As regras 1 (abstrato) e 2 (fora da cadeia) no Python de verdade.   python -I -B roda2.py
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "sintetico"))
from pkg.regras import *  # noqa

casos = [
    ("r_abstrato(EnvioA)", lambda: r_abstrato(EnvioA())),
    ("r_abstrato(EnvioC)", lambda: r_abstrato(EnvioC())),
    ("r_abstrato(Envio)", lambda: r_abstrato(Envio())),
    ("Envio.usar (self) em EnvioC", lambda: EnvioC().usar()),
    ("r_abstrato_meio(EnvioB)", lambda: r_abstrato_meio(EnvioB())),
    ("r_vazio(EnvioA)", lambda: r_vazio(EnvioA())),
    ("EnvioSuper.vazio (super): roda o corpo abstrato", lambda: EnvioSuper().vazio()),
    ("r_sem_sobrescrita", lambda: r_sem_sobrescrita(SemSobrescrita())),
    ("r_abc(ComAbcFilha)", lambda: r_abc(ComAbcFilha())),
    ("r_construtor: roda o corpo abstrato", r_construtor),
    ("r_fora", lambda: r_fora(DeFora())),
    ("r_base_dinamica", lambda: r_base_dinamica(DeBaseDinamica())),
    ("r_meta", lambda: r_meta(DeMeta())),
    ("r_decorador", lambda: r_decorador(DeDecorador())),
    ("r_posto_de_fora", lambda: r_posto_de_fora(DeFora())),
    ("base de fora que delega (Mock): Procurador(wraps=Alheia()).mm()", lambda: Procurador(wraps=Alheia()).mm()),
    ("super() nao usa __getattr__: SuperDelegado.mm", lambda: SuperDelegado(Alheia()).mm()),
]
for nome, f in casos:
    try:
        r = f()
    except Exception as e:
        r = type(e).__name__
    print(nome, "=>", r)
