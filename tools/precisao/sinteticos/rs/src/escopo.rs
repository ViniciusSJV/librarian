use crate::models::Cliente;
type Cl = Cliente;

pub fn esc_livre() -> usize {
    1
}

pub fn e_modulo() -> usize {
    esc_livre()
}

pub fn e_bloco() -> usize {
    fn esc_bloco() -> usize {
        2
    }
    esc_bloco()
}

pub fn n8_tipo() -> usize {
    let c = Cl::new("x");
    c.salvar()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e_glob() {
        esc_livre();
    }
}

pub mod de_fora {
    use std::collections::*;

    pub fn e_glob_fora() -> usize {
        esc_livre()
    }
}
