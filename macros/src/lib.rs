//! Procedural macros for [`marser`](https://docs.rs/marser).
//!
//! The main entry point is [`capture`], which builds a
//! `marser::parser::capture::Capture` parser from a grammar expression.

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::visit_mut::{self, VisitMut};
use syn::{Expr, Ident, Path, Result, Token, Type, parse_macro_input, parse_quote};

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

#[derive(Clone)]
struct TypedBindTarget {
    kind: BindKind,
    ident: Ident,
    ty: Option<Type>,
}

fn parse_typed_target(input: ParseStream) -> Result<TypedBindTarget> {
    let (kind, ident) = parse_kind_and_ident(input)?;
    let ty = if input.peek(Token![as]) {
        input.parse::<Token![as]>()?;
        Some(input.parse::<Type>()?)
    } else {
        None
    };
    Ok(TypedBindTarget { kind, ident, ty })
}

/// `bind!(parser, [*|?]ident [as Type] [, [*|?]span_ident [as Type]])`
struct BindInfo {
    parser: Expr,
    ident: Ident,
    kind: BindKind,
    value_ty: Option<Type>,
    span_ident: Option<Ident>,
    span_kind: Option<BindKind>,
    span_ty: Option<Type>,
}

impl Parse for BindInfo {
    fn parse(input: ParseStream) -> Result<Self> {
        let parser: Expr = input.parse()?;
        let _: Token![,] = input.parse()?;
        let value_target = parse_typed_target(input)?;

        let (span_ident, span_kind, span_ty) = if input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
            let span_target = parse_typed_target(input)?;
            (Some(span_target.ident), Some(span_target.kind), span_target.ty)
        } else {
            (None, None, None)
        };

        Ok(BindInfo {
            parser,
            ident: value_target.ident,
            kind: value_target.kind,
            value_ty: value_target.ty,
            span_ident,
            span_kind,
            span_ty,
        })
    }
}

/// `bind_span!(parser, [*|?]span_ident [as Type])` – binds only the span, no value capture.
struct BindSpanInfo {
    parser: Expr,
    span_ident: Ident,
    kind: BindKind,
    ty: Option<Type>,
}

impl Parse for BindSpanInfo {
    fn parse(input: ParseStream) -> Result<Self> {
        let parser: Expr = input.parse()?;
        let _: Token![,] = input.parse()?;
        let target = parse_typed_target(input)?;
        Ok(BindSpanInfo {
            parser,
            span_ident: target.ident,
            kind: target.kind,
            ty: target.ty,
        })
    }
}

/// `bind_slice!(parser, [*|?]slice_ident [as Type])` – binds only the consumed slice.
struct BindSliceInfo {
    parser: Expr,
    slice_ident: Ident,
    kind: BindKind,
    ty: Option<Type>,
}

impl Parse for BindSliceInfo {
    fn parse(input: ParseStream) -> Result<Self> {
        let parser: Expr = input.parse()?;
        let _: Token![,] = input.parse()?;
        let target = parse_typed_target(input)?;
        Ok(BindSliceInfo {
            parser,
            slice_ident: target.ident,
            kind: target.kind,
            ty: target.ty,
        })
    }
}

// ---------------------------------------------------------------------------
// Visitor
// ---------------------------------------------------------------------------

/// Collects all bound idents, keeping value idents and span idents in separate
/// ordered lists so the generated tuple is `(vals…, spans…)` for each bucket.
struct BindVisitor {
    marser_path: Path,
    // Singles (Option<_> / Option<Span>)
    single_values: Vec<TypedBinding>,
    single_spans: Vec<TypedBinding>,
    // Multiples (Vec<_> / Vec<Span>)
    multiple_values: Vec<TypedBinding>,
    multiple_spans: Vec<TypedBinding>,
    // Optionals (Option<_> / Option<Span>)
    optional_values: Vec<TypedBinding>,
    optional_spans: Vec<TypedBinding>,
}

#[derive(Clone)]
struct TypedBinding {
    ident: Ident,
    ty: Option<Type>,
}

impl BindVisitor {
    fn new(marser_path: Path) -> Self {
        Self {
            marser_path,
            single_values: vec![],
            single_spans: vec![],
            multiple_values: vec![],
            multiple_spans: vec![],
            optional_values: vec![],
            optional_spans: vec![],
        }
    }

    fn push_unique(list: &mut Vec<TypedBinding>, ident: Ident, ty: Option<Type>) {
        if list.iter().all(|entry| entry.ident != ident) {
            list.push(TypedBinding { ident, ty });
        }
    }

    fn register_value(&mut self, ident: Ident, ty: Option<Type>, kind: &BindKind) {
        match kind {
            BindKind::Single => Self::push_unique(&mut self.single_values, ident, ty),
            BindKind::Multiple => Self::push_unique(&mut self.multiple_values, ident, ty),
            BindKind::Optional => Self::push_unique(&mut self.optional_values, ident, ty),
        }
    }

    fn register_span(&mut self, ident: Ident, ty: Option<Type>, kind: &BindKind) {
        match kind {
            BindKind::Single => Self::push_unique(&mut self.single_spans, ident, ty),
            BindKind::Multiple => Self::push_unique(&mut self.multiple_spans, ident, ty),
            BindKind::Optional => Self::push_unique(&mut self.optional_spans, ident, ty),
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

                    self.register_value(id.clone(), info.value_ty.clone(), &info.kind);

                    *i = if let Some(span_id) = &info.span_ident {
                        let marser = self.marser_path.clone();
                        let span_kind = info.span_kind.as_ref().unwrap();
                        self.register_span(span_id.clone(), info.span_ty.clone(), span_kind);
                        // wrap: bind_span( bind_result(parser, id), span_id )
                        parse_quote! {
                            #marser::parser::capture::bind_span(
                                #marser::parser::capture::bind_result_with_debug(
                                    #parser,
                                    #id.clone(),
                                    #marser::parser::capture::BindDebugInfo {
                                        property_name: stringify!(#id),
                                        file: file!(),
                                        line: line!(),
                                        column: column!(),
                                    }
                                ),
                                #span_id.clone()
                            )
                        }
                    } else {
                        let marser = self.marser_path.clone();
                        parse_quote! {
                            #marser::parser::capture::bind_result_with_debug(
                                #parser,
                                #id.clone(),
                                #marser::parser::capture::BindDebugInfo {
                                    property_name: stringify!(#id),
                                    file: file!(),
                                    line: line!(),
                                    column: column!(),
                                }
                            )
                        }
                    };
                    return;
                }
            }

            // ── bind_span!(parser, [*|?]span_ident) ─────────────────────────
            if m.mac.path.is_ident("bind_span") {
                if let Ok(info) = m.mac.parse_body::<BindSpanInfo>() {
                    let span_id = &info.span_ident;
                    let parser = &info.parser;

                    self.register_span(span_id.clone(), info.ty.clone(), &info.kind);

                    let marser = self.marser_path.clone();
                    *i = parse_quote! { #marser::parser::capture::bind_span(#parser, #span_id.clone()) };
                    return;
                }
            }

            // ── bind_slice!(parser, [*|?]slice_ident) ───────────────────────
            if m.mac.path.is_ident("bind_slice") {
                if let Ok(info) = m.mac.parse_body::<BindSliceInfo>() {
                    let slice_id = &info.slice_ident;
                    let parser = &info.parser;

                    self.register_value(slice_id.clone(), info.ty.clone(), &info.kind);

                    let marser = self.marser_path.clone();
                    *i = parse_quote! { #marser::parser::capture::bind_slice(#parser, #slice_id.clone()) };
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

/// Build a [`Capture`](https://docs.rs/marser/latest/marser/parser/capture/struct.Capture.html) parser from grammar + result expressions.
///
/// # Syntax
///
/// ```ignore
/// capture!( <grammar> => <result> )
/// ```
///
/// - **`<grammar>`** — any expression after `bind!` / `bind_span!` expansion (typically a tuple
///   of matchers, often using [`crate::matcher::Matcher`](https://docs.rs/marser/latest/marser/matcher/trait.Matcher.html)
///   combinators like `many` / `one_of`).
/// - **`<result>`** — Rust expression that receives the captured bindings and produces the parser output.
///
/// Inside `<grammar>`, the macro recognizes:
///
/// - **`bind!(parser, ident)`** — single capture into `ident` (`Option<_>` in the bucket).
/// - **`bind!(parser, *ident)`** — repeated capture into `ident` (`Vec<_>`).
/// - **`bind!(parser, ?ident)`** — optional capture (`Option<_>`).
/// - **`bind!(parser, ident as T)`** / **`bind!(parser, *ident as T)`** / **`bind!(parser, ?ident as T)`** —
///   typed captures. With `*` / `?`, the sigil still wraps `T` (Option A semantics).
/// - **`bind!(parser, ident [as T], *span_ident [as U])`** (and `?` / `*` on the value) — value plus span capture.
/// - **`bind_span!(parser, ident)`** / **`bind_span!(parser, *ident)`** / **`bind_span!(parser, ?ident)`** / **`bind_span!(parser, ident as T)`** —
///   capture only a span (expands to `marser::parser::capture::bind_span`).
/// - **`bind_slice!(parser, ident)`** / **`bind_slice!(parser, *ident)`** / **`bind_slice!(parser, ?ident)`** / **`bind_slice!(parser, ident as T)`** —
///   capture only the consumed input slice (expands to `marser::parser::capture::bind_slice`).
///
/// Each binding becomes a parameter to both the grammar closure and the result closure generated
/// by this macro.
///
/// # Crate path
///
/// The expansion prefixes APIs with the dependency name from Cargo (via `proc_macro_crate::crate_name("marser")`).
/// If you rename the `marser` dependency in your `Cargo.toml`, generated paths use that name.
#[proc_macro]
pub fn capture(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as CaptureInput);
    let marser_path = marser_crate_path();
    let mut visitor = BindVisitor::new(marser_path.clone());

    visitor.visit_expr_mut(&mut input.grammar);

    let grammar = &input.grammar;
    let result_expr = &input.result_expr;

    // Build `(ident0, ident1, …)` pattern for a bucket.
    // Values always come first, spans are appended afterwards.
    let pat_tuple = |values: &[TypedBinding], spans: &[TypedBinding]| {
        let all: Vec<_> = values.iter().chain(spans.iter()).map(|x| &x.ident).collect();
        if all.is_empty() {
            quote! { () }
        } else {
            quote! { ( #(#all,)* ) }
        }
    };

    // Build the corresponding type tuple.
    //   Values → Option<_>  / Vec<_>
    //   Spans  → Option<span::Span> / Vec<span::Span>
    let type_tuple = |values: &[TypedBinding], spans: &[TypedBinding], is_vec: bool| {
        let wrap = |inner: proc_macro2::TokenStream| {
            if is_vec {
                quote! { ::std::vec::Vec<#inner> }
            } else {
                quote! { ::std::option::Option<#inner> }
            }
        };

        let val_types: Vec<_> = values
            .iter()
            .map(|binding| {
                let inner = if let Some(ty) = &binding.ty {
                    quote! { #ty }
                } else {
                    quote! { _ }
                };
                wrap(inner)
            })
            .collect();
        let span_types: Vec<_> = spans
            .iter()
            .map(|binding| {
                let inner = if let Some(ty) = &binding.ty {
                    quote! { #ty }
                } else {
                    quote! { (_, _) }
                };
                wrap(inner)
            })
            .collect();
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
        #marser_path::parser::capture::Capture::<(#s_ty, #m_ty, #o_ty), _, _>::new(
            |#s_pat, #m_pat, #o_pat| { #grammar     },
            |#s_pat, #m_pat, #o_pat| { #result_expr },
        )
    })
}

fn marser_crate_path() -> Path {
    match crate_name("marser") {
        Ok(FoundCrate::Itself) => parse_quote!(::marser),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            parse_quote!(::#ident)
        }
        Err(_) => parse_quote!(::marser),
    }
}
