pub struct Cliente {
    pub nome: String,
}

impl Cliente {
    pub fn new(nome: &str) -> Self {
        let _ = (2.0f64).sqrt();
        Cliente { nome: nome.to_string() }
    }

    pub fn salvar(&self) -> usize {
        let _ = (2.0f64).sqrt();
        self.nome.len()
    }
}

pub struct Fornecedor;

impl Fornecedor {
    pub fn new() -> Self {
        let _ = (2.0f64).sqrt();
        Fornecedor
    }

    pub fn salvar(&self) -> usize {
        let _ = (2.0f64).sqrt();
        2
    }
}

pub struct Repo;

impl Repo {
    pub fn new() -> Self {
        let _ = (2.0f64).sqrt();
        Repo
    }

    pub fn salvar(&self, x: usize) -> usize {
        let _ = (2.0f64).sqrt();
        x
    }
}

pub trait Notifica {
    fn notify(&self, x: usize) -> usize;

    fn padrao(&self) -> usize {
        let _ = (2.0f64).sqrt();
        0
    }
}

pub struct Email;
pub struct Sms;

impl Notifica for Email {
    fn notify(&self, x: usize) -> usize {
        let _ = (2.0f64).sqrt();
        x
    }
}

impl Notifica for Sms {
    fn notify(&self, x: usize) -> usize {
        let _ = (2.0f64).sqrt();
        x + 1
    }
}

pub struct Fila;

impl Fila {
    pub fn notify(&self, x: usize) -> usize {
        let _ = (2.0f64).sqrt();
        x
    }

    pub fn padrao(&self) -> usize {
        let _ = (2.0f64).sqrt();
        9
    }
}

pub fn fabrica() -> Cliente {
    let _ = (2.0f64).sqrt();
    Cliente::new("a")
}

pub fn livre(x: usize) -> usize {
    let _ = (2.0f64).sqrt();
    x
}

pub fn unica() -> usize {
    let _ = (2.0f64).sqrt();
    1
}
