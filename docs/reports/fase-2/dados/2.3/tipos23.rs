// Step 2.3, Parada 1 (diagnosis only; not production code). Reads a Python edition and writes, for
// every method call, what the proposed 2.3a rules read as the receiver's written type, and the
// facts the other fronts are counted from: calls in decorators and defaults, receivers bound to a
// call's result (N3), `module.Class()` calls. Writes JSON lines to stdout; reads nothing else.
//
//   cargo run --release -p librarian-ingest --example tipos23 -- <edition> > tipos.jsonl
use librarian_ingest::Catalog;
use rustpython_ast::{self as ast, text_size::TextRange, Ranged, Visitor};
use rustpython_parser::Parse;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

/// What a binding of a name (or a field write) gives, as written.
#[derive(Clone, Debug, PartialEq)]
enum Val {
    /// An annotation: the type name it names (last segment), and the head name written (for imports).
    Ann(String, String),
    /// `T(...)` or `m.T(...)`: the name called, its head, and the call's occurrence key.
    Call(String, String, (usize, usize), bool),
    /// The method's own receiver.
    SelfOf(String),
    None,
    /// The value of a name that has a type in the same function (`self.x = repo`).
    Name(String),
    Unknown,
}

fn span(r: TextRange, off: usize) -> (usize, usize) {
    (usize::from(r.start()) + off, usize::from(r.end()) + off)
}

/// The type an annotation names: `T`, `m.T`, `"T"`, `T | None`, `Optional[T]`, `Union[T, None]`,
/// `Annotated[T, ..]`. None for `Any`, unions of two types, collections and `type[T]`.
fn ann_type(e: &ast::Expr) -> Option<(String, String)> {
    match e {
        ast::Expr::Name(n) => (n.id.as_str() != "None").then(|| (n.id.to_string(), n.id.to_string())),
        ast::Expr::Attribute(a) => Some((a.attr.to_string(), head(&a.value)?)),
        ast::Expr::Constant(c) => match &c.value {
            ast::Constant::Str(s) => {
                let parsed = ast::Expr::parse(s, "<ann>").ok()?;
                ann_type(&parsed)
            }
            _ => None,
        },
        ast::Expr::BinOp(b) if matches!(b.op, ast::Operator::BitOr) => {
            let none = |x: &ast::Expr| matches!(x, ast::Expr::Constant(c) if matches!(c.value, ast::Constant::None)) || matches!(x, ast::Expr::Name(n) if n.id.as_str() == "None");
            if none(&b.right) {
                ann_type(&b.left)
            } else if none(&b.left) {
                ann_type(&b.right)
            } else {
                None
            }
        }
        ast::Expr::Subscript(s) => {
            let name = match s.value.as_ref() {
                ast::Expr::Name(n) => n.id.to_string(),
                ast::Expr::Attribute(a) => a.attr.to_string(),
                _ => return None,
            };
            match name.as_str() {
                "Optional" => ann_type(&s.slice),
                "Annotated" => match s.slice.as_ref() {
                    ast::Expr::Tuple(t) => t.elts.first().and_then(ann_type),
                    x => ann_type(x),
                },
                "Union" => match s.slice.as_ref() {
                    ast::Expr::Tuple(t) => {
                        let rest: Vec<&ast::Expr> = t.elts.iter().filter(|x| !matches!(x, ast::Expr::Constant(c) if matches!(c.value, ast::Constant::None)) && !matches!(x, ast::Expr::Name(n) if n.id.as_str() == "None")).collect();
                        if rest.len() == 1 { ann_type(rest[0]) } else { None }
                    }
                    _ => None,
                },
                _ => None,
            }
        }
        _ => None,
    }
}

fn head(e: &ast::Expr) -> Option<String> {
    match e {
        ast::Expr::Name(n) => Some(n.id.to_string()),
        ast::Expr::Attribute(a) => head(&a.value),
        _ => None,
    }
}

fn value(e: &ast::Expr, off: usize) -> Val {
    match e {
        ast::Expr::Constant(c) if matches!(c.value, ast::Constant::None) => Val::None,
        ast::Expr::Name(n) => Val::Name(n.id.to_string()),
        ast::Expr::Call(c) => match c.func.as_ref() {
            ast::Expr::Name(n) => Val::Call(n.id.to_string(), n.id.to_string(), span(c.func.range(), off), false),
            ast::Expr::Attribute(a) => match head(&a.value) {
                Some(h) => Val::Call(a.attr.to_string(), h, span(c.range, off), true),
                None => Val::Unknown,
            },
            _ => Val::Unknown,
        },
        _ => Val::Unknown,
    }
}

/// The names a function binds in its own body (not in nested scopes), with what each binding gives.
#[derive(Default, Debug)]
struct Binds {
    vals: BTreeMap<String, Vec<Val>>,
    outer: BTreeSet<String>,
}

fn targets(t: &ast::Expr, v: Val, b: &mut Binds) {
    match t {
        ast::Expr::Name(n) => b.vals.entry(n.id.to_string()).or_default().push(v),
        ast::Expr::Tuple(x) => x.elts.iter().for_each(|e| targets(e, Val::Unknown, b)),
        ast::Expr::List(x) => x.elts.iter().for_each(|e| targets(e, Val::Unknown, b)),
        ast::Expr::Starred(s) => targets(&s.value, Val::Unknown, b),
        _ => {}
    }
}

fn collect(body: &[ast::Stmt], off: usize, b: &mut Binds) {
    for s in body {
        match s {
            ast::Stmt::Assign(a) => a.targets.iter().for_each(|t| targets(t, value(&a.value, off), b)),
            ast::Stmt::AnnAssign(a) => {
                let v = ann_type(&a.annotation).map_or(Val::Unknown, |(t, h)| Val::Ann(t, h));
                targets(&a.target, v, b)
            }
            ast::Stmt::AugAssign(a) => targets(&a.target, Val::Unknown, b),
            ast::Stmt::For(f) => {
                targets(&f.target, Val::Unknown, b);
                collect(&f.body, off, b);
                collect(&f.orelse, off, b);
            }
            ast::Stmt::AsyncFor(f) => {
                targets(&f.target, Val::Unknown, b);
                collect(&f.body, off, b);
                collect(&f.orelse, off, b);
            }
            ast::Stmt::While(w) => {
                collect(&w.body, off, b);
                collect(&w.orelse, off, b);
            }
            ast::Stmt::If(i) => {
                collect(&i.body, off, b);
                collect(&i.orelse, off, b);
            }
            ast::Stmt::With(w) => {
                w.items.iter().filter_map(|i| i.optional_vars.as_deref()).for_each(|t| targets(t, Val::Unknown, b));
                collect(&w.body, off, b);
            }
            ast::Stmt::AsyncWith(w) => {
                w.items.iter().filter_map(|i| i.optional_vars.as_deref()).for_each(|t| targets(t, Val::Unknown, b));
                collect(&w.body, off, b);
            }
            ast::Stmt::Try(t) => {
                collect(&t.body, off, b);
                for h in &t.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = h;
                    if let Some(n) = &h.name {
                        b.vals.entry(n.to_string()).or_default().push(Val::Unknown);
                    }
                    collect(&h.body, off, b);
                }
                collect(&t.orelse, off, b);
                collect(&t.finalbody, off, b);
            }
            ast::Stmt::TryStar(t) => {
                collect(&t.body, off, b);
                for h in &t.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = h;
                    collect(&h.body, off, b);
                }
                collect(&t.orelse, off, b);
                collect(&t.finalbody, off, b);
            }
            ast::Stmt::Match(m) => {
                // Capture names are not read: any name in a case pattern is treated as unknown.
                for c in &m.cases {
                    collect(&c.body, off, b);
                }
                b.vals.entry("__match__".into()).or_default().push(Val::Unknown);
            }
            ast::Stmt::Import(i) => i.names.iter().for_each(|a| {
                let n = a.asname.as_ref().map_or_else(|| a.name.split('.').next().unwrap_or_default().to_owned(), |x| x.to_string());
                b.vals.entry(n).or_default().push(Val::Unknown)
            }),
            ast::Stmt::ImportFrom(i) => i.names.iter().for_each(|a| {
                b.vals.entry(a.asname.as_ref().unwrap_or(&a.name).to_string()).or_default().push(Val::Unknown)
            }),
            ast::Stmt::Global(g) => b.outer.extend(g.names.iter().map(|n| n.to_string())),
            ast::Stmt::Nonlocal(g) => b.outer.extend(g.names.iter().map(|n| n.to_string())),
            ast::Stmt::FunctionDef(f) => b.vals.entry(f.name.to_string()).or_default().push(Val::Unknown),
            ast::Stmt::AsyncFunctionDef(f) => b.vals.entry(f.name.to_string()).or_default().push(Val::Unknown),
            ast::Stmt::ClassDef(c) => b.vals.entry(c.name.to_string()).or_default().push(Val::Unknown),
            _ => {}
        }
    }
}

/// One function's scope: its parameters and bindings.
struct Scope {
    binds: Binds,
    /// The class a method belongs to, for `self`.
    class: Option<String>,
}

/// A class's fields as the proposed rules read them, by class name (several classes may share it).
#[derive(Default, Debug)]
struct Fields {
    /// class name -> field -> values written.
    writes: BTreeMap<String, BTreeMap<String, Vec<Val>>>,
    /// class name -> whether any class of that name has a written base other than `object`.
    bases: BTreeMap<String, bool>,
}

fn is_self(e: &ast::Expr) -> bool {
    matches!(e, ast::Expr::Name(n) if n.id.as_str() == "self")
}

/// Pass 1: field writes of every class (`self.x = ..` in its methods, `self.x: T` and `x: T` in the body).
fn fields_of(suite: &[ast::Stmt], off: usize, out: &mut Fields) {
    for s in suite {
        if let ast::Stmt::ClassDef(c) = s {
            let name = c.name.to_string();
            let based = c.bases.iter().any(|b| !matches!(b, ast::Expr::Name(n) if n.id.as_str() == "object"));
            *out.bases.entry(name.clone()).or_default() |= based;
            let w = out.writes.entry(name.clone()).or_default();
            for item in &c.body {
                match item {
                    ast::Stmt::AnnAssign(a) => {
                        if let (ast::Expr::Name(n), Some((t, h))) = (a.target.as_ref(), ann_type(&a.annotation)) {
                            let classvar = matches!(a.annotation.as_ref(), ast::Expr::Subscript(s) if matches!(s.value.as_ref(), ast::Expr::Name(n) if n.id.as_str() == "ClassVar"));
                            if !classvar {
                                w.entry(n.id.to_string()).or_default().push(Val::Ann(t, h));
                            }
                        }
                    }
                    ast::Stmt::FunctionDef(f) => method_writes(&f.name, &f.args, &f.body, off, w),
                    ast::Stmt::AsyncFunctionDef(f) => method_writes(&f.name, &f.args, &f.body, off, w),
                    _ => {}
                }
            }
            fields_of(&c.body, off, out);
        }
        // Nested classes inside functions are rare; not followed.
    }
}

fn method_writes(name: &str, args: &ast::Arguments, body: &[ast::Stmt], off: usize, w: &mut BTreeMap<String, Vec<Val>>) {
    // Parameter annotations of the method, for `self.x = param`.
    let anns: BTreeMap<String, (String, String)> = args.posonlyargs.iter().chain(&args.args).chain(&args.kwonlyargs)
        .filter_map(|a| Some((a.def.arg.to_string(), ann_type(a.def.annotation.as_deref()?)?))).collect();
    let mut stack: Vec<&ast::Stmt> = body.iter().collect();
    while let Some(s) = stack.pop() {
        match s {
            ast::Stmt::Assign(a) => {
                for t in &a.targets {
                    if let ast::Expr::Attribute(at) = t {
                        if is_self(&at.value) {
                            let mut v = value(&a.value, off);
                            if let Val::Name(n) = &v {
                                v = anns.get(n).map_or(Val::Unknown, |(t, h)| Val::Ann(t.clone(), h.clone()));
                            }
                            // Only `__init__` may give a type by constructor or parameter; elsewhere a write is a conflict unless it is None.
                            if name != "__init__" && !matches!(v, Val::None) {
                                v = match v { Val::Call(..) | Val::Ann(..) => Val::Unknown, x => x };
                                if matches!(v, Val::Unknown) {
                                    v = Val::Unknown;
                                }
                            }
                            w.entry(at.attr.to_string()).or_default().push(v);
                        }
                    }
                }
            }
            ast::Stmt::AnnAssign(a) => {
                if let ast::Expr::Attribute(at) = a.target.as_ref() {
                    if is_self(&at.value) {
                        if let Some((t, h)) = ann_type(&a.annotation) {
                            w.entry(at.attr.to_string()).or_default().push(Val::Ann(t, h));
                        }
                    }
                }
            }
            ast::Stmt::AugAssign(a) => {
                if let ast::Expr::Attribute(at) = a.target.as_ref() {
                    if is_self(&at.value) {
                        w.entry(at.attr.to_string()).or_default().push(Val::Unknown);
                    }
                }
            }
            ast::Stmt::If(i) => stack.extend(i.body.iter().chain(&i.orelse)),
            ast::Stmt::For(f) => stack.extend(f.body.iter().chain(&f.orelse)),
            ast::Stmt::While(f) => stack.extend(f.body.iter().chain(&f.orelse)),
            ast::Stmt::With(f) => stack.extend(f.body.iter()),
            ast::Stmt::Try(t) => {
                stack.extend(t.body.iter().chain(&t.orelse).chain(&t.finalbody));
                for h in &t.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = h;
                    stack.extend(h.body.iter());
                }
            }
            _ => {}
        }
    }
}

/// What a set of bindings gives: an annotation wins; otherwise all agree (None ignored).
fn settle(vals: &[Val]) -> Option<(Val, &'static str)> {
    let anns: Vec<&Val> = vals.iter().filter(|v| matches!(v, Val::Ann(..))).collect();
    if !anns.is_empty() {
        return anns.iter().all(|a| a == &anns[0]).then(|| (anns[0].clone(), "anotacao"));
    }
    let rest: Vec<&Val> = vals.iter().filter(|v| !matches!(v, Val::None)).collect();
    let first = rest.first()?;
    if !rest.iter().all(|v| match (v, first) {
        (Val::Call(a, ha, _, ma), Val::Call(b, hb, _, mb)) => a == b && ha == hb && ma == mb,
        (a, b) => a == b,
    }) {
        return None;
    }
    match first {
        Val::Call(..) => Some(((*first).clone(), if rest.len() > 1 { "construtor*" } else { "construtor" })),
        Val::SelfOf(_) => Some(((*first).clone(), "self")),
        _ => None,
    }
}

struct Walk<'a> {
    path: &'a str,
    off: usize,
    bytes: &'a [u8],
    scopes: Vec<Scope>,
    module: Binds,
    classes: Vec<String>,
    fields: &'a Fields,
    /// Depth inside decorators and defaults of a definition: (kind of the definition).
    deco: Vec<&'static str>,
    out: Vec<serde_json::Value>,
    class_depth: Vec<usize>,
}

impl Walk<'_> {
    fn text(&self, s: (usize, usize)) -> String {
        String::from_utf8_lossy(&self.bytes[s.0..s.1]).lines().next().unwrap_or_default().to_owned()
    }

    /// The binding of a plain name, from the innermost function that binds it, then the module.
    fn name_type(&self, name: &str) -> Option<(Val, &'static str)> {
        for s in self.scopes.iter().rev() {
            if s.binds.outer.contains(name) {
                break;
            }
            if let Some(v) = s.binds.vals.get(name) {
                return settle(v);
            }
        }
        self.module.vals.get(name).and_then(|v| settle(v)).map(|(v, r)| (v, if r == "anotacao" { "anotacao" } else { "modulo" }))
    }

    /// The class name a receiver expression is read as, the rule, and the head name of the type written.
    fn type_of(&self, e: &ast::Expr) -> Option<(String, String, &'static str, Option<(usize, usize, bool)>)> {
        match e {
            ast::Expr::Name(n) => {
                let (v, rule) = self.name_type(n.id.as_str())?;
                match v {
                    Val::Ann(t, h) => Some((t, h, rule, None)),
                    Val::Call(t, h, k, m) => Some((t, h, rule, Some((k.0, k.1, m)))),
                    Val::SelfOf(c) => Some((c.clone(), c, "self", None)),
                    _ => None,
                }
            }
            ast::Expr::Attribute(a) => {
                let (base, _, _, _) = self.type_of(&a.value)?;
                let w = self.fields.writes.get(&base)?.get(a.attr.as_str())?;
                let (v, rule) = settle(w)?;
                let rule = if rule == "anotacao" { "campo_anotacao" } else { "campo_construtor" };
                match v {
                    Val::Ann(t, h) => Some((t, h, rule, None)),
                    Val::Call(t, h, k, m) => Some((t, h, rule, Some((k.0, k.1, m)))),
                    _ => None,
                }
            }
            ast::Expr::Call(c) => match value(e, self.off) {
                Val::Call(t, h, k, m) => {
                    let _ = c;
                    Some((t, h, "construtor_direto", Some((k.0, k.1, m))))
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn function(&mut self, name: &str, range: TextRange, args: &ast::Arguments, body: &[ast::Stmt], decorators: &[ast::Expr], kind: &'static str) {
        self.deco.push(if kind == "method" { "decorador:method" } else { "decorador:function" });
        for d in decorators {
            self.visit_expr(d.clone());
        }
        self.deco.pop();
        self.deco.push(if kind == "method" { "default:method" } else { "default:function" });
        for a in args.posonlyargs.iter().chain(&args.args).chain(&args.kwonlyargs) {
            if let Some(d) = &a.default {
                self.visit_expr(*d.clone());
            }
        }
        self.deco.pop();
        let _ = range;
        let mut binds = Binds::default();
        let in_class = kind == "method";
        let staticm = decorators.iter().any(|d| matches!(d, ast::Expr::Name(n) if n.id.as_str() == "staticmethod" || n.id.as_str() == "classmethod"));
        let params: Vec<&ast::ArgWithDefault> = args.posonlyargs.iter().chain(&args.args).chain(&args.kwonlyargs).collect();
        for (i, a) in params.iter().enumerate() {
            let v = match a.def.annotation.as_deref().and_then(ann_type) {
                Some((t, h)) => Val::Ann(t, h),
                None if i == 0 && in_class && !staticm => Val::SelfOf(self.classes.last().cloned().unwrap_or_default()),
                None => Val::Unknown,
            };
            binds.vals.entry(a.def.arg.to_string()).or_default().push(v);
        }
        for a in args.vararg.iter().chain(&args.kwarg) {
            binds.vals.entry(a.arg.to_string()).or_default().push(Val::Unknown);
        }
        collect(body, self.off, &mut binds);
        let _ = name;
        self.scopes.push(Scope { binds, class: if kind == "method" { self.classes.last().cloned() } else { None } });
        let saved = std::mem::take(&mut self.classes);
        for s in body {
            self.visit_stmt(s.clone());
        }
        self.classes = saved;
        self.scopes.pop();
    }
}

impl Visitor for Walk<'_> {
    fn visit_stmt_function_def(&mut self, node: ast::StmtFunctionDef) {
        let kind = if !self.classes.is_empty() && self.class_body() { "method" } else { "function" };
        self.function(&node.name, node.range, &node.args, &node.body, &node.decorator_list, kind);
    }
    fn visit_stmt_async_function_def(&mut self, node: ast::StmtAsyncFunctionDef) {
        let kind = if !self.classes.is_empty() && self.class_body() { "method" } else { "function" };
        self.function(&node.name, node.range, &node.args, &node.body, &node.decorator_list, kind);
    }
    fn visit_stmt_class_def(&mut self, node: ast::StmtClassDef) {
        self.deco.push("decorador:class");
        for d in node.decorator_list {
            self.visit_expr(d);
        }
        self.deco.pop();
        for b in node.bases {
            self.visit_expr(b);
        }
        self.classes.push(node.name.to_string());
        self.class_depth.push(self.scopes.len());
        for s in node.body {
            self.visit_stmt(s);
        }
        self.class_depth.pop();
        self.classes.pop();
    }
    fn visit_expr_call(&mut self, node: ast::ExprCall) {
        let deco = self.deco.last().copied();
        let key = match node.func.as_ref() {
            ast::Expr::Attribute(_) => span(node.range, self.off),
            f => span(f.range(), self.off),
        };
        let mut rec = json!({ "path": self.path, "sb": key.0, "eb": key.1, "text": self.text(key), "method": matches!(node.func.as_ref(), ast::Expr::Attribute(_)) });
        if let Some(d) = deco {
            rec["decorator_or_default"] = json!(d);
        }
        if let ast::Expr::Attribute(a) = node.func.as_ref() {
            let r = span(a.value.range(), self.off);
            rec["recv"] = json!([r.0, r.1]);
            rec["head"] = json!(head(&a.value));
            rec["name"] = json!(a.attr.to_string());
            if let Some((t, h, rule, via)) = self.type_of(&a.value) {
                rec["type"] = json!(t);
                rec["type_head"] = json!(h);
                rec["rule"] = json!(rule);
                if let Some(v) = via {
                    rec["via_call"] = json!([v.0, v.1, v.2]);
                }
            }
        }
        self.out.push(rec);
        self.generic_visit_expr_call(node);
    }
    fn visit_arguments(&mut self, node: ast::Arguments) {
        for a in node.posonlyargs.into_iter().chain(node.args).chain(node.kwonlyargs) {
            if let Some(d) = a.default {
                self.visit_expr(*d);
            }
        }
    }
    fn visit_expr_lambda(&mut self, node: ast::ExprLambda) {
        // A lambda's parameters shadow; read as an empty scope with unknown params.
        let mut binds = Binds::default();
        for a in node.args.posonlyargs.iter().chain(&node.args.args).chain(&node.args.kwonlyargs) {
            binds.vals.entry(a.def.arg.to_string()).or_default().push(Val::Unknown);
        }
        self.scopes.push(Scope { binds, class: None });
        self.visit_expr(*node.body);
        self.scopes.pop();
    }
    fn visit_comprehension(&mut self, node: ast::Comprehension) {
        self.visit_expr(node.target);
        self.visit_expr(node.iter);
        for c in node.ifs {
            self.visit_expr(c);
        }
    }
    fn visit_withitem(&mut self, node: ast::WithItem) {
        self.visit_expr(node.context_expr);
        if let Some(t) = node.optional_vars {
            self.visit_expr(*t);
        }
    }
    fn visit_match_case(&mut self, node: ast::MatchCase) {
        if let Some(g) = node.guard {
            self.visit_expr(*g);
        }
        for s in node.body {
            self.visit_stmt(s);
        }
    }
    fn visit_keyword(&mut self, node: ast::Keyword) {
        self.visit_expr(node.value);
    }
}

// The class body is the innermost scope when no function was entered after the class.
trait ClassBody {
    fn class_body(&self) -> bool;
}
impl ClassBody for Walk<'_> {
    fn class_body(&self) -> bool {
        self.class_depth.last().is_some_and(|d| *d == self.scopes.len())
    }
}

fn main() {
    let edition = std::env::args().nth(1).expect("edition");
    let catalog = Catalog::load(std::path::Path::new(&edition)).unwrap();
    let mut parsed = vec![];
    for source in catalog.sources().values().filter(|s| s.language == "python") {
        let bytes = catalog.source_bytes(&source.path).unwrap().to_vec();
        let Ok(text) = String::from_utf8(bytes.clone()) else { continue };
        let off = if text.starts_with('\u{feff}') { 3 } else { 0 };
        let Ok(suite) = ast::Suite::parse(&text[off..], "<source>") else { continue };
        parsed.push((source.path.clone(), bytes, off, suite));
    }
    let mut fields = Fields::default();
    for (_, _, off, suite) in &parsed {
        fields_of(suite, *off, &mut fields);
    }
    for (path, bytes, off, suite) in &parsed {
        let mut module = Binds::default();
        collect(suite, *off, &mut module);
        let mut w = Walk { path, off: *off, bytes, scopes: vec![], module, classes: vec![], fields: &fields, deco: vec![], out: vec![], class_depth: vec![] };
        for s in suite.clone() {
            w.visit_stmt(s);
        }
        for r in w.out {
            println!("{r}");
        }
    }
    eprintln!("{} arquivos; {} classes", parsed.len(), fields.writes.len());
    // Classes with a base, for the rule that a type with a base never rules a candidate out.
    let based: Vec<&String> = fields.bases.iter().filter(|(_, b)| **b).map(|(n, _)| n).collect();
    std::fs::write(std::env::var("TIPOS_BASES").unwrap_or("bases.json".into()), serde_json::to_string(&based).unwrap()).unwrap();
}
