# O despacho no Python de verdade (3.11, o do Blender).   python -I -B roda.py
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "sintetico"))
from pkg.despacho import *  # noqa

casos = [
    ("Veiculo.rodar (self) em Veiculo, Carro, Jipe, Moto", lambda: [c().rodar() for c in (Veiculo, Carro, Jipe, Moto)]),
    ("Veiculo.rodar (self) em Irma, que sobrescreve quem chama", lambda: Irma().rodar()),
    ("Veiculo.rodar(Irma()), pelo nome da classe", lambda: Veiculo.rodar(Irma())),
    ("Veiculo.frear (self) em Veiculo, Carro", lambda: [c().frear() for c in (Veiculo, Carro)]),
    ("Veiculo.montar (cls) em Veiculo, Carro, Moto", lambda: [c.montar() for c in (Veiculo, Carro, Moto)]),
    ("Veiculo.ligar (self.__motor) em Carro", lambda: Carro().ligar()),
    ("Veiculo.pelo_nome em Carro", lambda: Carro().pelo_nome()),
    ("Carro.pelo_super em Jipe", lambda: Jipe().pelo_super()),
    ("d_parametro em Veiculo, Jipe, Moto", lambda: [d_parametro(c()) for c in (Veiculo, Jipe, Moto)]),
    ("d_meio em Carro, Jipe", lambda: [d_meio(c()) for c in (Carro, Jipe)]),
    ("d_irma", lambda: d_irma(Moto())),
    ("d_sem_sobrescrita em Carro", lambda: d_sem_sobrescrita(Carro())),
    ("d_construtor", d_construtor),
    ("d_construtor_direto", d_construtor_direto),
    ("d_construtor_anotado", d_construtor_anotado),
    ("d_retorno", d_retorno),
    ("Garagem.do_campo", lambda: Garagem().do_campo()),
    ("Garagem.da_propriedade", lambda: Garagem().da_propriedade()),
    ("d_abstrato em FormaMeio, Quadrado, Solida", lambda: [d_abstrato(c()) for c in (Quadrado, Solida)]),
    ("d_abstrato_meio em Quadrado, Solida", lambda: [d_abstrato_meio(c()) for c in (Quadrado, Solida)]),
    ("Ficha.usar (self) em Ficha, FichaMisturada", lambda: [c().usar() for c in (Ficha, FichaMisturada)]),
    ("d_mistura em FichaMisturada", lambda: d_mistura(FichaMisturada())),
    ("d_atributo em PortaAtributo", lambda: d_atributo(PortaAtributo())),
    ("d_fora_antes em MotorFio", lambda: d_fora_antes(MotorFio())),
    ("d_novo: __new__ devolve a filha", d_novo),
]
for nome, f in casos:
    try:
        r = f()
    except Exception as e:
        r = type(e).__name__
    print(nome, "=>", r)
