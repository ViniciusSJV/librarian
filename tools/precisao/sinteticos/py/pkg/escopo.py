from datetime import datetime as Dt
from os.path import join
from typing import TYPE_CHECKING, TypeAlias, overload
from pkg.models import Cliente, Fornecedor
from pkg.models import Cliente as C
from pkg.sub import reexp
from pkg import escopo_apoio

if TYPE_CHECKING:
    from pkg.escopo_apoio import so_tipo


def esc_modulo():
    return 0


@overload
def esc_sobre(x: int) -> int: ...


@overload
def esc_sobre(x: str) -> str: ...


def esc_sobre(x):
    return x


if escopo_apoio:
    def esc_cond():
        return 1
else:
    def esc_cond():
        return 2


Alias = Cliente
TipoF: TypeAlias = Fornecedor
R = Dt


def e_modulo():
    esc_modulo()


def e_fora():
    join("a", "b")


def e_local(fabrica):
    g = fabrica()
    g()


def e_overload():
    esc_sobre(1)


def e_cond():
    esc_cond()


def e_typechecking():
    so_tipo()


def e_reexport():
    reexp()


def n8_tipo(a: Alias):
    a.salvar()


def n8_local():
    D = Fornecedor
    d = D()
    d.salvar()


def n8_typealias(a: TipoF):
    a.salvar()


def n8_fora(r: R):
    r.salvar()


def n8_import():
    c = C("y")
    c.salvar()
