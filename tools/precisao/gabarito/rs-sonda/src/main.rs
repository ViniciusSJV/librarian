//! Gabarito de execucao do Rust (passo 2.4e, DEC-48; Parada 1, Q4, opcao a).
//!
//!   rs-sonda <projeto> <copia>
//!
//! Copia o projeto (sem `target/` e `.git/`) e, na copia, insere no comeco do corpo de cada funcao com corpo
//! (`fn` solta, de `impl`, metodo padrao de trait, aninhada) uma sonda:
//! `let _g24e = <crate>::__g24e_sonda::entrar("<arquivo>:<linha do nome>:<nome>");`, na mesma linha do `{`
//! (as linhas nao mudam; as colunas depois do `{` mudam, e o gabarito so usa arquivo e linha).
//! `const fn` fica sem sonda (nao pode chamar a sonda). Funcoes dentro de macros nao sao vistas (o syn nao abre
//! macros). O projeto original nao muda. Nao usa o grafo do Librarian: quem mede nao depende de quem responde.
use std::fs;
use std::path::{Path, PathBuf};
use syn::visit::Visit;

struct Insercao { byte: usize, texto: String }

struct Visitor<'a> { src: &'a str, rel: &'a str, prefixo: &'a str, out: Vec<Insercao>, const_fn: Vec<String> }

impl<'a> Visitor<'a> {
    fn byte_of(&self, lc: proc_macro2::LineColumn) -> usize {
        // line 1-based, column in chars
        let mut off = 0;
        for (i, l) in self.src.split_inclusive('\n').enumerate() {
            if i + 1 == lc.line { return off + l.char_indices().nth(lc.column).map(|(b, _)| b).unwrap_or(l.len()); }
            off += l.len();
        }
        off
    }
    fn add(&mut self, sig: &syn::Signature, block: &syn::Block) {
        let linha = sig.ident.span().start().line;
        let nome = sig.ident.to_string();
        if sig.constness.is_some() { self.const_fn.push(format!("{}:{}:{}", self.rel, linha, nome)); return; }
        let abre = block.brace_token.span.open().start();
        let byte = self.byte_of(abre) + 1;
        let texto = format!(" let _g24e = {}::__g24e_sonda::entrar(\"{}:{}:{}\");", self.prefixo, self.rel, linha, nome);
        self.out.push(Insercao { byte, texto });
    }
}

impl<'a, 'ast> Visit<'ast> for Visitor<'a> {
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) { self.add(&f.sig, &f.block); syn::visit::visit_item_fn(self, f); }
    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) { self.add(&f.sig, &f.block); syn::visit::visit_impl_item_fn(self, f); }
    fn visit_trait_item_fn(&mut self, f: &'ast syn::TraitItemFn) {
        if let Some(b) = &f.default { self.add(&f.sig, b); }
        syn::visit::visit_trait_item_fn(self, f);
    }
}

fn copiar(de: &Path, para: &Path) -> std::io::Result<()> {
    fs::create_dir_all(para)?;
    for e in fs::read_dir(de)? {
        let e = e?;
        let nome = e.file_name();
        if nome == "target" || nome == ".git" { continue; }
        let (a, b) = (e.path(), para.join(&nome));
        if e.file_type()?.is_dir() { copiar(&a, &b)?; } else { fs::copy(&a, &b)?; }
    }
    Ok(())
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() { rs_files(&p, out); } else if p.extension().is_some_and(|x| x == "rs") { out.push(p); }
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 3 { eprintln!("uso: rs-sonda <projeto> <copia>"); std::process::exit(2); }
    let (orig, copia) = (PathBuf::from(&a[1]), PathBuf::from(&a[2]));
    if copia.exists() { fs::remove_dir_all(&copia).expect("limpar a copia"); }
    copiar(&orig, &copia).expect("copiar o projeto");
    let manifest = fs::read_to_string(copia.join("Cargo.toml")).expect("Cargo.toml");
    let krate = manifest.lines().find_map(|l| l.trim().strip_prefix("name").map(|r| r.trim_start_matches([' ', '=']).trim().trim_matches('"').replace('-', "_"))).expect("nome do pacote");
    let mut files = vec![];
    rs_files(&copia.join("src"), &mut files);
    rs_files(&copia.join("tests"), &mut files);
    let (mut total, mut consts) = (0, vec![]);
    for f in &files {
        let rel = f.strip_prefix(&copia).unwrap().to_string_lossy().replace('\\', "/");
        let src = fs::read_to_string(f).expect("ler");
        let ast = match syn::parse_file(&src) { Ok(x) => x, Err(e) => { eprintln!("sem sonda (nao analisa): {rel}: {e}"); continue; } };
        let lib_mod = rel.starts_with("src/") && !rel.starts_with("src/bin/") && rel != "src/main.rs";
        let prefixo = if lib_mod { "crate".to_string() } else { krate.clone() };
        let mut v = Visitor { src: &src, rel: &rel, prefixo: &prefixo, out: vec![], const_fn: vec![] };
        v.visit_file(&ast);
        let mut ins = std::mem::take(&mut v.out);
        consts.extend(std::mem::take(&mut v.const_fn));
        ins.sort_by_key(|i| std::cmp::Reverse(i.byte));
        let mut s = src.clone();
        for i in &ins { s.insert_str(i.byte, &i.texto); }
        total += ins.len();
        fs::write(f, s).expect("gravar");
    }
    let lib = copia.join("src/lib.rs");
    let mut l = fs::read_to_string(&lib).expect("src/lib.rs");
    l.push_str("\n#[doc(hidden)]\npub mod __g24e_sonda;\n");
    fs::write(&lib, l).expect("gravar lib.rs");
    fs::write(copia.join("src/__g24e_sonda.rs"), include_str!("sonda.rs")).expect("gravar a sonda");
    println!("rs-sonda: {} arquivos, {} funcoes com sonda, {} const fn sem sonda{}", files.len(), total, consts.len(),
        if consts.is_empty() { String::new() } else { format!(": {}", consts.join(", ")) });
}
