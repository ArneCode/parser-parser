use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::visit_mut::{self, VisitMut};
use syn::{Expr, Ident, Result, Token, parse_macro_input, parse_quote};

// ---------------------------------------------------------------------------
// Input structs
// ---------------------------------------------------------------------------

struct CaptureInput {
    grammar: Expr,
    _arrow: Token![=>],
    result_expr: Expr,
}

impl Parse for CaptureInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let grammar = input.parse()?;
        let _arrow = input.parse::<Token![=>]>()?;
        let result_expr = input.parse()?;
        Ok(CaptureInput {
            grammar,
            _arrow,
            result_expr,
        })
    }
}

#[derive(Clone, PartialEq)]
enum BindKind {
    Single,
    Multiple,
    Optional,
}

/// Shared helper: peek at an optional `*` / `?` sigil, then parse the ident.
fn parse_kind_and_ident(input: ParseStream) -> Result<(BindKind, Ident)> {
    if input.peek(Token![*]) {
        input.parse::<Token![*]>()?;
        Ok((BindKind::Multiple, input.parse()?))
    } else if input.peek(Token![?]) {
        input.parse::<Token![?]>()?;
        Ok((BindKind::Optional, input.parse()?))
    } else {
        Ok((BindKind::Single, input.parse()?))
    }
}

/// `bind!(parser, [*|?]ident [, [*|?]span_ident])`
struct BindInfo {
    parser: Expr,
    ident: Ident,
    kind: BindKind,
    span_ident: Option<Ident>,
    span_kind: Option<BindKind>,
}

impl Parse for BindInfo {
    fn parse(input: ParseStream) -> Result<Self> {
        let parser: Expr = input.parse()?;
        let _: Token![,] = input.parse()?;
        let (kind, ident) = parse_kind_and_ident(input)?;

        let (span_ident, span_kind) = if input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
            let (sk, si) = parse_kind_and_ident(input)?;
            (Some(si), Some(sk))
        } else {
            (None, None)
        };

        Ok(BindInfo {
            parser,
            ident,
            kind,
            span_ident,
            span_kind,
        })
    }
}

/// `bind_span!(parser, [*|?]span_ident)` – binds only the span, no value capture.
struct BindSpanInfo {
    parser: Expr,
    span_ident: Ident,
    kind: BindKind,
}

impl Parse for BindSpanInfo {
    fn parse(input: ParseStream) -> Result<Self> {
        let parser: Expr = input.parse()?;
        let _: Token![,] = input.parse()?;
        let (kind, span_ident) = parse_kind_and_ident(input)?;
        Ok(BindSpanInfo {
            parser,
            span_ident,
            kind,
        })
    }
}

// ---------------------------------------------------------------------------
// Visitor
// ---------------------------------------------------------------------------

/// Collects all bound idents, keeping value idents and span idents in separate
/// ordered lists so the generated tuple is `(vals…, spans…)` for each bucket.
struct BindVisitor {
    // Singles (Option<_> / Option<Span>)
    single_values: Vec<Ident>,
    single_spans: Vec<Ident>,
    // Multiples (Vec<_> / Vec<Span>)
    multiple_values: Vec<Ident>,
    multiple_spans: Vec<Ident>,
    // Optionals (Option<_> / Option<Span>)
    optional_values: Vec<Ident>,
    optional_spans: Vec<Ident>,
}

impl BindVisitor {
    fn new() -> Self {
        Self {
            single_values: vec![],
            single_spans: vec![],
            multiple_values: vec![],
            multiple_spans: vec![],
            optional_values: vec![],
            optional_spans: vec![],
        }
    }

    fn push_unique(list: &mut Vec<Ident>, ident: Ident) {
        if !list.contains(&ident) {
            list.push(ident);
        }
    }

    fn register_value(&mut self, ident: Ident, kind: &BindKind) {
        match kind {
            BindKind::Single => Self::push_unique(&mut self.single_values, ident),
            BindKind::Multiple => Self::push_unique(&mut self.multiple_values, ident),
            BindKind::Optional => Self::push_unique(&mut self.optional_values, ident),
        }
    }

    fn register_span(&mut self, ident: Ident, kind: &BindKind) {
        match kind {
            BindKind::Single => Self::push_unique(&mut self.single_spans, ident),
            BindKind::Multiple => Self::push_unique(&mut self.multiple_spans, ident),
            BindKind::Optional => Self::push_unique(&mut self.optional_spans, ident),
        }
    }
}

impl VisitMut for BindVisitor {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        if let Expr::Macro(m) = i {
            // ── bind!(parser, [*|?]ident [, [*|?]span_ident]) ──────────────
            if m.mac.path.is_ident("bind") {
                if let Ok(info) = m.mac.parse_body::<BindInfo>() {
                    let id = &info.ident;
                    let parser = &info.parser;

                    self.register_value(id.clone(), &info.kind);

                    *i = if let Some(span_id) = &info.span_ident {
                        let span_kind = info.span_kind.as_ref().unwrap();
                        self.register_span(span_id.clone(), span_kind);
                        // wrap: bind_span( bind_result(parser, id), span_id )
                        parse_quote! {
                            bind_span(bind_result(#parser, #id.clone()), #span_id.clone())
                        }
                    } else {
                        parse_quote! { bind_result(#parser, #id.clone()) }
                    };
                    return;
                }
            }

            // ── bind_span!(parser, [*|?]span_ident) ─────────────────────────
            if m.mac.path.is_ident("bind_span") {
                if let Ok(info) = m.mac.parse_body::<BindSpanInfo>() {
                    let span_id = &info.span_ident;
                    let parser = &info.parser;

                    self.register_span(span_id.clone(), &info.kind);

                    *i = parse_quote! { bind_span(#parser, #span_id.clone()) };
                    return;
                }
            }
        }
        visit_mut::visit_expr_mut(self, i);
    }
}

// ---------------------------------------------------------------------------
// capture! proc-macro
// ---------------------------------------------------------------------------

#[proc_macro]
pub fn capture(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as CaptureInput);
    let mut visitor = BindVisitor::new();

    visitor.visit_expr_mut(&mut input.grammar);

    let grammar = &input.grammar;
    let result_expr = &input.result_expr;

    // Build `(ident0, ident1, …)` pattern for a bucket.
    // Values always come first, spans are appended afterwards.
    let pat_tuple = |values: &[Ident], spans: &[Ident]| {
        let all: Vec<_> = values.iter().chain(spans.iter()).collect();
        if all.is_empty() {
            quote! { () }
        } else {
            quote! { ( #(#all,)* ) }
        }
    };

    // Build the corresponding type tuple.
    //   Values → Option<_>  / Vec<_>
    //   Spans  → Option<span::Span> / Vec<span::Span>
    let type_tuple = |values: &[Ident], spans: &[Ident], is_vec: bool| {
        let val_ty = if is_vec {
            quote! { ::std::vec::Vec<_> }
        } else {
            quote! { ::std::option::Option<_>          }
        };
        let span_ty = if is_vec {
            quote! { ::std::vec::Vec<span::Span> }
        } else {
            quote! { ::std::option::Option<span::Span> }
        };

        let val_types: Vec<_> = values.iter().map(|_| val_ty.clone()).collect();
        let span_types: Vec<_> = spans.iter().map(|_| span_ty.clone()).collect();
        let all: Vec<_> = val_types.into_iter().chain(span_types).collect();

        if all.is_empty() {
            quote! { () }
        } else {
            quote! { ( #(#all,)* ) }
        }
    };

    let s_pat = pat_tuple(&visitor.single_values, &visitor.single_spans);
    let m_pat = pat_tuple(&visitor.multiple_values, &visitor.multiple_spans);
    let o_pat = pat_tuple(&visitor.optional_values, &visitor.optional_spans);

    let s_ty = type_tuple(&visitor.single_values, &visitor.single_spans, false);
    let m_ty = type_tuple(&visitor.multiple_values, &visitor.multiple_spans, true);
    let o_ty = type_tuple(&visitor.optional_values, &visitor.optional_spans, false);

    TokenStream::from(quote! {
        Capture::<(#s_ty, #m_ty, #o_ty), _, _>::new(
            |#s_pat, #m_pat, #o_pat| { #grammar     },
            |#s_pat, #m_pat, #o_pat| { #result_expr },
        )
    })
}
