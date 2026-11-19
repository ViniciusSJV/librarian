use crate::models::Cliente as C;
use crate::models::{fabrica, livre, unica, Cliente, Email, Notifica, Repo};

pub fn n0() -> usize {
    livre(1) + unica()
}

pub fn n1() -> usize {
    let a = Cliente::new("a");
    let _b = Cliente::new("b");
    a.salvar()
}

pub fn n2(a: &Cliente) -> usize {
    a.salvar()
}

pub fn n3() -> usize {
    let c = fabrica();
    c.salvar()
}

fn fabrica_local() -> Cliente {
    Cliente::new("l")
}

pub fn n3b() -> usize {
    let c = fabrica_local();
    c.salvar()
}

pub struct Servico {
    repo: Repo,
}

impl Servico {
    pub fn gravar(&self) -> usize {
        self.repo.salvar(1)
    }
}

pub fn n5(e: &Email) -> usize {
    e.notify(1) + e.padrao()
}

pub fn n6(canais: &[Email]) -> usize {
    let mut t = 0;
    for ch in canais {
        t += ch.notify(1);
    }
    t
}

pub fn n6b(canais: &[Box<dyn Notifica>]) -> usize {
    let mut t = 0;
    for ch in canais.iter() {
        t += ch.notify(1);
    }
    t
}

fn notify_one(x: usize) -> usize {
    x
}

pub fn n7(xs: Vec<usize>) -> Vec<usize> {
    xs.into_iter().map(notify_one).collect()
}

pub fn n8() -> usize {
    let c = C::new("x");
    c.salvar()
}
