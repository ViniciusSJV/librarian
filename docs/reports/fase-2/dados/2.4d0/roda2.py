# Os níveis CN-py-* do portão no Python de verdade (3.11, o do Blender).   python -I -B roda2.py
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../../tools/precisao/sinteticos/py"))
from pkg import classe_nome as classes, classe_nome_usos as usos  # noqa

args = {
    "cn_r_sombra_parametro": lambda: usos.cn_r_sombra_parametro(classes.CnOutra),
    "cn_g_religada_objeto": lambda: usos.cn_g_religada_objeto(classes.CnReligada).__name__,
    "cn_g_religada_chamada": lambda: usos.cn_g_religada_chamada(classes.CnReligada),
    "cn_g_tipo": lambda: usos.cn_g_tipo(classes.CnNeta()),
    "cn_g_type_self": lambda: usos.cn_g_type_self(classes.CnNeta()),
}
for nome, f in vars(usos).items():
    if nome.startswith("cn_") and callable(f):
        try:
            r = args.get(nome, f)()
        except Exception as e:
            r = f"{type(e).__name__}: {e}"
        print(nome, "=>", r)
print("CnFilha().quem =>", classes.CnFilha().quem, "| rotulo_cn =>", classes.CnFilha().rotulo_cn())
print("CnLocal =>", usos.CnLocal().de_dentro_cn(), usos.CnLocal.CnConf.de_dentro_cn(), "| CnPrivada =>", classes.CnPrivada.abrir_cn(), "| CnCofre2 =>", classes.CnCofre2.abrir_cn(), "| CnBalde =>", classes.CnBalde.por_ip_cn())
