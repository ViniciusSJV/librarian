//! Rust adapter (syn). Only syntactic declarations and occurrences.
//! No macro expansion, cfg evaluation or name resolution.
use super::{Extraction, Extractor, Occurrence};
use crate::{hash, Chunk, Diagnostic, Source, Symbol};
use proc_macro2::Span;
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
};

/// `g6` adds named struct fields, field accesses and call candidates (profile v3).
/// Without it, records and occurrences are exactly those of the frozen v1 adapter.
pub(crate) struct Rust {
    g6: bool,
}

pub(crate) static RUST: Rust = Rust { g6: false };
pub(crate) static RUST_V3: Rust = Rust { g6: true };

impl Extractor for Rust {
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
        let (tree, offset) = match parse(text) {
            Ok(parsed) => parsed,
            Err(error) => return failure("rust_parse_error", error.to_string()),
        };
        let mut visitor = Declarations {
            g6: self.g6,
            source,
            bytes,
            offset,
            scope: vec![],
            symbols: vec![],
            chunks: vec![],
        };
        visitor.visit_file(&tree);
        Extraction {
            symbols: visitor.symbols,
            chunks: visitor.chunks,
            diagnostics: vec![],
        }
    }

    fn observe(&self, bytes: &[u8]) -> Vec<Occurrence> {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return vec![];
        };
        let Ok((tree, offset)) = parse(text) else {
            return vec![];
        };
        let mut observer = Occurrences {
            g6: self.g6,
            offset,
            found: vec![],
        };
        observer.visit_file(&tree);
        observer.found
    }

    fn candidate_kinds(&self) -> &'static [&'static str] {
        &["struct", "enum", "trait", "type", "union"]
    }

    fn candidate_kinds_for(&self, occurrence: &str) -> &'static [&'static str] {
        match (self.g6, occurrence) {
            (true, "call_observed" | "method_call_observed") => &["function", "method", "trait_method"],
            (true, "attribute_access") => &["field"],
            _ => self.candidate_kinds(),
        }
    }

    fn navigation_kinds(&self) -> &'static [&'static str] {
        &["module", "impl"]
    }
}

/// parse_file strips a UTF-8 BOM and shebang before parsing; the offset restores original bytes.
fn parse(text: &str) -> syn::Result<(syn::File, usize)> {
    let tree = syn::parse_file(text)?;
    let offset = if text.starts_with('\u{feff}') { 3 } else { 0 }
        + tree.shebang.as_ref().map_or(0, String::len);
    Ok((tree, offset))
}

struct Declarations<'a> {
    g6: bool,
    source: &'a Source,
    bytes: &'a [u8],
    offset: usize,
    scope: Vec<String>,
    symbols: Vec<Symbol>,
    chunks: Vec<Chunk>,
}

impl Declarations<'_> {
    fn add(&mut self, kind: &str, name: String, span: Span, attrs: &[syn::Attribute]) {
        let parsed_range = span.byte_range();
        let range = (parsed_range.start + self.offset)..(parsed_range.end + self.offset);
        let id = format!("{}:{}:{}:{}", self.source.id, kind, range.start, range.end);
        let mut qualified = self.scope.clone();
        qualified.push(name.clone());
        self.symbols.push(Symbol {
            id: id.clone(),
            source_id: self.source.id.clone(),
            name,
            qualified_name: qualified.join("::"),
            kind: kind.into(),
            // This is an attribute observation, not evidence that a test ran.
            is_test: attrs.iter().any(|a| a.path().is_ident("test")),
            chunk_id: id.clone(),
        });
        self.chunks.push(Chunk {
            id,
            source_id: self.source.id.clone(),
            start_byte: range.start,
            end_byte: range.end,
            start_line: crate::line_at(self.bytes, range.start),
            end_line: crate::line_at(self.bytes, range.end.saturating_sub(1)),
            sha256: hash(&self.bytes[range]),
        });
    }

    fn span_text(&self, span: Span) -> String {
        let range = span.byte_range();
        String::from_utf8_lossy(&self.bytes[range.start + self.offset..range.end + self.offset])
            .into_owned()
    }
}

impl<'ast> Visit<'ast> for Declarations<'_> {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        let declaration = match item {
            syn::Item::Fn(i) => Some(("function", i.sig.ident.to_string(), &i.attrs)),
            syn::Item::Struct(i) => Some(("struct", i.ident.to_string(), &i.attrs)),
            syn::Item::Enum(i) => Some(("enum", i.ident.to_string(), &i.attrs)),
            syn::Item::Union(i) => Some(("union", i.ident.to_string(), &i.attrs)),
            syn::Item::Trait(i) => Some(("trait", i.ident.to_string(), &i.attrs)),
            syn::Item::Type(i) => Some(("type", i.ident.to_string(), &i.attrs)),
            syn::Item::Mod(i) => Some(("module", i.ident.to_string(), &i.attrs)),
            syn::Item::Const(i) => Some(("const", i.ident.to_string(), &i.attrs)),
            syn::Item::Static(i) => Some(("static", i.ident.to_string(), &i.attrs)),
            syn::Item::Impl(i) => {
                let target = self.span_text(i.self_ty.span());
                let name = match &i.trait_ {
                    Some((path, _)) => {
                        format!("impl {} for {}", self.span_text(path.span()), target)
                    }
                    None => format!("impl {}", target),
                };
                Some(("impl", name, &i.attrs))
            }
            _ => None,
        };
        if let Some((kind, name, attrs)) = declaration {
            self.add(kind, name.clone(), item.span(), attrs);
            self.scope.push(name);
            // Named fields are declarations of their struct; tuple fields have no name to match.
            if let (true, syn::Item::Struct(s)) = (self.g6, item) {
                for field in &s.fields {
                    if let Some(ident) = &field.ident {
                        self.add("field", ident.to_string(), field.span(), &field.attrs);
                    }
                }
            }
            visit::visit_item(self, item);
            self.scope.pop();
        } else {
            visit::visit_item(self, item);
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let name = item.sig.ident.to_string();
        self.add("method", name.clone(), item.span(), &item.attrs);
        self.scope.push(name);
        visit::visit_impl_item_fn(self, item);
        self.scope.pop();
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        let name = item.sig.ident.to_string();
        self.add("trait_method", name.clone(), item.span(), &item.attrs);
        self.scope.push(name);
        visit::visit_trait_item_fn(self, item);
        self.scope.pop();
    }
}

/// Written types, returns, calls and bindings. Calls are never linked to presumed targets.
struct Occurrences {
    g6: bool,
    offset: usize,
    found: Vec<Occurrence>,
}

impl Occurrences {
    fn push(&mut self, kind: &'static str, span: Span, candidate: Option<String>) {
        let range = span.byte_range();
        self.found.push(Occurrence {
            kind,
            start: range.start + self.offset,
            end: range.end + self.offset,
            candidate,
        });
    }
}

impl<'ast> Visit<'ast> for Occurrences {
    fn visit_type_path(&mut self, value: &'ast syn::TypePath) {
        let name = value.path.segments.last().map(|s| s.ident.to_string());
        self.push("type_reference", value.span(), name);
        visit::visit_type_path(self, value);
    }
    fn visit_return_type(&mut self, value: &'ast syn::ReturnType) {
        if let syn::ReturnType::Type(_, ty) = value {
            self.push("declared_return", ty.span(), None);
        }
        visit::visit_return_type(self, value);
    }
    fn visit_expr_call(&mut self, value: &'ast syn::ExprCall) {
        // The last path segment is the written name (`Ray::new` -> `new`); never a resolved callee.
        let name = match (self.g6, value.func.as_ref()) {
            (true, syn::Expr::Path(p)) => p.path.segments.last().map(|s| s.ident.to_string()),
            _ => None,
        };
        self.push("call_observed", value.func.span(), name);
        visit::visit_expr_call(self, value);
    }
    fn visit_expr_method_call(&mut self, value: &'ast syn::ExprMethodCall) {
        // Keep receiver and arguments; no receiver type is inferred.
        let name = self.g6.then(|| value.method.to_string());
        self.push("method_call_observed", value.span(), name);
        visit::visit_expr_method_call(self, value);
    }
    fn visit_expr_field(&mut self, value: &'ast syn::ExprField) {
        if let (true, syn::Member::Named(ident)) = (self.g6, &value.member) {
            self.push("attribute_access", value.span(), Some(ident.to_string()));
        }
        visit::visit_expr_field(self, value);
    }
    fn visit_local(&mut self, value: &'ast syn::Local) {
        self.push("local_binding", value.pat.span(), None);
        visit::visit_local(self, value);
    }
}
