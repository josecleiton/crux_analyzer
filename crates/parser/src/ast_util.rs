//! Small shared helpers for pattern/expression analysis.

use syn::parse::{Parse, ParseStream};

/// `Enum::Variant` (possibly `path::to::Enum::Variant`) → `(Enum, Variant)`.
pub(crate) fn enum_variant_path(path: &syn::Path) -> Option<(String, String)> {
    if path.segments.len() < 2 {
        return None;
    }
    let variant = path.segments.last()?.ident.to_string();
    let enum_name = path.segments[path.segments.len() - 2].ident.to_string();
    Some((enum_name, variant))
}

/// `Enum::Variant` value expressions in any construction form:
/// path (`State::Idle`), struct (`State::Busy { .. }`) or tuple call
/// (`State::Busy(..)`) → `(Enum, Variant)`.
pub(crate) fn enum_variant_of_expr(expr: &syn::Expr) -> Option<(String, String)> {
    match expr {
        syn::Expr::Path(path) => enum_variant_path(&path.path),
        syn::Expr::Struct(strct) => enum_variant_path(&strct.path),
        syn::Expr::Call(call) => {
            if let syn::Expr::Path(path) = &*call.func {
                enum_variant_path(&path.path)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Last field ident of an lvalue/scrutinee expression:
/// `model.recording.session.state` → `state`; a bare ident is returned as-is.
pub(crate) fn last_field_name(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Field(field) => match &field.member {
            syn::Member::Named(ident) => Some(ident.to_string()),
            syn::Member::Unnamed(_) => None,
        },
        syn::Expr::Path(path) => path.path.get_ident().map(|i| i.to_string()),
        syn::Expr::Reference(reference) => last_field_name(&reference.expr),
        syn::Expr::Paren(paren) => last_field_name(&paren.expr),
        syn::Expr::Unary(unary) => last_field_name(&unary.expr),
        _ => None,
    }
}

/// Dotted path of an lvalue-ish expression: `known.insight_status` →
/// `"known.insight_status"`. References, parens and `.clone()`-style calls
/// with no arguments are looked through.
pub(crate) fn expr_path_string(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Path(path) => Some(
            path.path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("."),
        ),
        syn::Expr::Field(field) => {
            let base = expr_path_string(&field.base)?;
            match &field.member {
                syn::Member::Named(ident) => Some(format!("{base}.{ident}")),
                syn::Member::Unnamed(index) => Some(format!("{base}.{}", index.index)),
            }
        }
        syn::Expr::Reference(reference) => expr_path_string(&reference.expr),
        syn::Expr::Paren(paren) => expr_path_string(&paren.expr),
        syn::Expr::MethodCall(call) if call.args.is_empty() => {
            // `.clone()`, `.to_owned()`, ... — the value is still the receiver's.
            expr_path_string(&call.receiver)
        }
        _ => None,
    }
}

/// The `expr, pat` arguments of a `matches!` invocation.
pub(crate) struct MatchesArgs {
    pub expr: syn::Expr,
    pub pat: syn::Pat,
}

impl Parse for MatchesArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let expr = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let pat = syn::Pat::parse_multi_with_leading_vert(input)?;
        // Optional trailing `if guard` — parsed and discarded.
        if input.peek(syn::Token![if]) {
            input.parse::<syn::Token![if]>()?;
            input.parse::<syn::Expr>()?;
        }
        Ok(MatchesArgs { expr, pat })
    }
}

/// Parses a macro as `matches!(expr, pat)` if it is one.
pub(crate) fn as_matches_macro(mac: &syn::Macro) -> Option<MatchesArgs> {
    if !mac.path.is_ident("matches") {
        return None;
    }
    syn::parse2::<MatchesArgs>(mac.tokens.clone()).ok()
}

/// Collects every `(Enum, Variant)` referenced by a pattern, walking through
/// or-patterns, bindings (`x @ pat`), parens, references and struct/tuple pats.
pub(crate) fn pattern_variants(pat: &syn::Pat, out: &mut Vec<(String, String)>) {
    match pat {
        syn::Pat::Path(path) => {
            if let Some(pair) = enum_variant_path(&path.path) {
                out.push(pair);
            }
        }
        syn::Pat::TupleStruct(tuple) => {
            if let Some(pair) = enum_variant_path(&tuple.path) {
                out.push(pair);
            }
            // Do not descend into element patterns here: nested variant paths
            // are resolved by the caller when it needs leaf events.
        }
        syn::Pat::Struct(strct) => {
            if let Some(pair) = enum_variant_path(&strct.path) {
                out.push(pair);
            }
        }
        syn::Pat::Or(or) => {
            for case in &or.cases {
                pattern_variants(case, out);
            }
        }
        syn::Pat::Ident(ident) => {
            if let Some((_, subpat)) = &ident.subpat {
                pattern_variants(subpat, out);
            }
        }
        syn::Pat::Paren(paren) => pattern_variants(&paren.pat, out),
        syn::Pat::Reference(reference) => pattern_variants(&reference.pat, out),
        _ => {}
    }
}

/// Whether a pattern matches anything (wildcard `_` or a bare binding).
pub(crate) fn is_catch_all(pat: &syn::Pat) -> bool {
    match pat {
        syn::Pat::Wild(_) => true,
        syn::Pat::Ident(ident) => ident.subpat.is_none(),
        syn::Pat::Paren(paren) => is_catch_all(&paren.pat),
        syn::Pat::Reference(reference) => is_catch_all(&reference.pat),
        _ => false,
    }
}
