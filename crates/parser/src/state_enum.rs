//! State machine detection by assignment analysis.
//!
//! An enum `E` and a field name `f` form a state machine when the crate
//! assigns that field somewhere — directly (`*.f = E::Variant`) or through a
//! struct reset (`*.x = T::default()` where `T` has a field `f: E`).
//! Assignment is the discriminating signal: ViewModel mirror enums are only
//! ever *constructed* into view structs, never assigned to a model field, so
//! they stay out without needing a match-usage requirement.
//!
//! # Composite (hierarchical) states
//!
//! A variant with exactly one unnamed field whose type is another crate enum
//! (`State::Active(ActiveState)`) is a composite state: its leaves are
//! `Active/Loading`, `Active/Ready`, ... — statechart-style nesting encoded
//! as `/`-separated paths in the contract.

use std::collections::BTreeSet;

use syn::visit::Visit;

use crate::ast_util::enum_variant_of_expr;
use crate::index::{CrateIndex, EnumDecl};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StateMachine {
    pub enum_name: String,
    pub field_name: String,
    /// Leaf state names, in declaration order. Children of composite
    /// variants appear as `Parent/Child` paths.
    pub variants: Vec<String>,
    /// Composite variants: (variant name, child enum name).
    pub composites: Vec<(String, String)>,
}

impl StateMachine {
    /// All leaves under a variant: the variant itself, or its children when
    /// it is composite.
    pub fn leaves_of(&self, variant: &str) -> Vec<String> {
        let prefix = format!("{variant}/");
        let children: Vec<String> = self
            .variants
            .iter()
            .filter(|leaf| leaf.starts_with(&prefix))
            .cloned()
            .collect();
        if children.is_empty() {
            vec![variant.to_string()]
        } else {
            children
        }
    }

    pub fn child_enum(&self, variant: &str) -> Option<&str> {
        self.composites
            .iter()
            .find(|(parent, _)| parent == variant)
            .map(|(_, child)| child.as_str())
    }
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
        .map(|(enum_name, field_name)| {
            // With colliding names, prefer the declaration with most
            // variants (state enums are the ones driven exhaustively).
            let decl = index
                .enum_decls(&enum_name)
                .iter()
                .max_by_key(|decl| decl.variants.len())
                .cloned();
            let (variants, composites) = decl
                .map(|decl| expand_leaves(&decl, index))
                .unwrap_or_default();
            StateMachine {
                enum_name,
                field_name,
                variants,
                composites,
            }
        })
        .collect()
}

/// Expands composite variants into `Parent/Child` leaves.
fn expand_leaves(decl: &EnumDecl, index: &CrateIndex) -> (Vec<String>, Vec<(String, String)>) {
    let mut leaves = Vec::new();
    let mut composites = Vec::new();

    for (position, variant) in decl.variants.iter().enumerate() {
        match composite_child_enum(decl, position, index) {
            Some((child_enum, child_decl)) => {
                composites.push((variant.clone(), child_enum));
                for child in &child_decl.variants {
                    leaves.push(format!("{variant}/{child}"));
                }
            }
            None => leaves.push(variant.clone()),
        }
    }
    (leaves, composites)
}

/// `Active(ActiveState)` → the child enum, when the variant has exactly one
/// unnamed field typed as a crate enum.
fn composite_child_enum(
    decl: &EnumDecl,
    position: usize,
    index: &CrateIndex,
) -> Option<(String, EnumDecl)> {
    let fields = &decl.variant_fields[position];
    let [field] = fields.as_slice() else { return None };
    if field.name.is_some() {
        return None;
    }
    let child = index
        .enum_decls(&field.type_name)
        .iter()
        .max_by_key(|d| d.variants.len())?
        .clone();
    Some((field.type_name.clone(), child))
}

struct Collector<'a> {
    index: &'a CrateIndex<'a>,
    /// (enum, field) pairs with assignment evidence.
    assigned: BTreeSet<(String, String)>,
}

impl<'a, 'ast> Visit<'ast> for Collector<'a> {
    fn visit_expr_assign(&mut self, assign: &'ast syn::ExprAssign) {
        // Direct: `*.field = Enum::Variant` (any construction form).
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
