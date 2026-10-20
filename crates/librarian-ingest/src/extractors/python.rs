//! Python adapter (rustpython-parser). Only syntactic declarations and occurrences.
//! No import resolution, dynamic dispatch, decorator evaluation or runtime behavior.
use super::{Extraction, Extractor, Occurrence};
use crate::{hash, Chunk, Diagnostic, Source, Symbol};
use rustpython_ast::{self as ast, text_size::TextRange, Ranged, Visitor};
use rustpython_parser::Parse;

pub(crate) struct Python;

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
        let mut walker = Walker::new(Some(source), bytes, offset);
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
        let mut walker = Walker::new(None, bytes, offset);
        for stmt in suite {
            walker.visit_stmt(stmt);
        }
        walker.found
    }

    fn candidate_kinds(&self) -> &'static [&'static str] {
        &["class"]
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
    source: Option<&'a Source>,
    bytes: &'a [u8],
    offset: usize,
    /// Enclosing declarations: name and whether it is a class.
    scope: Vec<(String, bool)>,
    symbols: Vec<Symbol>,
    chunks: Vec<Chunk>,
    found: Vec<Occurrence>,
}

impl<'a> Walker<'a> {
    fn new(source: Option<&'a Source>, bytes: &'a [u8], offset: usize) -> Self {
        Self { source, bytes, offset, scope: vec![], symbols: vec![], chunks: vec![], found: vec![] }
    }

    fn span(&self, range: TextRange) -> (usize, usize) {
        (usize::from(range.start()) + self.offset, usize::from(range.end()) + self.offset)
    }

    /// The parsed range starts at `def`/`class`; decorators belong to the declaration too.
    fn declare(&mut self, kind: &str, name: &str, range: TextRange, decorators: &[ast::Expr]) {
        let Some(source) = self.source else { return };
        let (mut start, end) = self.span(range);
        if let Some(first) = decorators.first() {
            let (at, _) = self.span(first.range());
            start = self.bytes[..at].iter().rposition(|&b| b == b'@').unwrap_or(start).min(start);
        }
        let id = format!("{}:{}:{}:{}", source.id, kind, start, end);
        let mut qualified: Vec<&str> = self.scope.iter().map(|(n, _)| n.as_str()).collect();
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
        for base in &node.bases {
            self.annotation(base);
        }
        self.scope.push((node.name.to_string(), true));
        self.generic_visit_stmt_class_def(node);
        self.scope.pop();
    }

    fn visit_stmt_assign(&mut self, node: ast::StmtAssign) {
        node.targets.iter().for_each(|t| self.binding(t));
        self.generic_visit_stmt_assign(node);
    }

    fn visit_stmt_ann_assign(&mut self, node: ast::StmtAnnAssign) {
        self.binding(&node.target);
        self.annotation(&node.annotation);
        self.generic_visit_stmt_ann_assign(node);
    }

    fn visit_stmt_for(&mut self, node: ast::StmtFor) {
        self.binding(&node.target);
        self.generic_visit_stmt_for(node);
    }

    fn visit_expr_call(&mut self, node: ast::ExprCall) {
        // Keep receiver and arguments; no receiver type or target is inferred.
        match node.func.as_ref() {
            ast::Expr::Attribute(_) => self.push("method_call_observed", node.range, None),
            func => self.push("call_observed", func.range(), None),
        }
        self.generic_visit_expr_call(node);
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
