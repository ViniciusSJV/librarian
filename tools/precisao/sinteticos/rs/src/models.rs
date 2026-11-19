pub struct Cliente {
    pub nome: String,
}

impl Cliente {
    pub fn new(nome: &str) -> Self {
        Cliente { nome: nome.to_string() }
    }

    pub fn salvar(&self) -> usize {
        self.nome.len()
    }
}

pub struct Fornecedor;

impl Fornecedor {
    pub fn new() -> Self {
        Fornecedor
    }

    pub fn salvar(&self) -> usize {
        2
    }
}

pub struct Repo;

impl Repo {
    pub fn new() -> Self {
        Repo
    }

    pub fn salvar(&self, x: usize) -> usize {
        x
    }
}

pub trait Notifica {
    fn notify(&self, x: usize) -> usize;

    fn padrao(&self) -> usize {
        0
    }
}

pub struct Email;
pub struct Sms;

impl Notifica for Email {
    fn notify(&self, x: usize) -> usize {
        x
    }
}

impl Notifica for Sms {
    fn notify(&self, x: usize) -> usize {
        x + 1
    }
}

pub struct Fila;

impl Fila {
    pub fn notify(&self, x: usize) -> usize {
        x
    }

    pub fn padrao(&self) -> usize {
        9
    }
}

pub fn fabrica() -> Cliente {
    Cliente::new("a")
}

pub fn livre(x: usize) -> usize {
    x
}

pub fn unica() -> usize {
    1
}
