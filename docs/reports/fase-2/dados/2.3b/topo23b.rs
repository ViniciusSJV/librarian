// Step 2.3b, Parada 1 (diagnosis only; not production code). For every call written in a Python
// file of an edition, where it runs (function body, class body, module top level, a decorator,
// a default, a base) and which symbol the graph makes its owner today (the smallest symbol whose
// chunk spans it; none = the occurrence is dropped). JSON lines to stdout.
//
//   cargo run --release -p librarian-ingest --example topo23b -- <edition> > topo.jsonl
use librarian_ingest::Catalog;
use rustpython_ast::{self as ast, text_size::TextRange, Ranged, Visitor};
use rustpython_parser::Parse;
use serde_json::json;

#[derive(Default)]
struct Calls(Vec<(TextRange, String)>);

fn name_of(f: &ast::Expr) -> String {
    match f {
        ast::Expr::Name(n) => n.id.to_string(),
        ast::Expr::Attribute(a) => format!("{}.{}", name_of(&a.value), a.attr),
        ast::Expr::Call(c) => format!("{}(..)", name_of(&c.func)),
        _ => "?".into(),
    }
}

impl Visitor for Calls {
    // Parada 2: the generic visit does not reach keyword argument values (`side_effect=E(..)`),
    // so they are visited here. Without this, 5 calls in decorators were missed (778, not 783).
    fn visit_expr_call(&mut self, node: ast::ExprCall) {
        self.0.push((node.range, name_of(&node.func)));
        self.visit_expr(*node.func);
        for a in node.args {
            self.visit_expr(a);
        }
        for k in node.keywords {
            self.visit_expr(k.value);
        }
    }
}

fn calls_expr(e: &ast::Expr) -> Vec<(TextRange, String)> {
    let mut c = Calls::default();
    c.visit_expr(e.clone());
    c.0
}

fn calls_stmt(s: &ast::Stmt) -> Vec<(TextRange, String)> {
    let mut c = Calls::default();
    c.visit_stmt(s.clone());
    c.0
}

struct Out<'a> {
    path: &'a str,
    off: usize,
    bytes: &'a [u8],
    owners: Vec<(usize, usize, String, String)>,
    rows: Vec<serde_json::Value>,
}

impl Out<'_> {
    fn emit(&mut self, calls: Vec<(TextRange, String)>, place: &str, stmt: &str) {
        for (r, name) in calls {
            let (s, e) = (usize::from(r.start()) + self.off, usize::from(r.end()) + self.off);
            let owner = self.owners.iter().filter(|o| o.0 <= s && o.1 >= e).min_by_key(|o| o.1 - o.0);
            let line = self.bytes[..s].iter().filter(|&&b| b == b'\n').count() + 1;
            let text: String = String::from_utf8_lossy(&self.bytes[s..e]).chars().take(90).collect();
            self.rows.push(json!({
                "path": self.path, "line": line, "start": s, "end": e, "name": name,
                "text": text.replace('\n', " "), "place": place, "stmt": stmt,
                "owner_kind": owner.map(|o| o.2.clone()), "owner": owner.map(|o| o.3.clone()),
            }));
        }
    }

    fn args(&mut self, a: &ast::Arguments, ctx: &str) {
        for d in a.posonlyargs.iter().chain(&a.args).chain(&a.kwonlyargs).filter_map(|x| x.default.as_deref()) {
            self.emit(calls_expr(d), &format!("padrao_{ctx}"), "def");
        }
    }

    fn suite(&mut self, body: &[ast::Stmt], ctx: &str) {
        for s in body {
            self.stmt(s, ctx);
        }
    }

    fn deco(&mut self, decos: &[ast::Expr], ctx: &str, what: &str) {
        let place = match ctx {
            "topo" => format!("decorador_{what}_topo"),
            "corpo_classe" => format!("decorador_{what}_metodo"),
            _ => format!("decorador_{what}_aninhado"),
        };
        for d in decos {
            self.emit(calls_expr(d), &place, "def");
            // The whole decorator expression, to find every occurrence written in it.
            let (s, e) = (usize::from(d.range().start()) + self.off, usize::from(d.range().end()) + self.off);
            let line = self.bytes[..s].iter().filter(|&&b| b == b'\n').count() + 1;
            self.rows.push(json!({ "path": self.path, "line": line, "start": s, "end": e, "place": place, "stmt": "decorator_expr",
                "text": String::from_utf8_lossy(&self.bytes[s..e]).chars().take(90).collect::<String>() }));
        }
    }

    fn stmt(&mut self, s: &ast::Stmt, ctx: &str) {
        match s {
            ast::Stmt::FunctionDef(f) => {
                self.deco(&f.decorator_list, ctx, "funcao");
                self.args(&f.args, ctx);
                self.suite(&f.body, "funcao");
            }
            ast::Stmt::AsyncFunctionDef(f) => {
                self.deco(&f.decorator_list, ctx, "funcao");
                self.args(&f.args, ctx);
                self.suite(&f.body, "funcao");
            }
            ast::Stmt::ClassDef(c) => {
                self.deco(&c.decorator_list, ctx, "classe");
                for b in c.bases.iter().chain(c.keywords.iter().map(|k| &k.value)) {
                    self.emit(calls_expr(b), &format!("base_{ctx}"), "class");
                }
                self.suite(&c.body, "corpo_classe");
            }
            ast::Stmt::If(i) => {
                self.emit(calls_expr(&i.test), ctx, "if");
                self.suite(&i.body, ctx);
                self.suite(&i.orelse, ctx);
            }
            ast::Stmt::For(f) => {
                self.emit(calls_expr(&f.iter), ctx, "for");
                self.suite(&f.body, ctx);
                self.suite(&f.orelse, ctx);
            }
            ast::Stmt::While(w) => {
                self.emit(calls_expr(&w.test), ctx, "while");
                self.suite(&w.body, ctx);
                self.suite(&w.orelse, ctx);
            }
            ast::Stmt::With(w) => {
                for it in &w.items {
                    self.emit(calls_expr(&it.context_expr), ctx, "with");
                }
                self.suite(&w.body, ctx);
            }
            ast::Stmt::Try(t) => {
                self.suite(&t.body, ctx);
                for h in &t.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = h;
                    self.suite(&h.body, ctx);
                }
                self.suite(&t.orelse, ctx);
                self.suite(&t.finalbody, ctx);
            }
            other => {
                let stmt = match other {
                    ast::Stmt::Assign(a) => format!("assign:{}", a.targets.iter().map(name_of).collect::<Vec<_>>().join(",")),
                    ast::Stmt::AugAssign(a) => format!("augassign:{}", name_of(&a.target)),
                    ast::Stmt::AnnAssign(a) => format!("annassign:{}", name_of(&a.target)),
                    ast::Stmt::Expr(_) => "expr".into(),
                    ast::Stmt::Return(_) => "return".into(),
                    _ => "outro".into(),
                };
                self.emit(calls_stmt(other), ctx, &stmt);
            }
        }
    }
}

fn main() {
    let edition = std::env::args().nth(1).expect("edition");
    let catalog = Catalog::load(std::path::Path::new(&edition)).unwrap();
    for source in catalog.sources().values().filter(|s| s.language == "python") {
        let bytes = catalog.source_bytes(&source.path).unwrap().to_vec();
        let Ok(text) = String::from_utf8(bytes.clone()) else { continue };
        let off = if text.starts_with('\u{feff}') { 3 } else { 0 };
        let Ok(suite) = ast::Suite::parse(&text[off..], "<source>") else { continue };
        let owners = catalog
            .symbols()
            .values()
            .filter(|s| s.source_id == source.id)
            .map(|s| {
                let c = &catalog.chunks()[&s.chunk_id];
                (c.start_byte, c.end_byte, s.kind.clone(), s.qualified_name.clone())
            })
            .collect();
        let mut out = Out { path: &source.path, off, bytes: &bytes, owners, rows: vec![] };
        out.suite(&suite, "topo");
        for r in out.rows {
            println!("{r}");
        }
    }
}
