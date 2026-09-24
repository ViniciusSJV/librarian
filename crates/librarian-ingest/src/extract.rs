use crate::{hash, Chunk, Diagnostic, Source, Symbol};
use proc_macro2::Span;
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
};

// Only syntactic declarations. No macro expansion, cfg evaluation or name resolution.
pub(crate) fn extract(source: &Source, bytes: &[u8]) -> (Vec<Symbol>, Vec<Chunk>, Vec<Diagnostic>) {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            return (
                vec![],
                vec![],
                vec![Diagnostic {
                    source_id: source.id.clone(),
                    code: "invalid_utf8".into(),
                    message: error.to_string(),
                }],
            )
        }
    };
    let tree = match syn::parse_file(text) {
        Ok(tree) => tree,
        Err(error) => {
            return (
                vec![],
                vec![],
                vec![Diagnostic {
                    source_id: source.id.clone(),
                    code: "rust_parse_error".into(),
                    message: error.to_string(),
                }],
            )
        }
    };
    // parse_file strips a UTF-8 BOM and shebang before parsing. Restore original byte offsets.
    let offset = if text.starts_with('\u{feff}') { 3 } else { 0 }
        + tree.shebang.as_ref().map_or(0, String::len);
    let mut visitor = Extractor {
        source,
        bytes,
        offset,
        scope: vec![],
        symbols: vec![],
        chunks: vec![],
    };
    visitor.visit_file(&tree);
    (visitor.symbols, visitor.chunks, vec![])
}

struct Extractor<'a> {
    source: &'a Source,
    bytes: &'a [u8],
    offset: usize,
    scope: Vec<String>,
    symbols: Vec<Symbol>,
    chunks: Vec<Chunk>,
}

impl Extractor<'_> {
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

impl<'ast> Visit<'ast> for Extractor<'_> {
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
