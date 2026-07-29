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
//! (`State::Active(ActiveState)`) is a composite state — its leaves are
//! `Active/Loading`, `Active/Ready`, ... statechart-style nesting encoded as
//! `/`-separated paths — but only when the code shows positive evidence of
//! treating the child as a sub-state: a nested variant pattern like
//! `State::Active(ActiveState::Loading)` somewhere in the crate. Without
//! that evidence the field is payload data (`State::Failed(ErrorCode)`) and
//! the variant stays a plain leaf.

use std::collections::BTreeSet;

use syn::visit::Visit;

use crate::ast_util::{as_matches_macro, enum_variant_of_expr, enum_variant_path};
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

/// Machines detected in the crate, plus the set of enums whose variants are
/// dispatched on in patterns (used to tell nested event enums from payload
/// data enums).
pub(crate) struct Detection {
    pub machines: Vec<StateMachine>,
    pub dispatched_enums: BTreeSet<String>,
}

pub(crate) fn find_state_machines(index: &CrateIndex) -> Detection {
    let mut collector = Collector {
        index,
        assigned: BTreeSet::new(),
        nested_patterns: BTreeSet::new(),
        dispatched_enums: BTreeSet::new(),
    };
    for fn_info in &index.fns {
        collector.visit_block(fn_info.block);
    }

    let nested_patterns = collector.nested_patterns;
    let dispatched_enums = collector.dispatched_enums;
    let machines = collector
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
                .map(|decl| expand_leaves(&enum_name, &decl, index, &nested_patterns))
                .unwrap_or_default();
            StateMachine {
                enum_name,
                field_name,
                variants,
                composites,
            }
        })
        .collect();

    Detection {
        machines,
        dispatched_enums,
    }
}

/// Expands composite variants into `Parent/Child` leaves. A variant is only
/// composite when a nested variant pattern (`Parent::Variant(Child::X)`)
/// exists somewhere in the crate — sub-state evidence, mirroring the
/// assignment-evidence rule for machines themselves.
fn expand_leaves(
    enum_name: &str,
    decl: &EnumDecl,
    index: &CrateIndex,
    nested_patterns: &BTreeSet<(String, String, String)>,
) -> (Vec<String>, Vec<(String, String)>) {
    let mut leaves = Vec::new();
    let mut composites = Vec::new();

    for (position, variant) in decl.variants.iter().enumerate() {
        let child = composite_child_enum(decl, position, index).filter(|(child_enum, _)| {
            nested_patterns.contains(&(
                enum_name.to_string(),
                variant.clone(),
                child_enum.clone(),
            ))
        });
        match child {
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
    /// Nested variant patterns seen anywhere: (parent enum, variant, child enum).
    nested_patterns: BTreeSet<(String, String, String)>,
    /// Enums whose variants appear in any pattern.
    dispatched_enums: BTreeSet<String>,
}

impl<'a> Collector<'a> {
    /// Records `Parent::Variant(Child::X ..)` pattern nesting as sub-state
    /// evidence.
    fn record_nested_patterns(&mut self, pat: &syn::Pat) {
        match pat {
            syn::Pat::TupleStruct(tuple) => {
                if let Some((parent, variant)) = enum_variant_path(&tuple.path) {
                    let mut inner = Vec::new();
                    for element in &tuple.elems {
                        crate::ast_util::pattern_variants(element, &mut inner);
                    }
                    for (child_enum, _) in inner {
                        self.nested_patterns.insert((parent.clone(), variant.clone(), child_enum));
                    }
                }
                for element in &tuple.elems {
                    self.record_nested_patterns(element);
                }
            }
            syn::Pat::Or(or) => {
                for case in &or.cases {
                    self.record_nested_patterns(case);
                }
            }
            syn::Pat::Ident(ident) => {
                if let Some((_, subpat)) = &ident.subpat {
                    self.record_nested_patterns(subpat);
                }
            }
            syn::Pat::Paren(paren) => self.record_nested_patterns(&paren.pat),
            syn::Pat::Reference(reference) => self.record_nested_patterns(&reference.pat),
            _ => {}
        }
    }

    fn record_dispatched(&mut self, pat: &syn::Pat) {
        let mut variants = Vec::new();
        crate::ast_util::pattern_variants(pat, &mut variants);
        for (enum_name, variant) in variants {
            if self
                .index
                .enum_decls(&enum_name)
                .iter()
                .any(|decl| decl.has_variant(&variant))
            {
                self.dispatched_enums.insert(enum_name);
            }
        }
    }
}

impl<'a, 'ast> Visit<'ast> for Collector<'a> {
    fn visit_pat(&mut self, pat: &'ast syn::Pat) {
        self.record_nested_patterns(pat);
        self.record_dispatched(pat);
        syn::visit::visit_pat(self, pat);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        // `matches!` patterns are token soup to the visitor — parse them.
        if let Some(args) = as_matches_macro(mac) {
            self.record_nested_patterns(&args.pat);
            self.record_dispatched(&args.pat);
        }
        syn::visit::visit_macro(self, mac);
    }

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
