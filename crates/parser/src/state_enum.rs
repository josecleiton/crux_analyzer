//! State machine detection by assignment analysis.
//!
//! An enum `E` and a field name `f` form a state machine when the crate both
//! assigns `*.f = E::Variant` somewhere and matches `*.f` against `E`
//! patterns (via `match` or `matches!`). Requiring both signals keeps
//! ViewModel mirror enums (assigned nowhere) and plain data enums (matched
//! nowhere as a field) out.

use std::collections::BTreeSet;

use syn::visit::Visit;

use crate::ast_util::{as_matches_macro, enum_variant_of_expr, last_field_name, pattern_variants};
use crate::index::CrateIndex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StateMachine {
    pub enum_name: String,
    pub field_name: String,
    /// Variants of the state enum, in declaration order.
    pub variants: Vec<String>,
}

pub(crate) fn find_state_machines(index: &CrateIndex) -> Vec<StateMachine> {
    let mut collector = Collector {
        index,
        assigned: BTreeSet::new(),
        matched: BTreeSet::new(),
    };
    for fn_info in &index.fns {
        collector.visit_block(fn_info.block);
    }

    collector
        .assigned
        .intersection(&collector.matched)
        .map(|(enum_name, field_name)| StateMachine {
            enum_name: enum_name.clone(),
            field_name: field_name.clone(),
            // With colliding names, prefer the declaration with most variants
            // (states enums are matched exhaustively somewhere by definition).
            variants: index
                .enum_decls(enum_name)
                .iter()
                .max_by_key(|decl| decl.variants.len())
                .map(|decl| decl.variants.clone())
                .unwrap_or_default(),
        })
        .collect()
}

struct Collector<'a> {
    index: &'a CrateIndex<'a>,
    /// (enum, field) pairs seen as `*.field = Enum::Variant`.
    assigned: BTreeSet<(String, String)>,
    /// (enum, field) pairs seen as `match *.field { Enum::V.. }` or
    /// `matches!(*.field, Enum::V..)`.
    matched: BTreeSet<(String, String)>,
}

impl<'a> Collector<'a> {
    fn record_match(&mut self, scrutinee: &syn::Expr, pat: &syn::Pat) {
        let Some(field) = last_field_name(scrutinee) else {
            return;
        };
        let mut variants = Vec::new();
        pattern_variants(pat, &mut variants);
        for (enum_name, variant) in variants {
            if self.index.any_enum_has_variant(&enum_name, &variant) {
                self.matched.insert((enum_name, field.clone()));
            }
        }
    }
}

impl<'a, 'ast> Visit<'ast> for Collector<'a> {
    fn visit_expr_assign(&mut self, assign: &'ast syn::ExprAssign) {
        if let Some(field) = last_field_name(&assign.left) {
            if let Some((enum_name, variant)) = enum_variant_of_expr(&assign.right) {
                if self.index.any_enum_has_variant(&enum_name, &variant) {
                    self.assigned.insert((enum_name, field));
                }
            }
        }
        syn::visit::visit_expr_assign(self, assign);
    }

    fn visit_expr_match(&mut self, expr_match: &'ast syn::ExprMatch) {
        for arm in &expr_match.arms {
            self.record_match(&expr_match.expr, &arm.pat);
        }
        syn::visit::visit_expr_match(self, expr_match);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(args) = as_matches_macro(mac) {
            self.record_match(&args.expr, &args.pat);
        }
        syn::visit::visit_macro(self, mac);
    }
}
