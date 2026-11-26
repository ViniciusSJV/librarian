// Inserido pelo rs-sonda na copia do projeto (passo 2.4e). Nao existe no projeto original.
// Cada funcao instrumentada chama `entrar` ao comecar: a pilha por thread da o chamador (o topo), e cada par
// (chamador, chamado) novo vai uma vez para o arquivo de G24E_SAIDA. Sem chamador (pilha vazia): "?".
use std::cell::RefCell;
use std::collections::HashSet;
use std::io::Write;
use std::sync::Mutex;

thread_local! {
    static PILHA: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    static VISTOS: RefCell<HashSet<(&'static str, &'static str)>> = RefCell::new(HashSet::new());
}
static GLOBAL: Mutex<Option<HashSet<(String, String)>>> = Mutex::new(None);

pub struct Guarda;

impl Drop for Guarda {
    fn drop(&mut self) {
        let _ = PILHA.try_with(|p| { p.borrow_mut().pop(); });
    }
}

pub fn entrar(chamado: &'static str) -> Guarda {
    let chamador = PILHA.with(|p| p.borrow().last().copied()).unwrap_or("?");
    let novo = VISTOS.with(|v| v.borrow_mut().insert((chamador, chamado)));
    if novo {
        let mut g = GLOBAL.lock().unwrap_or_else(|e| e.into_inner());
        let set = g.get_or_insert_with(HashSet::new);
        if set.insert((chamador.to_string(), chamado.to_string())) {
            if let Ok(saida) = std::env::var("G24E_SAIDA") {
                if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(saida) {
                    let fio = std::thread::current().name().unwrap_or("").replace('"', "'");
                    let exe = std::env::current_exe().ok().and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned())).unwrap_or_default();
                    let _ = writeln!(f, "{{\"chamador\":\"{}\",\"chamado\":\"{}\",\"fio\":\"{}\",\"binario\":\"{}\"}}", chamador, chamado, fio, exe);
                }
            }
        }
    }
    PILHA.with(|p| p.borrow_mut().push(chamado));
    Guarda
}
