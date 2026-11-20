use crate::apoio::referencia;

pub struct Pedido;
pub struct Nota;

impl Pedido {
    pub fn abrir() -> Self {
        Pedido
    }
    pub fn nota_nova() -> Nota {
        Nota
    }
    pub fn talvez() -> Option<Self> {
        None
    }
    pub fn caixa() -> Box<Self> {
        Box::new(Pedido)
    }
    pub fn tentar() -> Result<Self, String> {
        Ok(Pedido)
    }
    pub fn texto() -> String {
        String::new()
    }
    pub fn fechar(&self) -> usize {
        1
    }
    pub fn take(&self) -> usize {
        1
    }
    pub fn tamanho(&self) -> usize {
        1
    }
}

impl Nota {
    pub fn fechar(&self) -> usize {
        2
    }
    pub fn take(&self) -> usize {
        2
    }
}

pub fn r_self() -> usize {
    let p = Pedido::abrir();
    p.fechar()
}

pub fn r_outro() -> usize {
    let n = Pedido::nota_nova();
    n.fechar()
}

pub fn r_option() -> usize {
    let mut p = Pedido::talvez();
    p.take();
    0
}

pub fn r_box() -> usize {
    let b = Pedido::caixa();
    b.fechar()
}

pub fn r_ref() -> usize {
    let p = Pedido;
    let r = referencia(&p);
    r.fechar()
}

pub fn r_fora() -> usize {
    let s = Pedido::texto();
    s.tamanho()
}

pub fn feito() -> Pedido {
    Pedido
}

pub fn misto() -> Pedido {
    Pedido
}

pub fn r_mesmo_retorno() -> usize {
    let p = feito();
    p.fechar()
}

pub fn r_discordam() -> usize {
    let p = misto();
    p.fechar()
}

pub fn r_q() -> Result<usize, String> {
    let p = Pedido::tentar()?;
    Ok(p.fechar())
}
