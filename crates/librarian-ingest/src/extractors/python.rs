//! Python adapter (rustpython-parser). Only syntactic declarations and occurrences.
//! No import resolution, dynamic dispatch, decorator evaluation or runtime behavior.
use super::{Extraction, Extractor, Occurrence};
use crate::{hash, Chunk, Diagnostic, Source, Symbol};
use rustpython_ast::{self as ast, text_size::TextRange, Ranged, Visitor};
use rustpython_parser::Parse;

/// `g6` adds fields, attribute accesses, and calls and bases as candidates (profile v3).
/// Without it, records and occurrences are exactly those of the v2 adapter.
pub(crate) struct Python {
    g6: bool,
}

pub(crate) static PYTHON: Python = Python { g6: false };
pub(crate) static PYTHON_V3: Python = Python { g6: true };

impl Extractor for Python {
    fn extract(&self, source: &Source, bytes: &[u8]) -> Extraction {
        let failure = |code: &str, message: String| Extraction {
            symbols: vec![],
            chunks: vec![],
            diagnostics: vec![Diagnostic {
                source_id: source.id.clone(),
                code: code.into(),
                message,
            }],
        };
        let text = match std::str::from_utf8(bytes) {
            Ok(text) => text,
            Err(error) => return failure("invalid_utf8", error.to_string()),
        };
        let (suite, offset) = match parse(text) {
            Ok(parsed) => parsed,
            Err(error) => return failure("python_parse_error", error.to_string()),
        };
        let mut walker = Walker::new(self.g6, Some(source), bytes, offset);
        for stmt in suite {
            walker.visit_stmt(stmt);
        }
        Extraction {
            symbols: walker.symbols,
            chunks: walker.chunks,
            diagnostics: vec![],
        }
    }

    fn observe(&self, bytes: &[u8]) -> Vec<Occurrence> {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return vec![];
        };
        let Ok((suite, offset)) = parse(text) else {
            return vec![];
        };
        let mut walker = Walker::new(self.g6, None, bytes, offset);
        for stmt in suite {
            walker.visit_stmt(stmt);
        }
        walker.found
    }

    fn candidate_kinds(&self) -> &'static [&'static str] {
        &["class"]
    }

    fn candidate_kinds_for(&self, occurrence: &str) -> &'static [&'static str] {
        match (self.g6, occurrence) {
            // Calling a class constructs it, so classes are candidates of plain calls too.
            (true, "call_observed") => &["function", "method", "class"],
            (true, "method_call_observed") => &["method", "function"],
            (true, "attribute_access") => &["field"],
            (true, "base_class") => &["class"],
            _ => self.candidate_kinds(),
        }
    }

    /// A class spans all its methods, like a Rust impl; its methods are the answers.
    fn navigation_kinds(&self) -> &'static [&'static str] {
        &["class"]
    }
}

/// The parser does not skip a UTF-8 BOM; the offset restores original bytes.
fn parse(text: &str) -> Result<(ast::Suite, usize), rustpython_parser::ParseError> {
    let offset = if text.starts_with('\u{feff}') { 3 } else { 0 };
    Ok((ast::Suite::parse(&text[offset..], "<source>")?, offset))
}

/// One pass collects declarations (when a source is given) and occurrences.
struct Walker<'a> {
    g6: bool,
    source: Option<&'a Source>,
    bytes: &'a [u8],
    offset: usize,
    /// Enclosing declarations: name and whether it is a class.
    scope: Vec<(String, bool)>,
    symbols: Vec<Symbol>,
    chunks: Vec<Chunk>,
    found: Vec<Occurrence>,
    /// Fields already declared, by qualified name: a field is declared once per class.
    fields: std::collections::BTreeSet<String>,
}

impl<'a> Walker<'a> {
    fn new(g6: bool, source: Option<&'a Source>, bytes: &'a [u8], offset: usize) -> Self {
        Self { g6, source, bytes, offset, scope: vec![], symbols: vec![], chunks: vec![], found: vec![], fields: Default::default() }
    }

    fn span(&self, range: TextRange) -> (usize, usize) {
        (usize::from(range.start()) + self.offset, usize::from(range.end()) + self.offset)
    }

    /// The parsed range starts at `def`/`class`; decorators belong to the declaration too.
    fn declare(&mut self, kind: &str, name: &str, range: TextRange, decorators: &[ast::Expr]) {
        let scope: Vec<String> = self.scope.iter().map(|(n, _)| n.clone()).collect();
        self.declare_in(&scope, kind, name, range, decorators);
    }

    fn declare_in(&mut self, scope: &[String], kind: &str, name: &str, range: TextRange, decorators: &[ast::Expr]) {
        let Some(source) = self.source else { return };
        let (mut start, end) = self.span(range);
        if let Some(first) = decorators.first() {
            let (at, _) = self.span(first.range());
            start = self.bytes[..at].iter().rposition(|&b| b == b'@').unwrap_or(start).min(start);
        }
        let id = format!("{}:{}:{}:{}", source.id, kind, start, end);
        let mut qualified: Vec<&str> = scope.iter().map(String::as_str).collect();
        qualified.push(name);
        // A naming convention observed (pytest), not evidence that a test ran.
        let is_test = name.starts_with("test") || name.starts_with("Test")
            || self.scope.iter().any(|(n, class)| *class && n.starts_with("Test"));
        self.symbols.push(Symbol {
            id: id.clone(),
            source_id: source.id.clone(),
            name: name.into(),
            qualified_name: qualified.join("."),
            kind: kind.into(),
            is_test,
            chunk_id: id.clone(),
        });
        self.chunks.push(Chunk {
            id,
            source_id: source.id.clone(),
            start_byte: start,
            end_byte: end,
            start_line: crate::line_at(self.bytes, start),
            end_line: crate::line_at(self.bytes, end.saturating_sub(1)),
            sha256: hash(&self.bytes[start..end]),
        });
    }

    fn function(&mut self, name: &str, range: TextRange, decorators: &[ast::Expr], returns: Option<&ast::Expr>) {
        let kind = if self.scope.last().is_some_and(|(_, class)| *class) { "method" } else { "function" };
        self.declare(kind, name, range, decorators);
        if let Some(returns) = returns {
            self.push("declared_return", returns.range(), None);
            self.annotation(returns);
        }
    }

    /// G6 fields: names assigned in a class body, and `self.<name>` assigned in its `__init__`.
    /// Returns whether the target declares a field, so it is not observed as an access of itself.
    fn field(&mut self, target: &ast::Expr, statement: TextRange) -> bool {
        if !self.g6 {
            return false;
        }
        let depth = self.scope.len();
        let in_class_body = self.scope.last().is_some_and(|(_, class)| *class);
        let in_init = depth >= 2 && self.scope[depth - 1].0 == "__init__" && !self.scope[depth - 1].1 && self.scope[depth - 2].1;
        let name = match target {
            ast::Expr::Name(n) if in_class_body => n.id.to_string(),
            ast::Expr::Attribute(a) if in_init && matches!(a.value.as_ref(), ast::Expr::Name(n) if n.id.as_str() == "self") => {
                a.attr.to_string()
            }
            _ => return false,
        };
        let class: Vec<String> = self.scope[..if in_init { depth - 1 } else { depth }].iter().map(|(n, _)| n.clone()).collect();
        // A later assignment of the same field is a write to it, not a second declaration.
        if self.fields.insert(format!("{}.{}", class.join("."), name)) {
            self.declare_in(&class, "field", &name, statement, &[]);
            return true;
        }
        false
    }

    fn push(&mut self, kind: &'static str, range: TextRange, candidate: Option<String>) {
        let (start, end) = self.span(range);
        self.found.push(Occurrence { kind, start, end, candidate });
    }

    /// Names written in an annotation or base list; candidates, never resolved types.
    fn annotation(&mut self, expr: &ast::Expr) {
        match expr {
            ast::Expr::Name(n) => self.push("type_reference", n.range, Some(n.id.to_string())),
            ast::Expr::Attribute(a) => self.push("type_reference", a.range, Some(a.attr.to_string())),
            ast::Expr::Subscript(s) => {
                self.annotation(&s.value);
                self.annotation(&s.slice);
            }
            ast::Expr::Tuple(t) => t.elts.iter().for_each(|e| self.annotation(e)),
            ast::Expr::List(l) => l.elts.iter().for_each(|e| self.annotation(e)),
            ast::Expr::BinOp(b) => {
                self.annotation(&b.left);
                self.annotation(&b.right);
            }
            // Forward reference written as a string, e.g. "Node".
            ast::Expr::Constant(c) => {
                if let ast::Constant::Str(text) = &c.value {
                    if !text.is_empty() && text.chars().all(|ch| ch.is_alphanumeric() || ch == '_') {
                        self.push("type_reference", c.range, Some(text.clone()));
                    }
                }
            }
            _ => {}
        }
    }

    /// Names bound by assignment targets; attributes and subscripts are not local bindings.
    fn binding(&mut self, target: &ast::Expr) {
        match target {
            ast::Expr::Name(n) => self.push("local_binding", n.range, None),
            ast::Expr::Tuple(t) => t.elts.iter().for_each(|e| self.binding(e)),
            ast::Expr::List(l) => l.elts.iter().for_each(|e| self.binding(e)),
            ast::Expr::Starred(s) => self.binding(&s.value),
            _ => {}
        }
    }
}

impl Visitor for Walker<'_> {
    fn visit_stmt_function_def(&mut self, node: ast::StmtFunctionDef) {
        self.function(&node.name, node.range, &node.decorator_list, node.returns.as_deref());
        self.scope.push((node.name.to_string(), false));
        self.generic_visit_stmt_function_def(node);
        self.scope.pop();
    }

    fn visit_stmt_async_function_def(&mut self, node: ast::StmtAsyncFunctionDef) {
        self.function(&node.name, node.range, &node.decorator_list, node.returns.as_deref());
        self.scope.push((node.name.to_string(), false));
        self.generic_visit_stmt_async_function_def(node);
        self.scope.pop();
    }

    fn visit_stmt_class_def(&mut self, node: ast::StmtClassDef) {
        self.declare("class", &node.name, node.range, &node.decorator_list);
        // G6: bases are their own occurrence kind, so inheritance is told apart from annotations.
        for base in &node.bases {
            match (self.g6, base) {
                (true, ast::Expr::Name(n)) => self.push("base_class", n.range, Some(n.id.to_string())),
                (true, ast::Expr::Attribute(a)) => self.push("base_class", a.range, Some(a.attr.to_string())),
                _ => self.annotation(base),
            }
        }
        self.scope.push((node.name.to_string(), true));
        self.generic_visit_stmt_class_def(node);
        self.scope.pop();
    }

    fn visit_stmt_assign(&mut self, node: ast::StmtAssign) {
        node.targets.iter().for_each(|t| self.binding(t));
        if !self.g6 {
            return self.generic_visit_stmt_assign(node);
        }
        // A target that declares a field is not also an access to that field. With several
        // targets (`x = y = 0`) each field spans its own name, so their records stay distinct.
        let several = node.targets.len() > 1;
        for target in node.targets {
            let range = if several { target.range() } else { node.range };
            if !self.field(&target, range) {
                self.visit_expr(target);
            }
        }
        self.visit_expr(*node.value);
    }

    fn visit_stmt_ann_assign(&mut self, node: ast::StmtAnnAssign) {
        self.binding(&node.target);
        self.annotation(&node.annotation);
        if !self.g6 {
            return self.generic_visit_stmt_ann_assign(node);
        }
        // The annotation is already a type reference; only target and value are visited.
        if !self.field(&node.target, node.range) {
            self.visit_expr(*node.target);
        }
        if let Some(value) = node.value {
            self.visit_expr(*value);
        }
    }

    fn visit_expr_attribute(&mut self, node: ast::ExprAttribute) {
        if self.g6 {
            self.push("attribute_access", node.range, Some(node.attr.to_string()));
        }
        self.generic_visit_expr_attribute(node);
    }

    fn visit_stmt_for(&mut self, node: ast::StmtFor) {
        self.binding(&node.target);
        self.generic_visit_stmt_for(node);
    }

    fn visit_expr_call(&mut self, node: ast::ExprCall) {
        // Keep receiver and arguments; no receiver type or target is inferred.
        if !self.g6 {
            match node.func.as_ref() {
                ast::Expr::Attribute(_) => self.push("method_call_observed", node.range, None),
                func => self.push("call_observed", func.range(), None),
            }
            return self.generic_visit_expr_call(node);
        }
        // G6: the written callee name is a candidate. A called attribute is a call, not also an
        // attribute access, so only its receiver is visited.
        match *node.func {
            ast::Expr::Attribute(a) => {
                self.push("method_call_observed", node.range, Some(a.attr.to_string()));
                self.visit_expr(*a.value);
            }
            func => {
                let name = match &func {
                    ast::Expr::Name(n) => Some(n.id.to_string()),
                    _ => None,
                };
                self.push("call_observed", func.range(), name);
                self.visit_expr(func);
            }
        }
        node.args.into_iter().for_each(|a| self.visit_expr(a));
        node.keywords.into_iter().for_each(|k| self.visit_keyword(k));
    }

    // The generated visitor leaves these empty; without them nested calls and defs are missed.
    fn visit_arguments(&mut self, node: ast::Arguments) {
        let with_defaults = node.posonlyargs.into_iter().chain(node.args).chain(node.kwonlyargs);
        for arg in with_defaults {
            self.visit_arg(arg.def);
            if let Some(default) = arg.default {
                self.visit_expr(*default);
            }
        }
        for arg in node.vararg.into_iter().chain(node.kwarg) {
            self.visit_arg(*arg);
        }
    }

    fn visit_arg(&mut self, node: ast::Arg) {
        if let Some(annotation) = &node.annotation {
            self.annotation(annotation);
        }
    }

    fn visit_keyword(&mut self, node: ast::Keyword) {
        self.visit_expr(node.value);
    }

    fn visit_withitem(&mut self, node: ast::WithItem) {
        if let Some(target) = &node.optional_vars {
            self.binding(target);
        }
        self.visit_expr(node.context_expr);
    }

    fn visit_match_case(&mut self, node: ast::MatchCase) {
        if let Some(guard) = node.guard {
            self.visit_expr(*guard);
        }
        for stmt in node.body {
            self.visit_stmt(stmt);
        }
    }

    fn visit_comprehension(&mut self, node: ast::Comprehension) {
        self.binding(&node.target);
        self.visit_expr(node.iter);
        for condition in node.ifs {
            self.visit_expr(condition);
        }
    }
}
