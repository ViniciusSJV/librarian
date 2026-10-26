//! Rust adapter (syn). Only syntactic declarations and occurrences.
//! No macro expansion, cfg evaluation or name resolution.
use super::{Extraction, Extractor, Occurrence, OwnerSyntax};
use crate::{hash, Chunk, Diagnostic, Source, Symbol};
use proc_macro2::Span;
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
};

/// `g6` adds named struct fields, field accesses and call candidates (profile v3).
/// `g6d` adds struct literals, field writes and the type written in an impl (profile v4).
/// Without them, records and occurrences are exactly those of the frozen v1 adapter.
pub(crate) struct Rust {
    g6: bool,
    g6d: bool,
}

pub(crate) static RUST: Rust = Rust { g6: false, g6d: false };
pub(crate) static RUST_V3: Rust = Rust { g6: true, g6d: false };
pub(crate) static RUST_V4: Rust = Rust { g6: true, g6d: true };

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
            g6d: self.g6d,
            offset,
            found: vec![],
            impl_type: None,
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
            (true, "attribute_access" | "field_assign" | "field_init") => &["field"],
            (true, "impl_self_type" | "struct_literal") => &["struct", "enum", "union", "type"],
            _ => self.candidate_kinds(),
        }
    }

    fn navigation_kinds(&self) -> &'static [&'static str] {
        &["module", "impl"]
    }

    fn owner_syntax(&self) -> Option<&'static OwnerSyntax> {
        self.g6d.then_some(&OWNER)
    }

    fn call_kinds(&self) -> &'static [&'static str] {
        &["call_observed", "method_call_observed"]
    }
}

/// A method's owner is the type its impl names; fields are written by assignment or literal.
static OWNER: OwnerSyntax = OwnerSyntax {
    method_kinds: &["method"],
    owner_containers: &[],
    naming_container: Some(("impl", "impl_self_type")),
    owner_kinds: &["struct", "enum", "union"],
    receiver: "self",
    read: "attribute_access",
    write: Some("field_assign"),
    literal: Some(("struct_literal", "field_init", "Self")),
};

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
    g6d: bool,
    offset: usize,
    found: Vec<Occurrence>,
    /// Span of the type written in the impl being visited: it is `impl_self_type`, not also a
    /// type reference.
    impl_type: Option<std::ops::Range<usize>>,
}

impl Occurrences {
    /// G6d: `x.field = ...` (or `+=` and the like) writes the field: a `field_assign`, not also an
    /// access, and only the receiver is visited. False when the target is not a named field.
    fn write(&mut self, target: &syn::Expr) -> bool {
        let syn::Expr::Field(field) = target else { return false };
        let syn::Member::Named(ident) = &field.member else { return false };
        self.push("field_assign", field.span(), Some(ident.to_string()));
        self.visit_expr(&field.base);
        true
    }

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
        if self.impl_type.as_ref() == Some(&value.span().byte_range()) {
            self.impl_type = None;
        } else {
            let name = value.path.segments.last().map(|s| s.ident.to_string());
            self.push("type_reference", value.span(), name);
        }
        visit::visit_type_path(self, value);
    }
    fn visit_item_impl(&mut self, value: &'ast syn::ItemImpl) {
        // G6d: the type an impl is for, by its written name (`impl<T> Shape for Sphere<T>` ->
        // `Sphere`). Trait and generic arguments stay type references.
        if let (true, syn::Type::Path(path)) = (self.g6d, value.self_ty.as_ref()) {
            if let Some(last) = path.path.segments.last() {
                self.push("impl_self_type", last.ident.span(), Some(last.ident.to_string()));
                self.impl_type = Some(path.span().byte_range());
            }
        }
        visit::visit_item_impl(self, value);
        self.impl_type = None;
    }
    fn visit_expr_struct(&mut self, value: &'ast syn::ExprStruct) {
        // G6d: `Camera { transform, .. }` constructs the written type, and each named field it
        // fills is a `field_init` inside it. `Self` matches no declaration.
        if self.g6d {
            let name = value.path.segments.last().map(|s| s.ident.to_string());
            self.push("struct_literal", value.span(), name);
            for field in &value.fields {
                if let syn::Member::Named(ident) = &field.member {
                    self.push("field_init", ident.span(), Some(ident.to_string()));
                }
            }
        }
        visit::visit_expr_struct(self, value);
    }
    fn visit_expr_assign(&mut self, value: &'ast syn::ExprAssign) {
        if !self.g6d || !self.write(&value.left) {
            return visit::visit_expr_assign(self, value);
        }
        self.visit_expr(&value.right);
    }
    fn visit_expr_binary(&mut self, value: &'ast syn::ExprBinary) {
        use syn::BinOp::*;
        let compound = matches!(value.op, AddAssign(_) | SubAssign(_) | MulAssign(_) | DivAssign(_) | RemAssign(_)
            | BitXorAssign(_) | BitAndAssign(_) | BitOrAssign(_) | ShlAssign(_) | ShrAssign(_));
        if !self.g6d || !compound || !self.write(&value.left) {
            return visit::visit_expr_binary(self, value);
        }
        self.visit_expr(&value.right);
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
