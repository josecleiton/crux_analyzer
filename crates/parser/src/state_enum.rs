//! State machine detection by assignment analysis.
//!
//! An enum `E` and a field name `f` form a state machine when the crate
//! assigns that field somewhere — directly (`*.f = E::Variant`) or through a
//! struct reset (`*.x = T::default()` where `T` has a field `f: E`).
//! Assignment is the discriminating signal: ViewModel mirror enums are only
//! ever *constructed* into view structs, never assigned to a model field, so
//! they stay out without needing a match-usage requirement.

use std::collections::BTreeSet;

use syn::visit::Visit;

use crate::ast_util::enum_variant_of_expr;
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
    };
    for fn_info in &index.fns {
        collector.visit_block(fn_info.block);
    }

    collector
        .assigned
        .into_iter()
        .map(|(enum_name, field_name)| StateMachine {
            variants: index
                .enum_decls(&enum_name)
                .iter()
                // With colliding names, prefer the declaration with most
                // variants (state enums are the ones driven exhaustively).
                .max_by_key(|decl| decl.variants.len())
                .map(|decl| decl.variants.clone())
                .unwrap_or_default(),
            enum_name,
            field_name,
        })
        .collect()
}

struct Collector<'a> {
    index: &'a CrateIndex<'a>,
    /// (enum, field) pairs with assignment evidence.
    assigned: BTreeSet<(String, String)>,
}

impl<'a, 'ast> Visit<'ast> for Collector<'a> {
    fn visit_expr_assign(&mut self, assign: &'ast syn::ExprAssign) {
        // Direct: `*.field = Enum::Variant`.
        if let Some(field) = crate::ast_util::last_field_name(&assign.left) {
            if let Some((enum_name, variant)) = enum_variant_of_expr(&assign.right) {
                if self
                    .index
                    .enum_decls(&enum_name)
                    .iter()
                    .any(|decl| decl.has_variant(&variant))
                {
                    self.assigned.insert((enum_name, field));
                }
            }
        }

        // Reset: `*.x = T::default()` assigns every enum-typed field of `T`.
        if let Some(type_name) = default_call_type(&assign.right) {
            if let Some(strct) = self.index.structs.get(&type_name) {
                for (field_name, field_type) in &strct.fields {
                    if !self.index.enum_decls(field_type).is_empty() {
                        self.assigned.insert((field_type.clone(), field_name.clone()));
                    }
                }
            }
        }

        syn::visit::visit_expr_assign(self, assign);
    }
}

/// `T::default()` → `T`.
fn default_call_type(expr: &syn::Expr) -> Option<String> {
    let syn::Expr::Call(call) = expr else { return None };
    let syn::Expr::Path(path) = &*call.func else { return None };
    let segments: Vec<String> = path
        .path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    match segments.as_slice() {
        [.., type_name, method] if method == "default" => Some(type_name.clone()),
        _ => None,
    }
}
