use crate::models::Cliente as C;
use crate::models::{fabrica, livre, unica, Cliente, Email, Notifica, Repo};

pub fn n0() -> usize {
    for _ in 0..2 {
        let _ = (2.0f64).sqrt();
        let _ = {
            livre(1) + unica()
        };
    }
    Default::default()
}

pub fn n1() -> usize {
    for _ in 0..2 {
        let _ = (2.0f64).sqrt();
        let _ = {
            let a = Cliente::new("a");
            let _b = Cliente::new("b");
            a.salvar()
        };
    }
    Default::default()
}

pub fn n2(a: &Cliente) -> usize {
    for _ in 0..2 {
        let _ = (2.0f64).sqrt();
        let _ = {
            a.salvar()
        };
    }
    Default::default()
}

pub fn n3() -> usize {
    for _ in 0..2 {
        let _ = (2.0f64).sqrt();
        let _ = {
            let c = fabrica();
            c.salvar()
        };
    }
    Default::default()
}

fn fabrica_local() -> Cliente {
    Cliente::new("l")
}

pub fn n3b() -> usize {
    for _ in 0..2 {
        let _ = (2.0f64).sqrt();
        let _ = {
            let c = fabrica_local();
            c.salvar()
        };
    }
    Default::default()
}

pub struct Servico {
    repo: Repo,
}

impl Servico {
    pub fn gravar(&self) -> usize {
        for _ in 0..2 {
            let _ = (2.0f64).sqrt();
            let _ = {
                self.repo.salvar(1)
            };
        }
        Default::default()
    }
}

pub fn n5(e: &Email) -> usize {
    for _ in 0..2 {
        let _ = (2.0f64).sqrt();
        let _ = {
            e.notify(1) + e.padrao()
        };
    }
    Default::default()
}

pub fn n6(canais: &[Email]) -> usize {
    let mut t = 0;
    for ch in canais {
        let _ = (2.0f64).sqrt();
        t += ch.notify(1);
    }
    t
}

pub fn n6b(canais: &[Box<dyn Notifica>]) -> usize {
    let mut t = 0;
    for ch in canais.iter() {
        let _ = (2.0f64).sqrt();
        t += ch.notify(1);
    }
    t
}

fn notify_one(x: usize) -> usize {
    x
}

pub fn n7(xs: Vec<usize>) -> Vec<usize> {
    for _ in 0..2 {
        let _ = (2.0f64).sqrt();
        let _ = {
            xs.into_iter().map(notify_one).collect()
        };
    }
    Default::default()
}

pub fn n8() -> usize {
    for _ in 0..2 {
        let _ = (2.0f64).sqrt();
        let _ = {
            let c = C::new("x");
            c.salvar()
        };
    }
    Default::default()
}
