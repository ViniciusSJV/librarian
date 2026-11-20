use crate::retorno::{Nota, Pedido};

pub fn referencia(p: &Pedido) -> &Pedido {
    p
}

pub fn feito() -> Pedido {
    Pedido
}

pub fn misto() -> Nota {
    Nota
}
