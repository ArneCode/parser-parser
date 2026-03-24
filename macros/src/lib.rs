// my_macro_crate/src/lib.rs

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::visit_mut::VisitMut;
use syn::{Block, Expr, Ident, Token, parse_macro_input, parse_quote};

// 1. Structure to hold our parsed input: { grammar } => { body }
struct CaptureInput {
    grammar: Expr,
    _arrow: Token![=>],
    constructor: Block,
}

impl Parse for CaptureInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // We expect a block for the grammar, but standard Expr handles `{ ... }`
        let grammar = input.parse()?;
        let _arrow = input.parse()?;
        let constructor = input.parse()?;
        Ok(CaptureInput {
            grammar,
            _arrow,
            constructor,
        })
    }
}

// 2. Enum to classify the type of binding
enum BindKind {
    Single(Ident),
    Multiple(Ident), // starts with *
    Optional(Ident), // starts with ?
}
struct BindInput {
    parser: Expr,
    _comma: Token![,],
    kind: BindKind,
}

impl Parse for BindInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let parser: Expr = input.parse()?;
        let _comma: Token![,] = input.parse()?;

        let kind = if input.peek(Token![*]) {
            input.parse::<Token![*]>()?;
            BindKind::Multiple(input.parse()?)
        } else if input.peek(Token![?]) {
            input.parse::<Token![?]>()?;
            BindKind::Optional(input.parse()?)
        } else {
            BindKind::Single(input.parse()?)
        };

        Ok(BindInput {
            parser,
            _comma,
            kind,
        })
    }
}
// 3. The visitor that will traverse the AST and mutate it
struct BindVisitor {
    single_vars: Vec<Ident>,
    multiple_vars: Vec<Ident>,
    optional_vars: Vec<Ident>,
    errors: Vec<syn::Error>,
}

impl BindVisitor {
    fn new() -> Self {
        Self {
            single_vars: Vec::new(),
            multiple_vars: Vec::new(),
            optional_vars: Vec::new(),
            errors: Vec::new(),
        }
    }

    // Helper to add a var without duplicating it (allows re-using `params`)
    fn add_unique(vars: &mut Vec<Ident>, ident: Ident) {
        if !vars.iter().any(|v| v == &ident) {
            vars.push(ident);
        }
    }
}

impl VisitMut for BindVisitor {
    fn visit_expr_mut(&mut self, node: &mut Expr) {
        // Visit children first
        syn::visit_mut::visit_expr_mut(self, node);

        // Check if this node is a macro call to `bind!`
        if let Expr::Macro(expr_macro) = node {
            if expr_macro.mac.path.is_ident("bind") {
                // Now syn::parse2 knows exactly what to parse into: BindInput
                match syn::parse2::<BindInput>(expr_macro.mac.tokens.clone()) {
                    Ok(bind) => {
                        let ident = match bind.kind {
                            BindKind::Single(id) => {
                                Self::add_unique(&mut self.single_vars, id.clone());
                                id
                            }
                            BindKind::Multiple(id) => {
                                Self::add_unique(&mut self.multiple_vars, id.clone());
                                id
                            }
                            BindKind::Optional(id) => {
                                Self::add_unique(&mut self.optional_vars, id.clone());
                                id
                            }
                        };

                        let parser_expr = bind.parser;
                        *node = parse_quote! {
                            capture_property(#parser_expr, #ident)
                        };
                    }
                    Err(e) => self.errors.push(e),
                }
            }
        }
    }
}

#[proc_macro]
pub fn capture(input: TokenStream) -> TokenStream {
    let mut parsed_input = parse_macro_input!(input as CaptureInput);
    let mut visitor = BindVisitor::new();

    // Traverse and mutate the grammar expression
    visitor.visit_expr_mut(&mut parsed_input.grammar);

    // If there were syntax errors inside the `bind!` calls, return them
    if let Some(err) = visitor.errors.into_iter().reduce(|mut a, b| {
        a.combine(b);
        a
    }) {
        return err.to_compile_error().into();
    }

    let grammar = parsed_input.grammar;
    let constructor = parsed_input.constructor;

    // Get the lists and counts
    let singles = &visitor.single_vars;
    let multiples = &visitor.multiple_vars;
    let optionals = &visitor.optional_vars;

    let n_single = singles.len();
    let n_multiple = multiples.len();
    let n_optional = optionals.len();

    // Generate extraction logic for the constructor
    // Example: let name = __ctx.match_result.single_matches[0].take().expect(...);
    let extract_singles = singles.iter().enumerate().map(|(i, id)| {
        quote! {
            let #id = __ctx.match_result.single_matches[#i]
                .take()
                .expect(concat!("capture!: single capture `", stringify!(#id), "` was never set"));
        }
    });

    let extract_multiples = multiples.iter().enumerate().map(|(i, id)| {
        quote! {
            let #id = ::std::mem::take(&mut __ctx.match_result.multiple_matches[#i]);
        }
    });

    let extract_optionals = optionals.iter().enumerate().map(|(i, id)| {
        quote! {
            let #id = __ctx.match_result.optional_matches[#i].take();
        }
    });

    // Expand into the final output
    let expanded = quote! {
        Capture::new::<#n_single, #n_multiple, #n_optional, _>(
            |[#(#singles),*], [#(#multiples),*], [#(#optionals),*]| {
                #grammar
            },
            |mut __ctx| {
                #(#extract_singles)*
                #(#extract_multiples)*
                #(#extract_optionals)*

                // Finally, run the user's block
                #constructor
            }
        )
    };

    TokenStream::from(expanded)
}
