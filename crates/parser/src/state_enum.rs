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
use std::path::PathBuf;

use syn::visit::Visit;

use crate::annotations::DocBlock;
use crate::ast_util::{as_matches_macro, enum_variant_of_expr, enum_variant_path};
use crate::index::{CrateIndex, EnumDecl};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StateMachine {
    pub enum_name: String,
    pub field_name: String,
    /// Leaf state names, in declaration order. Children of composite
    /// variants appear as `Parent/Child` paths.
    ///
    /// Stays a plain `Vec<String>`: this is the analysis vocabulary that
    /// `transitions.rs` resolves patterns against, so documentation rides
    /// alongside in `variant_docs` rather than inside it.
    pub variants: Vec<String>,
    /// Documentation for each leaf in `variants`. Parallel vector.
    pub variant_docs: Vec<DocBlock>,
    /// Documentation authored on the state enum itself — becomes the machine's.
    pub docs: DocBlock,
    /// File the chosen declaration came from, for diagnostics.
    pub file: PathBuf,
    /// Composite variants: (variant name, child enum name).
    pub composites: Vec<(String, String)>,
    /// The leaf the enum declares as its `#[default]` variant, when it declares
    /// one and that variant is a leaf of this machine.
    ///
    /// Always a top-level variant name in practice: `#[derive(Default)]` only
    /// accepts `#[default]` on a *unit* variant, and a composite variant holds a
    /// child enum, so a composite can never be the declared default. The
    /// membership check keeps that an observation rather than an assumption —
    /// input that says otherwise leaves this `None` instead of naming a state
    /// the model does not declare. A `#[default]` on the *child* enum of a
    /// composite is not read: it says which sub-state `Active` starts in, not
    /// where the machine starts.
    pub default_state: Option<String>,
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
    /// `(enum, variant)` pairs whose arm binds the payload and hands it straight
    /// on — `Event::Session(event) => Self::update_session(event, …)` or
    /// `=> match event { … }`. That is what delegation looks like, and it is the
    /// only thing that makes a payload enum an event enum in its own right.
    pub delegating_variants: BTreeSet<(String, String)>,
}

pub(crate) fn find_state_machines(index: &CrateIndex) -> Detection {
    let mut collector = Collector {
        index,
        assigned: BTreeSet::new(),
        value_flow_fields: BTreeSet::new(),
        nested_patterns: BTreeSet::new(),
        dispatched_enums: BTreeSet::new(),
        delegating_variants: BTreeSet::new(),
    };
    for fn_info in &index.fns {
        collector.visit_block(fn_info.block);
    }

    let nested_patterns = collector.nested_patterns;
    let dispatched_enums = collector.dispatched_enums;
    let delegating_variants = collector.delegating_variants;
    let value_flow_fields = collector.value_flow_fields;
    let mut assigned = collector.assigned;

    // Value-flow evidence. A field written from an event payload or cloned from
    // another field never names its enum at the assignment, so the field name
    // alone is not evidence — it becomes evidence when the `Model` holds a
    // field of that name typed as an enum the crate dispatches on. See
    // `docs/roadmap.md` §6.
    for (enum_name, field_name) in model_reachable_enum_fields(index) {
        if value_flow_fields.contains(&field_name) && dispatched_enums.contains(&enum_name) {
            assigned.insert((enum_name, field_name));
        }
    }

    let machines = assigned
        .into_iter()
        .map(|(enum_name, field_name)| {
            // With colliding names, prefer the declaration with most
            // variants (state enums are the ones driven exhaustively).
            let decl = index
                .enum_decls(&enum_name)
                .iter()
                .max_by_key(|decl| decl.variants.len())
                .cloned();
            let docs = decl
                .as_ref()
                .map(|decl| decl.docs.clone())
                .unwrap_or_default();
            let file = decl
                .as_ref()
                .map(|decl| decl.file.clone())
                .unwrap_or_default();
            let declared_default = decl.as_ref().and_then(|decl| decl.default_variant.clone());
            let leaves = decl
                .map(|decl| expand_leaves(&enum_name, &decl, index, &nested_patterns))
                .unwrap_or_default();
            let default_state =
                declared_default.filter(|variant| leaves.names.iter().any(|leaf| leaf == variant));
            StateMachine {
                enum_name,
                field_name,
                variants: leaves.names,
                variant_docs: leaves.docs,
                docs,
                file,
                composites: leaves.composites,
                default_state,
            }
        })
        .collect();

    Detection {
        machines,
        dispatched_enums,
        delegating_variants,
    }
}

/// `(enum, field)` pairs for every enum-typed field reachable from a Core's
/// `Model` associated type, following struct fields through the containers that
/// hold them (`Vec<Entry>` → `Entry`, so a status held per entity counts).
///
/// This is what makes value-flow assignment safe to accept as evidence.
/// ViewModel mirror enums are *constructed* into view structs and never held by
/// the model, so they are not reachable and stay out — the same exclusion the
/// literal-assignment rule achieves, without a naming heuristic.
///
/// Only struct fields are followed. A struct sitting behind an enum variant is
/// not reached, which is the next widening if a real application wants it. The
/// walk needs no depth cap: `visited` admits each type once and the type graph
/// is finite, so a cyclic model terminates.
fn model_reachable_enum_fields(index: &CrateIndex) -> BTreeSet<(String, String)> {
    let mut found = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut queue: Vec<String> = index
        .trait_impls
        .iter()
        .filter(|imp| imp.trait_name == "App")
        .filter_map(|imp| crate::core_finder::associated_type(imp.item, "Model"))
        .collect();

    while let Some(type_name) = queue.pop() {
        if !visited.insert(type_name.clone()) {
            continue;
        }
        let Some(strct) = index.structs.get(&type_name) else {
            continue;
        };
        for field in &strct.fields {
            if !index.enum_decls(&field.reachable).is_empty() {
                found.insert((field.reachable.clone(), field.name.clone()));
            }
            queue.push(field.reachable.clone());
        }
    }
    found
}

/// The leaves of a state enum: their names, the documentation authored on
/// each, and which variants turned out to be composite. Three parallel results
/// is where a tuple stops being readable.
#[derive(Default)]
struct Leaves {
    names: Vec<String>,
    /// Parallel to `names`.
    docs: Vec<DocBlock>,
    composites: Vec<(String, String)>,
}

/// Expands composite variants into `Parent/Child` leaves. A variant is only
/// composite when a nested variant pattern (`Parent::Variant(Child::X)`)
/// exists somewhere in the crate — sub-state evidence, mirroring the
/// assignment-evidence rule for machines themselves.
///
/// A child leaf inherits its parent variant's documentation (see
/// [`DocBlock::inherit`]): the parent has no node of its own in the model, so
/// anything written on it would otherwise be lost.
fn expand_leaves(
    enum_name: &str,
    decl: &EnumDecl,
    index: &CrateIndex,
    nested_patterns: &BTreeSet<(String, String, String)>,
) -> Leaves {
    let mut leaves = Leaves::default();

    for (position, variant) in decl.variants.iter().enumerate() {
        let child = composite_child_enum(decl, position, index).filter(|(child_enum, _)| {
            nested_patterns.contains(&(enum_name.to_string(), variant.clone(), child_enum.clone()))
        });
        match child {
            Some((child_enum, child_decl)) => {
                leaves.composites.push((variant.clone(), child_enum));
                let parent_docs = decl.docs_of(position);
                for (child_position, child) in child_decl.variants.iter().enumerate() {
                    leaves.names.push(format!("{variant}/{child}"));
                    leaves
                        .docs
                        .push(child_decl.docs_of(child_position).inherit(parent_docs));
                }
            }
            None => {
                leaves.names.push(variant.clone());
                leaves.docs.push(decl.docs_of(position).clone());
            }
        }
    }
    leaves
}

/// `Active(ActiveState)` → the child enum, when the variant has exactly one
/// unnamed field typed as a crate enum.
fn composite_child_enum(
    decl: &EnumDecl,
    position: usize,
    index: &CrateIndex,
) -> Option<(String, EnumDecl)> {
    let fields = &decl.variant_fields[position];
    let [field] = fields.as_slice() else {
        return None;
    };
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
    /// (enum, field) pairs with literal assignment evidence.
    assigned: BTreeSet<(String, String)>,
    /// Field names assigned from something other than a recognized variant
    /// path — an event payload, a `.clone()` of another field. Half-evidence:
    /// the type is missing, and model reachability supplies it.
    value_flow_fields: BTreeSet<String>,
    /// Nested variant patterns seen anywhere: (parent enum, variant, child enum).
    nested_patterns: BTreeSet<(String, String, String)>,
    /// Enums whose variants appear in any pattern.
    dispatched_enums: BTreeSet<String>,
    /// See [`Detection::delegating_variants`].
    delegating_variants: BTreeSet<(String, String)>,
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
                        self.nested_patterns
                            .insert((parent.clone(), variant.clone(), child_enum));
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

    /// Records `E::V(x) => match x { … }` and `E::V(x) => f(x, …)` as
    /// delegation: the arm does not handle the event, it hands the payload on.
    fn record_delegation(&mut self, arm: &syn::Arm) {
        let syn::Pat::TupleStruct(tuple) = strip_pattern(&arm.pat) else {
            return;
        };
        let Some((enum_name, variant)) = enum_variant_path(&tuple.path) else {
            return;
        };
        let elems: Vec<&syn::Pat> = tuple.elems.iter().collect();
        let [syn::Pat::Ident(binding)] = elems[..] else {
            return;
        };
        if binding.subpat.is_some() {
            return;
        }
        if body_hands_on(&arm.body, &binding.ident.to_string()) {
            self.delegating_variants.insert((enum_name, variant));
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

/// Peels the wrappers a pattern can be written behind.
fn strip_pattern(pat: &syn::Pat) -> &syn::Pat {
    match pat {
        syn::Pat::Paren(paren) => strip_pattern(&paren.pat),
        syn::Pat::Reference(reference) => strip_pattern(&reference.pat),
        other => other,
    }
}

/// The binding, however it was written on the way out: `inner`, `*inner`,
/// `&inner`, `inner.clone()`.
fn is_binding(expr: &syn::Expr, name: &str) -> bool {
    match expr {
        syn::Expr::Path(path) => path.path.is_ident(name),
        syn::Expr::Unary(unary) => is_binding(&unary.expr, name),
        syn::Expr::Reference(reference) => is_binding(&reference.expr, name),
        syn::Expr::Paren(paren) => is_binding(&paren.expr, name),
        syn::Expr::Group(group) => is_binding(&group.expr, name),
        syn::Expr::MethodCall(call) => is_binding(&call.receiver, name),
        _ => false,
    }
}

/// Whether an arm body matches on `name` or passes it to a call — the two ways
/// an arm delegates instead of handling.
fn body_hands_on(body: &syn::Expr, name: &str) -> bool {
    match body {
        syn::Expr::Match(expr_match) => is_binding(&expr_match.expr, name),
        syn::Expr::Call(call) => call.args.iter().any(|arg| is_binding(arg, name)),
        syn::Expr::MethodCall(call) => {
            is_binding(&call.receiver, name) || call.args.iter().any(|arg| is_binding(arg, name))
        }
        syn::Expr::Block(block) => block.block.stmts.iter().any(|stmt| match stmt {
            syn::Stmt::Expr(expr, _) => body_hands_on(expr, name),
            _ => false,
        }),
        syn::Expr::Paren(paren) => body_hands_on(&paren.expr, name),
        syn::Expr::Group(group) => body_hands_on(&group.expr, name),
        _ => false,
    }
}

impl<'a, 'ast> Visit<'ast> for Collector<'a> {
    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        self.record_delegation(arm);
        syn::visit::visit_arm(self, arm);
    }

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
        if let Some(field) = crate::ast_util::last_field_name(&assign.left) {
            // Direct: `*.field = Enum::Variant` (any construction form).
            let literal = enum_variant_of_expr(&assign.right)
                .filter(|(enum_name, variant)| {
                    self.index
                        .enum_decls(enum_name)
                        .iter()
                        .any(|decl| decl.has_variant(variant))
                })
                .map(|(enum_name, _)| enum_name);
            match literal {
                Some(enum_name) => {
                    self.assigned.insert((enum_name, field));
                }
                // Everything else assigned into a field: a payload binding, a
                // clone of another field, a call result. Resolved against the
                // model in `find_state_machines`.
                None => {
                    self.value_flow_fields.insert(field);
                }
            }
        }

        // Reset: `*.x = T::default()` assigns every enum-typed field of `T`.
        if let Some(type_name) = default_call_type(&assign.right) {
            if let Some(strct) = self.index.structs.get(&type_name) {
                for field in &strct.fields {
                    // The declared type, not the reachable one: `default()` on
                    // an `Option<E>` field is `None`, not a variant of `E`.
                    if !self.index.enum_decls(&field.declared).is_empty() {
                        self.assigned
                            .insert((field.declared.clone(), field.name.clone()));
                    }
                }
            }
        }

        syn::visit::visit_expr_assign(self, assign);
    }
}

/// `T::default()` → `T`.
fn default_call_type(expr: &syn::Expr) -> Option<String> {
    let syn::Expr::Call(call) = expr else {
        return None;
    };
    let syn::Expr::Path(path) = &*call.func else {
        return None;
    };
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
