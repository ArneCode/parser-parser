use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::visit_mut::{self, VisitMut};
use syn::{Expr, Ident, Result, Token, parse_macro_input, parse_quote};

// --- (Keep CaptureInput, BindKind, and BindInfo from previous version) ---
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

enum BindKind {
    Single,
    Multiple,
    Optional,
}
struct BindInfo {
    parser: Expr,
    ident: Ident,
    kind: BindKind,
}

impl Parse for BindInfo {
    fn parse(input: ParseStream) -> Result<Self> {
        let parser: Expr = input.parse()?;
        let _: Token![,] = input.parse()?;
        if input.peek(Token![*]) {
            input.parse::<Token![*]>()?;
            Ok(BindInfo {
                parser,
                ident: input.parse()?,
                kind: BindKind::Multiple,
            })
        } else if input.peek(Token![?]) {
            input.parse::<Token![?]>()?;
            Ok(BindInfo {
                parser,
                ident: input.parse()?,
                kind: BindKind::Optional,
            })
        } else {
            Ok(BindInfo {
                parser,
                ident: input.parse()?,
                kind: BindKind::Single,
            })
        }
    }
}

struct BindVisitor {
    singles: Vec<Ident>,
    multiples: Vec<Ident>,
    optionals: Vec<Ident>,
}

impl VisitMut for BindVisitor {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        if let Expr::Macro(m) = i {
            if m.mac.path.is_ident("bind") {
                if let Ok(info) = m.mac.parse_body::<BindInfo>() {
                    let id = &info.ident;
                    let parser = &info.parser;
                    match info.kind {
                        BindKind::Single => {
                            if !self.singles.contains(id) {
                                self.singles.push(id.clone())
                            }
                        }
                        BindKind::Multiple => {
                            if !self.multiples.contains(id) {
                                self.multiples.push(id.clone())
                            }
                        }
                        BindKind::Optional => {
                            if !self.optionals.contains(id) {
                                self.optionals.push(id.clone())
                            }
                        }
                    }
                    *i = parse_quote! { capture_property(#parser, #id.clone()) };
                    return;
                }
            }
        }
        visit_mut::visit_expr_mut(self, i);
    }
}

#[proc_macro]
pub fn capture(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as CaptureInput);
    let mut visitor = BindVisitor {
        singles: vec![],
        multiples: vec![],
        optionals: vec![],
    };

    visitor.visit_expr_mut(&mut input.grammar);

    let grammar = &input.grammar;
    let result_expr = &input.result_expr;

    // Helper to build the value tuples: (name, body,)
    let build_val_tuple = |idents: &[Ident]| {
        if idents.is_empty() {
            quote! { () }
        } else {
            quote! { ( #(#idents,)* ) }
        }
    };

    // Helper to build the type tuples: (Option<_>,) or (Vec<_>,)
    // We use fully qualified paths for context independence.
    let build_type_tuple = |idents: &[Ident], is_vec: bool| {
        if idents.is_empty() {
            quote! { () }
        } else {
            let inner = if is_vec {
                quote! { ::std::vec::Vec<_> }
            } else {
                quote! { ::std::option::Option<_> }
            };
            let types = idents.iter().map(|_| &inner);
            quote! { ( #(#types,)* ) }
        }
    };

    let s_pat = build_val_tuple(&visitor.singles);
    let m_pat = build_val_tuple(&visitor.multiples);
    let o_pat = build_val_tuple(&visitor.optionals);

    let s_ty = build_type_tuple(&visitor.singles, false); // Option<_>
    let m_ty = build_type_tuple(&visitor.multiples, true); // Vec<_>
    let o_ty = build_type_tuple(&visitor.optionals, false); // Option<_>

    let expanded = quote! {
        // Here we explicitly provide the capture group types
        Capture::<(#s_ty, #m_ty, #o_ty), _, _>::new(
            |#s_pat, #m_pat, #o_pat| {
                #grammar
            },
            |#s_pat, #m_pat, #o_pat| {
                #result_expr
            }
        )
    };

    TokenStream::from(expanded)
}
