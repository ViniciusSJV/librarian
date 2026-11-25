# Os casos no Python de verdade (3.11, o do Blender).   python -I -B roda.py
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "sintetico"))
from pkg import classes, usos  # noqa

args = {
    "r_sombra_parametro": lambda: usos.r_sombra_parametro(classes.Outra),
    "g_religada_objeto": lambda: usos.g_religada_objeto(classes.Religada).__name__,
    "g_religada_chamada": lambda: usos.g_religada_chamada(classes.Religada),
    "g_tipo": lambda: usos.g_tipo(classes.Neta()),
    "g_type_self": lambda: usos.g_type_self(classes.Neta()),
}
for nome, f in vars(usos).items():
    if nome[:2] in ("c_", "r_", "f_", "g_") and callable(f):
        try:
            r = args.get(nome, f)()
        except Exception as e:
            r = f"{type(e).__name__}: {e}"
        print(nome, "=>", r)
print("Filha().quem =>", classes.Filha().quem, "| Filha().rotulo =>", classes.Filha().rotulo())
print("Local.de_dentro =>", usos.Local().de_dentro(), "| Local.Conf.de_dentro =>", usos.Local.Conf.de_dentro())
print("Privada.abrir =>", classes.Privada.abrir(), "| Cofre2.abrir =>", classes.Cofre2.abrir(), "| Balde.por_ip =>", classes.Balde.por_ip())
