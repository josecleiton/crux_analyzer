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
        syn::Expr::MethodCall(call)
            if call.args.is_empty() && is_identity_method(&call.method.to_string()) =>
        {
            // Only genuinely identity-preserving calls are looked through —
            // `.take()`/`.unwrap()`/business accessors yield different values
            // and must NOT alias to their receiver's path.
            expr_path_string(&call.receiver)
        }
        _ => None,
    }
}

/// The *receiver* of a field access: `draft.status` → `"draft"`,
/// `model.session.state` → `"model.session"`. `None` when there is no receiver
/// to speak of — a bare binding (`status`), or a base this walker cannot spell
/// as a path (`entries[0].status`).
///
/// The companion to [`last_field_name`], which answers *which* field and
/// deliberately forgets *whose*. Source-state evidence needs both.
pub(crate) fn receiver_path(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Field(field) => expr_path_string(&field.base),
        syn::Expr::Reference(reference) => receiver_path(&reference.expr),
        syn::Expr::Paren(paren) => receiver_path(&paren.expr),
        syn::Expr::Unary(unary) => receiver_path(&unary.expr),
        syn::Expr::MethodCall(call)
            if call.args.is_empty() && is_identity_method(&call.method.to_string()) =>
        {
            receiver_path(&call.receiver)
        }
        _ => None,
    }
}

/// Whether two receiver paths can denote the same object.
///
/// Exact match, or one a dotted suffix of the other: `session` and
/// `model.recording.session` are one object reached two ways, which is why this
/// cannot be string equality — a helper taking `&mut CaptureSession` writes
/// `session.state` under a guard its caller wrote on the long path.
///
/// `None` on either side means "could not tell", and stays permissive. The rule
/// exists to reject a *known* mismatch (a guard about a different record), not to
/// demand proof of identity before believing a guard.
pub(crate) fn receivers_may_alias(subject: Option<&str>, other: Option<&str>) -> bool {
    let (Some(subject), Some(other)) = (subject, other) else {
        return true;
    };
    subject == other
        || subject.ends_with(&format!(".{other}"))
        || other.ends_with(&format!(".{subject}"))
}

fn is_identity_method(name: &str) -> bool {
    matches!(
        name,
        "clone" | "to_owned" | "as_ref" | "as_mut" | "as_deref" | "borrow" | "borrow_mut"
    )
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

/// Splits a match arm into the pattern it matches and its guard predicate.
///
/// syn 3 moved the guard into the pattern grammar: `Some(x) if x > 0` parses as
/// a `Pat::Guard` wrapping the pattern, where syn 2 kept a separate `Arm::guard`
/// field. Every walker that reads an arm's pattern unwraps here, so a guarded
/// arm yields the same states, events and bindings as an unguarded one — the
/// alternative is a guard silently hiding its arm's evidence.
pub(crate) fn arm_pattern_and_guard(arm: &syn::Arm) -> (&syn::Pat, Option<&syn::Expr>) {
    match &arm.pat {
        syn::Pat::Guard(guard) => (&guard.pat, Some(&guard.guard)),
        pat => (pat, None),
    }
}

/// Collects every `(Enum, Variant)` referenced by a pattern, walking through
/// or-patterns, bindings (`x @ pat`), parens, references and struct/tuple pats.
pub(crate) fn pattern_variants(pat: &syn::Pat, out: &mut Vec<(String, String)>) {
    pattern_variants_at(pat, out, 0);
}

/// Patterns nest without limit — `&&&&(((x)))` is valid Rust — and this walker
/// follows that nesting, so hostile input would recurse until the stack ran
/// out. The cap is far above any pattern a person writes; past it the extra
/// nesting simply yields no variants. See `docs/security.md`.
const MAX_PATTERN_DEPTH: usize = 128;

fn pattern_variants_at(pat: &syn::Pat, out: &mut Vec<(String, String)>, depth: usize) {
    if depth >= MAX_PATTERN_DEPTH {
        return;
    }
    let pattern_variants = |pat: &syn::Pat, out: &mut Vec<(String, String)>| {
        pattern_variants_at(pat, out, depth + 1)
    };
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
        // A guard narrows *when* the arm runs, never *what* it matches.
        syn::Pat::Guard(guard) => pattern_variants(&guard.pat, out),
        _ => {}
    }
}

/// Whether a pattern matches anything (wildcard `_` or a bare binding).
///
/// A guard is deliberately not consulted: `_ if cond` counts as a catch-all
/// here, because callers use this to decide which states an arm *can* cover,
/// and the guard's outcome is exactly what static analysis cannot know.
pub(crate) fn is_catch_all(pat: &syn::Pat) -> bool {
    match pat {
        syn::Pat::Wild(_) => true,
        syn::Pat::Ident(ident) => ident.subpat.is_none(),
        syn::Pat::Paren(paren) => is_catch_all(&paren.pat),
        syn::Pat::Reference(reference) => is_catch_all(&reference.pat),
        syn::Pat::Guard(guard) => is_catch_all(&guard.pat),
        _ => false,
    }
}
