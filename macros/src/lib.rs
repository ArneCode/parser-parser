//! Procedural macros for [`marser`](https://docs.rs/marser).
//!
//! Prefer `use marser::capture;` (re-exported from the main crate). This crate is the proc-macro
//! implementation; [`capture`] builds a `marser::parser::capture::Capture` parser from a grammar expression.

use std::cell::{Cell, RefCell};

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream, Result as ParseResult};
use syn::visit::{self, Visit};
use syn::visit_mut::{self, VisitMut};
use syn::{
    Expr, ExprClosure, Ident, Index, Path, Pat, Token, Type, parse_quote,
};

// ---------------------------------------------------------------------------
// Input structs
// ---------------------------------------------------------------------------

struct CaptureInput {
    grammar: Expr,
    _arrow: Token![=>],
    result_expr: Expr,
}

impl Parse for CaptureInput {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let grammar = input.parse()?;
        let _arrow = input.parse::<Token![=>]>()?;
        let result_expr = input.parse()?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after `capture!( … => … )`"));
        }
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
fn parse_kind_and_ident(input: ParseStream) -> ParseResult<(BindKind, Ident)> {
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

fn parse_typed_target(input: ParseStream) -> ParseResult<TypedBindTarget> {
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
    fn parse(input: ParseStream) -> ParseResult<Self> {
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

        if !input.is_empty() {
            return Err(input.error("unexpected tokens in `bind!` (expected `bind!(parser, target [, span_target])`)"));
        }

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
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let parser: Expr = input.parse()?;
        let _: Token![,] = input.parse()?;
        let target = parse_typed_target(input)?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens in `bind_span!`"));
        }
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
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let parser: Expr = input.parse()?;
        let _: Token![,] = input.parse()?;
        let target = parse_typed_target(input)?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens in `bind_slice!`"));
        }
        Ok(BindSliceInfo {
            parser,
            slice_ident: target.ident,
            kind: target.kind,
            ty: target.ty,
        })
    }
}

#[derive(Clone)]
struct TypedBinding {
    ident: Ident,
    ty: Option<Type>,
}

/// Registry of `bind!` / `bind_span!` / `bind_slice!` idents (layout matches [`BindCollector`] output).
#[derive(Default, Clone)]
struct BindRegistry {
    single_values: Vec<TypedBinding>,
    single_spans: Vec<TypedBinding>,
    multiple_values: Vec<TypedBinding>,
    multiple_spans: Vec<TypedBinding>,
    optional_values: Vec<TypedBinding>,
    optional_spans: Vec<TypedBinding>,
}

impl BindRegistry {
    fn types_compatible(a: &Option<Type>, b: &Option<Type>) -> bool {
        match (a, b) {
            (None, _) | (_, None) => true,
            (Some(t1), Some(t2)) => quote!(#t1).to_string() == quote!(#t2).to_string(),
        }
    }

    fn value_kind_if_present(&self, id: &Ident) -> Option<BindKind> {
        if self.single_values.iter().any(|b| b.ident == *id) {
            Some(BindKind::Single)
        } else if self.multiple_values.iter().any(|b| b.ident == *id) {
            Some(BindKind::Multiple)
        } else if self.optional_values.iter().any(|b| b.ident == *id) {
            Some(BindKind::Optional)
        } else {
            None
        }
    }

    fn span_kind_if_present(&self, id: &Ident) -> Option<BindKind> {
        if self.single_spans.iter().any(|b| b.ident == *id) {
            Some(BindKind::Single)
        } else if self.multiple_spans.iter().any(|b| b.ident == *id) {
            Some(BindKind::Multiple)
        } else if self.optional_spans.iter().any(|b| b.ident == *id) {
            Some(BindKind::Optional)
        } else {
            None
        }
    }

    fn ident_in_any_span_list(&self, id: &Ident) -> bool {
        self.span_kind_if_present(id).is_some()
    }

    fn ident_in_any_value_list(&self, id: &Ident) -> bool {
        self.value_kind_if_present(id).is_some()
    }

    fn values_mut(&mut self, kind: &BindKind) -> &mut Vec<TypedBinding> {
        match kind {
            BindKind::Single => &mut self.single_values,
            BindKind::Multiple => &mut self.multiple_values,
            BindKind::Optional => &mut self.optional_values,
        }
    }

    fn spans_mut(&mut self, kind: &BindKind) -> &mut Vec<TypedBinding> {
        match kind {
            BindKind::Single => &mut self.single_spans,
            BindKind::Multiple => &mut self.multiple_spans,
            BindKind::Optional => &mut self.optional_spans,
        }
    }

    fn merge_into(list: &mut Vec<TypedBinding>, ident: Ident, ty: Option<Type>) -> std::result::Result<(), syn::Error> {
        if let Some(existing) = list.iter_mut().find(|e| e.ident == ident) {
            if !Self::types_compatible(&existing.ty, &ty) {
                return Err(syn::Error::new_spanned(
                    &ident,
                    format!(
                        "conflicting explicit `as` types for repeated binding `{}` in `capture!`",
                        ident
                    ),
                ));
            }
            if existing.ty.is_none() {
                if let Some(t) = ty {
                    existing.ty = Some(t);
                }
            }
            Ok(())
        } else {
            list.push(TypedBinding { ident, ty });
            Ok(())
        }
    }

    /// Register a value capture. Repeated uses of the same `ident` with the same sigil bucket are merged
    /// when `as` types are compatible (see module docs on `capture!`).
    fn register_value(&mut self, ident: Ident, ty: Option<Type>, kind: &BindKind) -> std::result::Result<(), syn::Error> {
        if self.ident_in_any_span_list(&ident) {
            return Err(syn::Error::new_spanned(
                &ident,
                format!(
                    "binding `{}` is already used as a span binding; value and span captures cannot share an identifier in `capture!`",
                    ident
                ),
            ));
        }
        if let Some(existing) = self.value_kind_if_present(&ident) {
            if existing != *kind {
                return Err(syn::Error::new_spanned(
                    &ident,
                    format!(
                        "binding `{}` is used with incompatible sigils (for example `x` vs `*x` vs `?x`) in the same `capture!`",
                        ident
                    ),
                ));
            }
        }
        let list = self.values_mut(kind);
        Self::merge_into(list, ident, ty)
    }

    fn register_span(&mut self, ident: Ident, ty: Option<Type>, kind: &BindKind) -> std::result::Result<(), syn::Error> {
        if self.ident_in_any_value_list(&ident) {
            return Err(syn::Error::new_spanned(
                &ident,
                format!(
                    "binding `{}` is already used as a value binding; value and span captures cannot share an identifier in `capture!`",
                    ident
                ),
            ));
        }
        if let Some(existing) = self.span_kind_if_present(&ident) {
            if existing != *kind {
                return Err(syn::Error::new_spanned(
                    &ident,
                    format!(
                        "span binding `{}` is used with incompatible sigils in the same `capture!`",
                        ident
                    ),
                ));
            }
        }
        let list = self.spans_mut(kind);
        Self::merge_into(list, ident, ty)
    }
}

/// Walk the raw grammar `Expr` before `bind!` expansion and collect binding targets.
struct BindCollector {
    reg: BindRegistry,
    errors: Option<syn::Error>,
}

impl BindCollector {
    fn bump_err(&mut self, e: syn::Error) {
        self.errors = Some(match self.errors.take() {
            None => e,
            Some(mut prev) => {
                prev.combine(e);
                prev
            }
        });
    }

    fn collect(expr: &Expr) -> std::result::Result<BindRegistry, syn::Error> {
        let mut c = Self {
            reg: BindRegistry::default(),
            errors: None,
        };
        c.visit_expr(expr);
        if let Some(e) = c.errors {
            Err(e)
        } else {
            Ok(c.reg)
        }
    }
}

impl<'ast> Visit<'ast> for BindCollector {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if let Expr::Macro(m) = expr {
            if m.mac.path.is_ident("bind") {
                let info = match m.mac.parse_body::<BindInfo>() {
                    Ok(i) => i,
                    Err(e) => {
                        self.bump_err(e);
                        visit::visit_expr(self, expr);
                        return;
                    }
                };
                if let Some(ref span_ident) = info.span_ident {
                    if *span_ident == info.ident {
                        self.bump_err(syn::Error::new_spanned(
                            span_ident,
                            "`bind!` value and span targets must use distinct identifiers",
                        ));
                        self.visit_expr(&info.parser);
                        return;
                    }
                }
                if let Err(e) = self.reg.register_value(
                    info.ident.clone(),
                    info.value_ty.clone(),
                    &info.kind,
                ) {
                    self.bump_err(e);
                }
                if let Some(span_ident) = &info.span_ident {
                    let span_kind = info.span_kind.as_ref().unwrap();
                    if let Err(e) = self.reg.register_span(
                        span_ident.clone(),
                        info.span_ty.clone(),
                        span_kind,
                    ) {
                        self.bump_err(e);
                    }
                }
                self.visit_expr(&info.parser);
                return;
            }
            if m.mac.path.is_ident("bind_span") {
                let info = match m.mac.parse_body::<BindSpanInfo>() {
                    Ok(i) => i,
                    Err(e) => {
                        self.bump_err(e);
                        visit::visit_expr(self, expr);
                        return;
                    }
                };
                if let Err(e) = self
                    .reg
                    .register_span(info.span_ident.clone(), info.ty.clone(), &info.kind)
                {
                    self.bump_err(e);
                }
                self.visit_expr(&info.parser);
                return;
            }
            if m.mac.path.is_ident("bind_slice") {
                let info = match m.mac.parse_body::<BindSliceInfo>() {
                    Ok(i) => i,
                    Err(e) => {
                        self.bump_err(e);
                        visit::visit_expr(self, expr);
                        return;
                    }
                };
                if let Err(e) = self.reg.register_value(
                    info.slice_ident.clone(),
                    info.ty.clone(),
                    &info.kind,
                ) {
                    self.bump_err(e);
                }
                self.visit_expr(&info.parser);
                return;
            }
        }
        visit::visit_expr(self, expr);
    }
}

/// One `use_binds!(|ctx| { … })` site inside a `capture!`, collected for shared factory codegen.
struct UseBindSite {
    site: usize,
    ctx_ident: Ident,
    inner: proc_macro2::TokenStream,
}

/// Build the `(S, M, O)` tuple for [`Capture::<MRes, _, _>`] (same rules as the historical
/// `type_tuple` in `capture!`: explicit `as T` is preserved; untyped values use `_`).
fn build_capture_mres_tuple(registry: &BindRegistry) -> proc_macro2::TokenStream {
    let build_bucket = |values: &[TypedBinding], spans: &[TypedBinding], is_vec: bool| {
        let wrap = |inner: proc_macro2::TokenStream| {
            if is_vec {
                quote! { ::std::vec::Vec<#inner> }
            } else {
                quote! { ::std::option::Option<#inner> }
            }
        };
        let mut pieces = Vec::new();
        for b in values.iter() {
            let inner = if let Some(ty) = &b.ty {
                quote! { #ty }
            } else {
                quote! { _ }
            };
            pieces.push(wrap(inner));
        }
        for b in spans.iter() {
            let inner = if let Some(ty) = &b.ty {
                quote! { #ty }
            } else {
                quote! { (_, _) }
            };
            pieces.push(wrap(inner));
        }
        if pieces.is_empty() {
            quote! { () }
        } else {
            quote! { ( #(#pieces,)* ) }
        }
    };

    let s_ty = build_bucket(&registry.single_values, &registry.single_spans, false);
    let m_ty = build_bucket(&registry.multiple_values, &registry.multiple_spans, true);
    let o_ty = build_bucket(&registry.optional_values, &registry.optional_spans, false);
    quote! { (#s_ty, #m_ty, #o_ty) }
}

/// Build the `(S, M, O)` tuple for [`BuildInlineError<MRes>`] on `__UseBindsFactory`, plus the list
/// of fresh type parameters `__BindTn` (1A — avoids spelling outer lifetimes in nested impls).
fn build_factory_mres_tuple(registry: &BindRegistry) -> (proc_macro2::TokenStream, Vec<Ident>) {
    let mut gen_names: Vec<Ident> = Vec::new();
    let mut next_ty = |span: Span| -> proc_macro2::TokenStream {
        let n = gen_names.len();
        let id = Ident::new(&format!("__BindT{n}"), span);
        gen_names.push(id.clone());
        quote! { #id }
    };

    let mut build_bucket = |values: &[TypedBinding], spans: &[TypedBinding], is_vec: bool| {
        let wrap = |inner: proc_macro2::TokenStream| {
            if is_vec {
                quote! { ::std::vec::Vec<#inner> }
            } else {
                quote! { ::std::option::Option<#inner> }
            }
        };
        let mut pieces = Vec::new();
        for b in values.iter() {
            pieces.push(wrap(next_ty(b.ident.span())));
        }
        for b in spans.iter() {
            let inner = if let Some(ty) = &b.ty {
                quote! { #ty }
            } else {
                quote! { (usize, usize) }
            };
            pieces.push(wrap(inner));
        }
        if pieces.is_empty() {
            quote! { () }
        } else {
            quote! { ( #(#pieces,)* ) }
        }
    };

    let s_ty = build_bucket(&registry.single_values, &registry.single_spans, false);
    let m_ty = build_bucket(&registry.multiple_values, &registry.multiple_spans, true);
    let o_ty = build_bucket(&registry.optional_values, &registry.optional_spans, false);
    (quote! { (#s_ty, #m_ty, #o_ty) }, gen_names)
}

fn snapshot_bind_lets(
    registry: &BindRegistry,
) -> (
    Vec<proc_macro2::TokenStream>,
    Vec<proc_macro2::TokenStream>,
    Vec<proc_macro2::TokenStream>,
) {
    let mut single_lets = Vec::new();
    for (i, b) in registry
        .single_values
        .iter()
        .chain(&registry.single_spans)
        .enumerate()
    {
        let idx = Index::from(i);
        let id = &b.ident;
        single_lets.push(quote! {
            #[allow(unused_variables)]
            let #id = __single.#idx;
        });
    }
    let mut multiple_lets = Vec::new();
    for (i, b) in registry
        .multiple_values
        .iter()
        .chain(&registry.multiple_spans)
        .enumerate()
    {
        let idx = Index::from(i);
        let id = &b.ident;
        multiple_lets.push(quote! {
            #[allow(unused_variables)]
            let #id = &__multiple.#idx;
        });
    }
    let mut optional_lets = Vec::new();
    for (i, b) in registry
        .optional_values
        .iter()
        .chain(&registry.optional_spans)
        .enumerate()
    {
        let idx = Index::from(i);
        let id = &b.ident;
        optional_lets.push(quote! {
            #[allow(unused_variables)]
            let #id = __optional.#idx;
        });
    }
    (single_lets, multiple_lets, optional_lets)
}

/// Emit one `__UseBindsFactory<const SITE>` plus a single `BuildInlineError` impl that dispatches
/// on `SITE` (4B).
fn emit_use_binds_factory(
    sites: &[UseBindSite],
    registry: &BindRegistry,
    marser: &Path,
    mres: &proc_macro2::TokenStream,
    mres_generics: &[Ident],
) -> proc_macro2::TokenStream {
    if sites.is_empty() {
        return quote! {};
    }

    let (single_lets, multiple_lets, optional_lets) = snapshot_bind_lets(registry);

    let const_site_params = quote! { <const __SITE: usize> };
    let build_inline_params = if mres_generics.is_empty() {
        quote! { <const __SITE: usize> }
    } else {
        quote! { <const __SITE: usize, #(#mres_generics),*> }
    };

    let arms: Vec<_> = sites
        .iter()
        .map(|s| {
            let lit = syn::LitInt::new(&format!("{}", s.site), Span::call_site());
            let ctx = &s.ctx_ident;
            let inner = &s.inner;
            quote! {
                #lit => {
                    let #ctx = __ctx;
                    #inner
                }
            }
        })
        .collect();

    quote! {
        struct __UseBindsFactory<const __SITE: usize>;

        impl #const_site_params ::core::clone::Clone for __UseBindsFactory<__SITE> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl #const_site_params ::core::marker::Copy for __UseBindsFactory<__SITE> {}

        impl #build_inline_params #marser::error::BuildInlineError<#mres> for __UseBindsFactory<__SITE> {
            fn build_inline_error<'__snap>(
                &self,
                __ctx: #marser::error::MatchDiagCtx,
                __snap: <#mres as #marser::parser::capture::MatchResult>::Snapshot<'__snap>,
            ) -> #marser::error::InlineError
            where
                #mres: '__snap,
            {
                let __single = &__snap.0;
                let __multiple = &__snap.1;
                let __optional = &__snap.2;
                #(#single_lets)*
                #(#multiple_lets)*
                #(#optional_lets)*
                match __SITE {
                    #(#arms)*
                    _ => ::core::unreachable!("use_binds site out of range"),
                }
            }
        }
    }
}

fn parse_use_binds_closure(closure: ExprClosure) -> std::result::Result<(Ident, proc_macro2::TokenStream), syn::Error> {
    let ctx_ident = match closure.inputs.iter().next() {
        Some(Pat::Type(pt)) => {
            if let Pat::Ident(pi) = pt.pat.as_ref() {
                pi.ident.clone()
            } else {
                Ident::new("ctx", Span::call_site())
            }
        }
        Some(Pat::Ident(pi)) => pi.ident.clone(),
        _ => Ident::new("ctx", Span::call_site()),
    };

    let inner = match closure.body.as_ref() {
        Expr::Block(b) => {
            let stmts = b.block.stmts.iter();
            quote! { #(#stmts)* }
        }
        expr => quote! { #expr },
    };
    Ok((ctx_ident, inner))
}

struct UseBindsRewriter {
    sites: RefCell<Vec<UseBindSite>>,
    next_site: Cell<usize>,
    errors: RefCell<Option<syn::Error>>,
}

impl VisitMut for UseBindsRewriter {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Expr::Macro(m) = expr
            && m.mac.path.is_ident("use_binds")
        {
            let closure = match m.mac.parse_body::<ExprClosure>() {
                Ok(c) => c,
                Err(e) => {
                    let mut slot = self.errors.borrow_mut();
                    *slot = Some(match slot.take() {
                        None => e,
                        Some(mut prev) => {
                            prev.combine(e);
                            prev
                        }
                    });
                    return;
                }
            };
            let (ctx_ident, inner) = match parse_use_binds_closure(closure) {
                Ok(x) => x,
                Err(e) => {
                    let mut slot = self.errors.borrow_mut();
                    *slot = Some(match slot.take() {
                        None => e,
                        Some(mut prev) => {
                            prev.combine(e);
                            prev
                        }
                    });
                    return;
                }
            };
            let site = self.next_site.get();
            self.next_site.set(site + 1);
            self.sites.borrow_mut().push(UseBindSite {
                site,
                ctx_ident,
                inner,
            });
            let lit = syn::LitInt::new(&format!("{}", site), Span::call_site());
            match syn::parse2::<Expr>(quote! { __UseBindsFactory::<#lit> }) {
                Ok(expanded) => *expr = expanded,
                Err(e) => {
                    let mut slot = self.errors.borrow_mut();
                    *slot = Some(match slot.take() {
                        None => e,
                        Some(mut prev) => {
                            prev.combine(e);
                            prev
                        }
                    });
                }
            }
            return;
        }
        visit_mut::visit_expr_mut(self, expr);
    }
}

// ---------------------------------------------------------------------------
// Bind macro expansion
// ---------------------------------------------------------------------------

/// Expands `bind!` / `bind_span!` / `bind_slice!` inside `capture!` after [`BindCollector`] validation.
struct BindMacroExpander {
    marser_path: Path,
    errors: Option<syn::Error>,
}

impl BindMacroExpander {
    fn new(marser_path: Path) -> Self {
        Self {
            marser_path,
            errors: None,
        }
    }

    fn bump_err(&mut self, e: syn::Error) {
        self.errors = Some(match self.errors.take() {
            None => e,
            Some(mut prev) => {
                prev.combine(e);
                prev
            }
        });
    }

    fn take_errors(self) -> Option<syn::Error> {
        self.errors
    }
}

impl VisitMut for BindMacroExpander {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        if let Expr::Macro(m) = i {
            if m.mac.path.is_ident("bind") {
                let info = match m.mac.parse_body::<BindInfo>() {
                    Ok(info) => info,
                    Err(e) => {
                        self.bump_err(e);
                        visit_mut::visit_expr_mut(self, i);
                        return;
                    }
                };
                let id = &info.ident;
                let parser = &info.parser;
                let bind_span = id.span();
                let rewrite = if let Some(span_id) = &info.span_ident {
                    let marser = self.marser_path.clone();
                    let span_id = span_id;
                    quote_spanned! {bind_span=>
                        #marser::parser::capture::bind_span(
                            #marser::parser::capture::bind_result(#parser, #id),
                            #span_id
                        )
                    }
                } else {
                    let marser = self.marser_path.clone();
                    quote_spanned! {bind_span=>
                        #marser::parser::capture::bind_result(#parser, #id)
                    }
                };
                match syn::parse2(rewrite) {
                    Ok(expr) => *i = expr,
                    Err(e) => self.bump_err(e),
                }
                return;
            }

            if m.mac.path.is_ident("bind_span") {
                let info = match m.mac.parse_body::<BindSpanInfo>() {
                    Ok(i) => i,
                    Err(e) => {
                        self.bump_err(e);
                        visit_mut::visit_expr_mut(self, i);
                        return;
                    }
                };
                let span_id = &info.span_ident;
                let parser = &info.parser;
                let marser = self.marser_path.clone();
                *i = parse_quote! { #marser::parser::capture::bind_span(#parser, #span_id) };
                return;
            }

            if m.mac.path.is_ident("bind_slice") {
                let info = match m.mac.parse_body::<BindSliceInfo>() {
                    Ok(i) => i,
                    Err(e) => {
                        self.bump_err(e);
                        visit_mut::visit_expr_mut(self, i);
                        return;
                    }
                };
                let slice_id = &info.slice_ident;
                let parser = &info.parser;
                let marser = self.marser_path.clone();
                *i = parse_quote! { #marser::parser::capture::bind_slice(#parser, #slice_id) };
                return;
            }
        }
        visit_mut::visit_expr_mut(self, i);
    }
}

// ---------------------------------------------------------------------------
// capture! proc-macro
// ---------------------------------------------------------------------------

/// Build a parser from grammar + result expressions (via [`Capture`](https://docs.rs/marser/latest/marser/parser/capture/struct.Capture.html), returned as `impl Parser` to limit type size).
///
/// # Syntax
///
/// ```rust,ignore
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
/// - **`use_binds!(move \|ctx: MatchDiagCtx\| { … })`** — expands to `__UseBindsFactory::<N>`; the
///   `capture!` expansion emits one shared `__UseBindsFactory<const SITE>` implementing
///   [`marser::error::BuildInlineError`] with `match SITE` dispatch so multiple sites share one type.
///
/// Repeated **compatible** binds to the same identifier (same sigil bucket and compatible `as`
/// types) are merged into one capture slot. Conflicting sigils, incompatible explicit types, or
/// reusing the same name for both value and span captures are **compile errors** with spans on the
/// offending `bind!` / `bind_span!` / `bind_slice!` sites.
///
/// These helper macros are only expanded meaningfully inside `capture!`; using them elsewhere
/// yields normal unresolved-macro errors unless you import the `marser` crate and use `capture!`
/// from it (or depend on `marser_macros` directly for experimentation).
///
/// Each binding becomes a parameter to both the grammar closure and the result closure; the grammar
/// side often ignores those names because wiring goes through `bind_result` / `bind_span`.
///
/// The expansion prefixes APIs with the dependency name from Cargo (via `proc_macro_crate::crate_name("marser")`).
/// If you rename the `marser` dependency in your `Cargo.toml`, generated paths use that name.
#[proc_macro]
pub fn capture(input: TokenStream) -> TokenStream {
    let mut input: CaptureInput = syn::parse_macro_input!(input as CaptureInput);
    let marser_path = marser_crate_path();
    let registry = match BindCollector::collect(&input.grammar) {
        Ok(r) => r,
        Err(e) => return e.to_compile_error().into(),
    };

    let mut expander = BindMacroExpander::new(marser_path.clone());
    expander.visit_expr_mut(&mut input.grammar);
    if let Some(e) = expander.take_errors() {
        return e.to_compile_error().into();
    }

    let pat_tuple = |values: &[TypedBinding], spans: &[TypedBinding]| {
        let all: Vec<_> = values.iter().chain(spans.iter()).map(|x| &x.ident).collect();
        if all.is_empty() {
            quote! { () }
        } else {
            quote! { ( #(#all,)* ) }
        }
    };

    let s_pat = pat_tuple(&registry.single_values, &registry.single_spans);
    let m_pat = pat_tuple(&registry.multiple_values, &registry.multiple_spans);
    let o_pat = pat_tuple(&registry.optional_values, &registry.optional_spans);

    let mres_capture = build_capture_mres_tuple(&registry);
    let (mres_factory, mres_generics) = build_factory_mres_tuple(&registry);

    let mut use_binds_rw = UseBindsRewriter {
        sites: RefCell::new(Vec::new()),
        next_site: Cell::new(0),
        errors: RefCell::new(None),
    };
    use_binds_rw.visit_expr_mut(&mut input.grammar);
    use_binds_rw.visit_expr_mut(&mut input.result_expr);
    if let Some(e) = use_binds_rw.errors.into_inner() {
        return e.to_compile_error().into();
    }
    let sites = use_binds_rw.sites.into_inner();

    let factory_block = emit_use_binds_factory(
        &sites,
        &registry,
        &marser_path,
        &mres_factory,
        &mres_generics,
    );

    let grammar = &input.grammar;
    let result_expr = &input.result_expr;

    TokenStream::from(quote! {
        {
            #factory_block
            #[allow(unused_variables)]
            #marser_path::parser::as_parser(
                #marser_path::parser::capture::Capture::<#mres_capture, _, _>::new(
                    |#s_pat, #m_pat, #o_pat| { #grammar     },
                    |#s_pat, #m_pat, #o_pat| { #result_expr },
                ),
            )
        }
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
