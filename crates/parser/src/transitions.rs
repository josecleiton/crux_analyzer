//! Transition extraction: walks a Core's `update` and every helper it calls,
//! carrying the current event label(s) and the conditions in force, and
//! records a transition at each state assignment.
//!
//! Source states are resolved per machine at each assignment, from three
//! kinds of evidence:
//! - `matches!(state, ...)` guards and `if matches!` conditions (including
//!   `!matches!` complements and `&&`/`||` combinations)
//! - `match state { ... }` arms, with wildcards resolved to the complement
//!   of the variants matched by earlier arms
//! - predicate methods on the state enum (`state.has_capture_in_flight()`),
//!   resolved by analyzing the method's body
//!
//! An assignment with no state evidence at all fires from ANY state and is
//! emitted with the wildcard source `"*"`. Only assignments whose evidence
//! exists but cannot be resolved statically are dropped with a [`Warning`].

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use syn::spanned::Spanned;

use crate::ast_util::{
    as_matches_macro, enum_variant_of_expr, enum_variant_path, expr_path_string, is_catch_all,
    last_field_name, pattern_variants,
};
use crate::core_finder::CoreInfo;
use crate::index::CrateIndex;
use crate::state_enum::StateMachine;
use crate::Warning;

/// The wildcard source: the transition fires from any state.
pub(crate) const ANY_STATE: &str = "*";

/// Maximum predicate-method resolution depth (predicates calling predicates).
const MAX_PREDICATE_DEPTH: usize = 3;

/// A transition attributed to a specific state machine (enum + field —
/// the same enum can drive more than one machine through different fields).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawTransition {
    pub machine: String,
    pub field: String,
    pub from: String,
    pub event: String,
    pub to: String,
    /// Effects requested by the event arm this transition came from.
    pub effects: Vec<String>,
}

/// What a condition says about a machine's current state.
#[derive(Debug, Clone, PartialEq)]
enum GuardEval {
    /// The condition does not constrain this machine.
    NoConstraint,
    /// The machine is in one of these states.
    Known(Vec<String>),
    /// The condition constrains this machine but cannot be resolved.
    Unresolved,
}

#[derive(Clone)]
struct Ctx<'a> {
    /// Event labels currently in scope. `None` = not statically known.
    events: Option<Vec<String>>,
    /// Guard / `if` conditions currently in force.
    conditions: Vec<&'a syn::Expr>,
    /// Facts from `match`-on-state arms: (machine enum, field, possible states).
    facts: Vec<(String, String, Vec<String>)>,
    /// Bindings introduced by the current event arm's pattern:
    /// binding name → payload type (`Event::Updated { status }` → status:
    /// InsightStatus). Valid within the arm body only.
    payload_bindings: HashMap<String, String>,
    /// The event arm this code belongs to — transitions and effects found
    /// under the same arm are associated with each other.
    arm: usize,
}

pub(crate) fn extract(
    index: &CrateIndex,
    core: &CoreInfo,
    machines: &[StateMachine],
    warnings: &mut Vec<Warning>,
) -> Vec<RawTransition> {
    let Some(update) = index.find_fn(Some(&core.name), "update") else {
        warnings.push(Warning {
            file: PathBuf::new(),
            line: 0,
            message: format!("core {}: no `update` method found", core.name),
        });
        return Vec::new();
    };

    let mut walker = Walker {
        index,
        core,
        machines,
        warnings,
        out: Vec::new(),
        effects_by_arm: HashMap::new(),
        arm_counter: 0,
        call_stack: vec![(Some(core.name.clone()), "update".to_string())],
    };
    walker.walk_block(
        update.block,
        &Ctx {
            events: None,
            conditions: Vec::new(),
            facts: Vec::new(),
            payload_bindings: HashMap::new(),
            arm: 0,
        },
        update.self_ty.clone(),
        update.file,
    );

    // Attach the effects observed under each event arm to its transitions.
    let effects_by_arm = walker.effects_by_arm;
    let mut out = walker.out;
    for (transition, arm) in &mut out {
        if let Some(effects) = effects_by_arm.get(arm) {
            transition.effects = effects.clone();
        }
    }
    out.into_iter().map(|(transition, _)| transition).collect()
}

struct Walker<'w, 'a> {
    index: &'w CrateIndex<'a>,
    core: &'w CoreInfo,
    machines: &'w [StateMachine],
    warnings: &'w mut Vec<Warning>,
    out: Vec<(RawTransition, usize)>,
    /// Effect labels observed under each event arm.
    effects_by_arm: HashMap<usize, Vec<String>>,
    arm_counter: usize,
    /// Functions currently being walked — breaks recursion cycles while still
    /// allowing the same helper to be re-walked under a different context.
    call_stack: Vec<(Option<String>, String)>,
}

impl<'w, 'a> Walker<'w, 'a> {
    fn machine_for_field(&self, field: &str) -> Option<&'w StateMachine> {
        self.machines.iter().find(|m| m.field_name == field)
    }

    fn is_event_enum(&self, name: &str) -> bool {
        self.core.is_event_enum(name)
    }

    /// Whether an event-enum variant only wraps another event enum
    /// (`Event::Recording(RecordingEvent)`) — such variants delegate and are
    /// not event labels themselves.
    fn is_wrapper_variant(&self, enum_name: &str, variant: &str) -> bool {
        let Some(decl) = self.core.event_enums.get(enum_name) else {
            return false;
        };
        let Some(position) = decl.variants.iter().position(|v| v == variant) else {
            return false;
        };
        decl.field_types(position)
            .any(|field_type| self.is_event_enum(field_type))
    }

    // ---- walking ----------------------------------------------------------

    fn walk_block(
        &mut self,
        block: &'a syn::Block,
        ctx: &Ctx<'a>,
        self_ty: Option<String>,
        file: &Path,
    ) {
        // let-else statements narrow the context for the REST of the block:
        // `let Some(d) = list.find(|d| d.state == State::X) else { return }`
        // guarantees the closure's constraints below it.
        let mut running_ctx = ctx.clone();

        for stmt in &block.stmts {
            match stmt {
                syn::Stmt::Expr(expr, _) => self.walk_expr(expr, &running_ctx, &self_ty, file),
                syn::Stmt::Local(local) => {
                    if let Some(init) = &local.init {
                        self.walk_expr(&init.expr, &running_ctx, &self_ty, file);
                        if let Some((_, else_branch)) = &init.diverge {
                            self.walk_expr(else_branch, &running_ctx, &self_ty, file);
                            for closure_body in closure_bodies(&init.expr) {
                                running_ctx.conditions.push(closure_body);
                            }
                        }
                    }
                }
                syn::Stmt::Macro(_) | syn::Stmt::Item(_) => {}
            }
        }
    }

    fn walk_expr(
        &mut self,
        expr: &'a syn::Expr,
        ctx: &Ctx<'a>,
        self_ty: &Option<String>,
        file: &Path,
    ) {
        match expr {
            syn::Expr::Assign(assign) => {
                self.handle_assignment(assign, ctx, file);
                self.walk_expr(&assign.right, ctx, self_ty, file);
            }
            syn::Expr::Match(expr_match) => self.walk_match(expr_match, ctx, self_ty, file),
            syn::Expr::If(expr_if) => {
                // The condition holds inside the then-branch.
                let mut then_ctx = ctx.clone();
                then_ctx.conditions.push(&expr_if.cond);
                self.walk_expr(&expr_if.cond, ctx, self_ty, file);
                self.walk_block(&expr_if.then_branch, &then_ctx, self_ty.clone(), file);
                if let Some((_, else_expr)) = &expr_if.else_branch {
                    self.walk_expr(else_expr, ctx, self_ty, file);
                }
            }
            syn::Expr::Call(call) => {
                // `AudioOperation::Start(..)` — a tuple-variant effect.
                if let syn::Expr::Path(path) = &*call.func {
                    self.record_effect_path(&path.path, ctx);
                }
                for arg in &call.args {
                    self.walk_expr(arg, ctx, self_ty, file);
                }
                self.follow_call(&call.func, ctx, self_ty);
            }
            syn::Expr::MethodCall(method_call) => {
                self.walk_expr(&method_call.receiver, ctx, self_ty, file);
                for arg in &method_call.args {
                    self.walk_expr(arg, ctx, self_ty, file);
                }
            }
            syn::Expr::Block(block) => self.walk_block(&block.block, ctx, self_ty.clone(), file),
            syn::Expr::Unsafe(unsafe_block) => {
                self.walk_block(&unsafe_block.block, ctx, self_ty.clone(), file);
            }
            syn::Expr::Loop(loop_expr) => {
                self.walk_block(&loop_expr.body, ctx, self_ty.clone(), file);
            }
            syn::Expr::While(while_expr) => {
                self.walk_expr(&while_expr.cond, ctx, self_ty, file);
                self.walk_block(&while_expr.body, ctx, self_ty.clone(), file);
            }
            syn::Expr::ForLoop(for_expr) => {
                self.walk_expr(&for_expr.expr, ctx, self_ty, file);
                self.walk_block(&for_expr.body, ctx, self_ty.clone(), file);
            }
            syn::Expr::Paren(paren) => self.walk_expr(&paren.expr, ctx, self_ty, file),
            syn::Expr::Group(group) => self.walk_expr(&group.expr, ctx, self_ty, file),
            syn::Expr::Reference(reference) => self.walk_expr(&reference.expr, ctx, self_ty, file),
            syn::Expr::Return(ret) => {
                if let Some(inner) = &ret.expr {
                    self.walk_expr(inner, ctx, self_ty, file);
                }
            }
            syn::Expr::Binary(binary) => {
                self.walk_expr(&binary.left, ctx, self_ty, file);
                self.walk_expr(&binary.right, ctx, self_ty, file);
            }
            syn::Expr::Unary(unary) => self.walk_expr(&unary.expr, ctx, self_ty, file),
            syn::Expr::Let(let_expr) => self.walk_expr(&let_expr.expr, ctx, self_ty, file),
            syn::Expr::Try(try_expr) => self.walk_expr(&try_expr.expr, ctx, self_ty, file),
            syn::Expr::Await(await_expr) => self.walk_expr(&await_expr.base, ctx, self_ty, file),
            syn::Expr::Field(field) => self.walk_expr(&field.base, ctx, self_ty, file),
            // `AudioOperation::Pause` — a unit-variant effect.
            syn::Expr::Path(path) => self.record_effect_path(&path.path, ctx),
            syn::Expr::Struct(strct) => {
                // `SomeOperation::Variant { .. }` — a struct-variant effect.
                self.record_effect_path(&strct.path, ctx);
                for field in &strct.fields {
                    self.walk_expr(&field.expr, ctx, self_ty, file);
                }
            }
            syn::Expr::Tuple(tuple) => {
                for element in &tuple.elems {
                    self.walk_expr(element, ctx, self_ty, file);
                }
            }
            // Iterator closures can mutate state (`for_each(|d| d.state = ..)`).
            syn::Expr::Closure(closure) => self.walk_expr(&closure.body, ctx, self_ty, file),
            _ => {}
        }
    }

    /// Follows `Self::helper(...)`, `Type::helper(...)` or `helper(...)` into
    /// the callee's body, keeping the current context.
    fn follow_call(&mut self, func: &'a syn::Expr, ctx: &Ctx<'a>, self_ty: &Option<String>) {
        let syn::Expr::Path(path) = func else { return };
        let segments: Vec<String> = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();

        let (callee_self, callee_name) = match segments.as_slice() {
            [name] => (None, name.clone()),
            [ty, name] if ty == "Self" => (self_ty.clone(), name.clone()),
            [ty, name] => (Some(ty.clone()), name.clone()),
            _ => return,
        };

        let key = (callee_self.clone(), callee_name.clone());
        if self.call_stack.contains(&key) {
            return; // recursion cycle
        }
        let Some(callee) = self.index.find_fn(callee_self.as_deref(), &callee_name) else {
            // External function: `render()` is crux_core's render effect.
            if callee_self.is_none() && callee_name == "render" {
                self.record_effect(ctx, "Render".to_string());
            }
            return;
        };

        self.call_stack.push(key);
        self.walk_block(callee.block, ctx, callee.self_ty.clone(), callee.file);
        self.call_stack.pop();
    }

    // ---- match handling ----------------------------------------------------

    fn walk_match(
        &mut self,
        expr_match: &'a syn::ExprMatch,
        ctx: &Ctx<'a>,
        self_ty: &Option<String>,
        file: &Path,
    ) {
        // A match on the state field drives the state facts per arm.
        if let Some(field) = last_field_name(&expr_match.expr) {
            if let Some(machine) = self.machine_for_field(&field) {
                if self.arms_reference_enum(&expr_match.arms, &machine.enum_name) {
                    self.walk_match_on_state(expr_match, machine.clone(), ctx, self_ty, file);
                    return;
                }
            }
        }

        // A match whose arms reference event-enum variants drives the event context.
        if self.arms_reference_events(&expr_match.arms) {
            self.walk_match_on_event(expr_match, ctx, self_ty, file);
            return;
        }

        // Anything else: walk generically.
        self.walk_expr(&expr_match.expr, ctx, self_ty, file);
        for arm in &expr_match.arms {
            if let Some((_, guard)) = &arm.guard {
                self.walk_expr(guard, ctx, self_ty, file);
            }
            self.walk_expr(&arm.body, ctx, self_ty, file);
        }
    }

    fn arms_reference_enum(&self, arms: &[syn::Arm], enum_name: &str) -> bool {
        arms.iter().any(|arm| {
            let mut variants = Vec::new();
            pattern_variants(&arm.pat, &mut variants);
            variants.iter().any(|(e, _)| e == enum_name)
        })
    }

    fn arms_reference_events(&self, arms: &[syn::Arm]) -> bool {
        arms.iter().any(|arm| {
            let mut variants = Vec::new();
            pattern_variants(&arm.pat, &mut variants);
            variants.iter().any(|(e, _)| self.is_event_enum(e))
        })
    }

    fn walk_match_on_state(
        &mut self,
        expr_match: &'a syn::ExprMatch,
        machine: StateMachine,
        ctx: &Ctx<'a>,
        self_ty: &Option<String>,
        file: &Path,
    ) {
        let mut seen: Vec<String> = Vec::new();

        for arm in &expr_match.arms {
            let arm_states = self.state_leaves_of_pattern(&arm.pat, &machine);

            let states = if !arm_states.is_empty() {
                arm_states
            } else if is_catch_all(&arm.pat) {
                // `_` matches whatever earlier arms did not.
                machine
                    .variants
                    .iter()
                    .filter(|v| !seen.contains(v))
                    .cloned()
                    .collect()
            } else {
                Vec::new()
            };

            seen.extend(states.iter().cloned());

            let mut arm_ctx = ctx.clone();
            if !states.is_empty() {
                arm_ctx
                    .facts
                    .push((machine.enum_name.clone(), machine.field_name.clone(), states));
            }
            if let Some((_, guard)) = &arm.guard {
                arm_ctx.conditions.push(guard);
                self.walk_expr(guard, ctx, self_ty, file);
            }
            self.walk_expr(&arm.body, &arm_ctx, self_ty, file);
        }
    }

    fn walk_match_on_event(
        &mut self,
        expr_match: &'a syn::ExprMatch,
        ctx: &Ctx<'a>,
        self_ty: &Option<String>,
        file: &Path,
    ) {
        for arm in &expr_match.arms {
            let labels = self.event_labels(&arm.pat);
            let events = match labels {
                EventLabels::Labels(labels) => Some(labels),
                // Wrapper variants (`Event::Recording(e)`) and catch-all
                // bindings delegate: the inner match resolves the label.
                EventLabels::Delegating => ctx.events.clone(),
                EventLabels::None => None,
            };

            let mut arm_ctx = ctx.clone();
            arm_ctx.events = events;
            // Each event arm gets its own effect scope and payload bindings.
            self.arm_counter += 1;
            arm_ctx.arm = self.arm_counter;
            arm_ctx.payload_bindings.extend(self.payload_bindings(&arm.pat));
            if let Some((_, guard)) = &arm.guard {
                arm_ctx.conditions.push(guard);
                self.walk_expr(guard, ctx, self_ty, file);
            }
            self.walk_expr(&arm.body, &arm_ctx, self_ty, file);
        }
    }

    /// Bindings introduced by an event-arm pattern, with their payload types:
    /// `Event::Updated { id, status }` → `{id: String, status: InsightStatus}`;
    /// `Event::Sync(state)` → `{state: State}` (positional).
    fn payload_bindings(&self, pat: &syn::Pat) -> HashMap<String, String> {
        let mut bindings = HashMap::new();
        self.collect_payload_bindings(pat, &mut bindings);
        bindings
    }

    fn collect_payload_bindings(&self, pat: &syn::Pat, out: &mut HashMap<String, String>) {
        match pat {
            syn::Pat::Struct(strct) => {
                let Some((enum_name, variant)) = enum_variant_path(&strct.path) else {
                    return;
                };
                let Some(fields) = self.variant_fields(&enum_name, &variant) else {
                    return;
                };
                for field_pat in &strct.fields {
                    let syn::Member::Named(member) = &field_pat.member else {
                        continue;
                    };
                    let syn::Pat::Ident(binding) = &*field_pat.pat else {
                        continue;
                    };
                    if let Some(field) = fields
                        .iter()
                        .find(|f| f.name.as_deref() == Some(member.to_string().as_str()))
                    {
                        out.insert(binding.ident.to_string(), field.type_name.clone());
                    }
                }
            }
            syn::Pat::TupleStruct(tuple) => {
                let Some((enum_name, variant)) = enum_variant_path(&tuple.path) else {
                    return;
                };
                let Some(fields) = self.variant_fields(&enum_name, &variant) else {
                    return;
                };
                for (position, element) in tuple.elems.iter().enumerate() {
                    if let (syn::Pat::Ident(binding), Some(field)) = (element, fields.get(position))
                    {
                        out.insert(binding.ident.to_string(), field.type_name.clone());
                    }
                }
            }
            syn::Pat::Or(or) => {
                for case in &or.cases {
                    self.collect_payload_bindings(case, out);
                }
            }
            syn::Pat::Ident(ident) => {
                if let Some((_, subpat)) = &ident.subpat {
                    self.collect_payload_bindings(subpat, out);
                }
            }
            syn::Pat::Paren(paren) => self.collect_payload_bindings(&paren.pat, out),
            syn::Pat::Reference(reference) => self.collect_payload_bindings(&reference.pat, out),
            _ => {}
        }
    }

    /// Fields of an event-enum variant.
    fn variant_fields(
        &self,
        enum_name: &str,
        variant: &str,
    ) -> Option<&[crate::index::VariantField]> {
        let decl = self.core.event_enums.get(enum_name)?;
        let position = decl.variants.iter().position(|v| v == variant)?;
        Some(&decl.variant_fields[position])
    }

    // ---- effect collection ----------------------------------------------

    /// Records `Enum::Variant` as an effect when `Enum` belongs to the core's
    /// effect closure.
    fn record_effect_path(&mut self, path: &syn::Path, ctx: &Ctx<'a>) {
        let Some((enum_name, variant)) = enum_variant_path(path) else {
            return;
        };
        if self.core.is_effect_enum(&enum_name) {
            self.record_effect(ctx, format!("{enum_name}::{variant}"));
        }
    }

    fn record_effect(&mut self, ctx: &Ctx<'a>, label: String) {
        let effects = self.effects_by_arm.entry(ctx.arm).or_default();
        if !effects.contains(&label) {
            effects.push(label);
        }
    }

    /// Event labels contributed by an event-arm pattern.
    fn event_labels(&self, pat: &syn::Pat) -> EventLabels {
        let mut variants = Vec::new();
        pattern_variants(pat, &mut variants);
        let event_variants: Vec<(String, String)> = variants
            .into_iter()
            .filter(|(e, _)| self.is_event_enum(e))
            .collect();

        if event_variants.is_empty() {
            return if is_catch_all(pat) {
                EventLabels::Delegating
            } else {
                EventLabels::None
            };
        }

        let mut labels = Vec::new();
        let mut delegating = false;
        for (enum_name, variant) in event_variants {
            if self.is_wrapper_variant(&enum_name, &variant) {
                delegating = true;
            } else {
                labels.push(variant);
            }
        }
        if labels.is_empty() && delegating {
            EventLabels::Delegating
        } else {
            EventLabels::Labels(labels)
        }
    }

    // ---- source-state evaluation --------------------------------------------

    /// Resolves the source states for `machine` from the context: facts from
    /// match-on-state arms first, then every condition in force.
    fn source_states(&self, ctx: &Ctx<'a>, machine: &StateMachine) -> GuardEval {
        let mut result = GuardEval::NoConstraint;

        for (fact_machine, fact_field, states) in &ctx.facts {
            if fact_machine == &machine.enum_name && fact_field == &machine.field_name {
                result = and(result, GuardEval::Known(states.clone()));
            }
        }
        for condition in &ctx.conditions {
            result = and(result, self.eval_condition(condition, machine, &["self"], 0));
        }
        result
    }

    /// What `condition` says about `machine`'s current state.
    ///
    /// `self_fields` are extra field spellings accepted as the state field —
    /// `"self"` inside predicate methods on the state enum.
    fn eval_condition(
        &self,
        condition: &syn::Expr,
        machine: &StateMachine,
        self_fields: &[&str],
        depth: usize,
    ) -> GuardEval {
        match condition {
            syn::Expr::Macro(expr_macro) => {
                let Some(args) = as_matches_macro(&expr_macro.mac) else {
                    return GuardEval::NoConstraint;
                };
                let Some(field) = last_field_name(&args.expr) else {
                    return GuardEval::NoConstraint;
                };
                if field != machine.field_name && !self_fields.contains(&field.as_str()) {
                    return GuardEval::NoConstraint;
                }
                let states = self.state_leaves_of_pattern(&args.pat, machine);
                if states.is_empty() {
                    GuardEval::NoConstraint
                } else {
                    GuardEval::Known(states)
                }
            }
            syn::Expr::MethodCall(call) => {
                let Some(field) = last_field_name(&call.receiver) else {
                    return GuardEval::NoConstraint;
                };
                if field != machine.field_name && !self_fields.contains(&field.as_str()) {
                    return GuardEval::NoConstraint;
                }
                self.eval_predicate(&call.method.to_string(), machine, depth)
            }
            syn::Expr::Binary(binary) => match binary.op {
                syn::BinOp::And(_) => and(
                    self.eval_condition(&binary.left, machine, self_fields, depth),
                    self.eval_condition(&binary.right, machine, self_fields, depth),
                ),
                syn::BinOp::Or(_) => or(
                    self.eval_condition(&binary.left, machine, self_fields, depth),
                    self.eval_condition(&binary.right, machine, self_fields, depth),
                ),
                // `state == State::X` and `state != State::X` comparisons.
                syn::BinOp::Eq(_) | syn::BinOp::Ne(_) => {
                    let Some(variant) =
                        self.comparison_variant(&binary.left, &binary.right, machine, self_fields)
                    else {
                        return GuardEval::NoConstraint;
                    };
                    if matches!(binary.op, syn::BinOp::Eq(_)) {
                        GuardEval::Known(vec![variant])
                    } else {
                        GuardEval::Known(
                            machine
                                .variants
                                .iter()
                                .filter(|v| **v != variant)
                                .cloned()
                                .collect(),
                        )
                    }
                }
                _ => GuardEval::NoConstraint,
            },
            // Constraints inside `if let Some(x) = list.find(|d| ...)`: the
            // closures' bodies hold for the bound element in the then-branch.
            syn::Expr::Let(let_expr) => closure_bodies(&let_expr.expr)
                .into_iter()
                .fold(GuardEval::NoConstraint, |acc, body| {
                    and(acc, self.eval_condition(body, machine, self_fields, depth))
                }),
            syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Not(_)) => {
                match self.eval_condition(&unary.expr, machine, self_fields, depth) {
                    GuardEval::Known(states) => GuardEval::Known(
                        machine
                            .variants
                            .iter()
                            .filter(|v| !states.contains(v))
                            .cloned()
                            .collect(),
                    ),
                    other => other,
                }
            }
            syn::Expr::Paren(paren) => {
                self.eval_condition(&paren.expr, machine, self_fields, depth)
            }
            syn::Expr::Group(group) => {
                self.eval_condition(&group.expr, machine, self_fields, depth)
            }
            // A block condition (e.g. a closure body) is its trailing expression.
            syn::Expr::Block(block) => match block.block.stmts.last() {
                Some(syn::Stmt::Expr(trailing, None)) => {
                    self.eval_condition(trailing, machine, self_fields, depth)
                }
                _ => GuardEval::NoConstraint,
            },
            _ => GuardEval::NoConstraint,
        }
    }

    /// `state == State::X` (either side order) → the state leaf, when the
    /// other side is the machine's state field.
    fn comparison_variant(
        &self,
        left: &syn::Expr,
        right: &syn::Expr,
        machine: &StateMachine,
        self_fields: &[&str],
    ) -> Option<String> {
        let is_state_field = |expr: &syn::Expr| {
            last_field_name(expr).is_some_and(|field| {
                field == machine.field_name || self_fields.contains(&field.as_str())
            })
        };

        if is_state_field(left) {
            self.state_leaf_of_expr(right, machine)
        } else if is_state_field(right) {
            self.state_leaf_of_expr(left, machine)
        } else {
            None
        }
    }

    /// All state leaves a pattern matches, resolving composite variants:
    /// `State::Idle` → `[Idle]`; `State::Active(ActiveState::Ready)` →
    /// `[Active/Ready]`; `State::Active(_)` → every `Active/*` leaf.
    fn state_leaves_of_pattern(&self, pat: &syn::Pat, machine: &StateMachine) -> Vec<String> {
        let is_machine_enum = |name: &str| name == machine.enum_name || name == "Self";
        match pat {
            syn::Pat::Path(path) => match enum_variant_path(&path.path) {
                Some((enum_name, variant)) if is_machine_enum(&enum_name) => {
                    machine.leaves_of(&variant)
                }
                _ => Vec::new(),
            },
            syn::Pat::Struct(strct) => match enum_variant_path(&strct.path) {
                Some((enum_name, variant)) if is_machine_enum(&enum_name) => {
                    machine.leaves_of(&variant)
                }
                _ => Vec::new(),
            },
            syn::Pat::TupleStruct(tuple) => {
                let Some((enum_name, variant)) = enum_variant_path(&tuple.path) else {
                    return Vec::new();
                };
                if !is_machine_enum(&enum_name) {
                    return Vec::new();
                }
                let Some(child_enum) = machine.child_enum(&variant) else {
                    return machine.leaves_of(&variant);
                };
                // Composite: resolve the child pattern.
                let mut child_variants = Vec::new();
                for element in &tuple.elems {
                    pattern_variants(element, &mut child_variants);
                }
                let children: Vec<String> = child_variants
                    .into_iter()
                    .filter(|(e, _)| *e == child_enum || e == "Self")
                    .map(|(_, v)| format!("{variant}/{v}"))
                    .collect();
                if children.is_empty() {
                    machine.leaves_of(&variant) // `Active(_)` → all children
                } else {
                    children
                }
            }
            syn::Pat::Or(or) => {
                let mut leaves = Vec::new();
                for case in &or.cases {
                    for leaf in self.state_leaves_of_pattern(case, machine) {
                        if !leaves.contains(&leaf) {
                            leaves.push(leaf);
                        }
                    }
                }
                leaves
            }
            syn::Pat::Ident(ident) => ident
                .subpat
                .as_ref()
                .map(|(_, subpat)| self.state_leaves_of_pattern(subpat, machine))
                .unwrap_or_default(),
            syn::Pat::Paren(paren) => self.state_leaves_of_pattern(&paren.pat, machine),
            syn::Pat::Reference(reference) => {
                self.state_leaves_of_pattern(&reference.pat, machine)
            }
            _ => Vec::new(),
        }
    }

    /// The state leaf a value expression constructs:
    /// `State::Idle` → `Idle`; `State::Active(ActiveState::Ready)` →
    /// `Active/Ready`; a composite with a dynamic child → `None`.
    fn state_leaf_of_expr(&self, expr: &syn::Expr, machine: &StateMachine) -> Option<String> {
        let (enum_name, variant) = enum_variant_of_expr(expr)?;
        if enum_name != machine.enum_name && enum_name != "Self" {
            return None;
        }
        match machine.child_enum(&variant) {
            None => machine.variants.contains(&variant).then_some(variant),
            Some(child_enum) => {
                let syn::Expr::Call(call) = expr else { return None };
                let [argument] = call.args.iter().collect::<Vec<_>>()[..] else {
                    return None;
                };
                let (child_name, child_variant) = enum_variant_of_expr(argument)?;
                (child_name == child_enum)
                    .then(|| format!("{variant}/{child_variant}"))
                    .filter(|leaf| machine.variants.contains(leaf))
            }
        }
    }

    /// Resolves a predicate method on the state enum by analyzing its body
    /// (e.g. `fn has_capture(&self) -> bool { !matches!(self, Self::Idle) }`).
    fn eval_predicate(&self, method: &str, machine: &StateMachine, depth: usize) -> GuardEval {
        if depth >= MAX_PREDICATE_DEPTH {
            return GuardEval::Unresolved;
        }
        let Some(function) = self.index.find_fn(Some(&machine.enum_name), method) else {
            return GuardEval::Unresolved;
        };
        // The predicate's value is its trailing expression.
        let Some(syn::Stmt::Expr(trailing, None)) = function.block.stmts.last() else {
            return GuardEval::Unresolved;
        };
        match self.eval_condition(trailing, machine, &["self", machine.field_name.as_str()], depth + 1)
        {
            GuardEval::NoConstraint => GuardEval::Unresolved,
            resolved => resolved,
        }
    }

    // ---- transition emission ------------------------------------------------

    fn handle_assignment(&mut self, assign: &'a syn::ExprAssign, ctx: &Ctx<'a>, file: &Path) {
        // `*.state = Enum::Variant` — a direct transition target
        // (composite children included: `State::Active(ActiveState::Ready)`).
        if let Some(field) = last_field_name(&assign.left) {
            if let Some(machine) = self.machine_for_field(&field) {
                let machine = machine.clone();
                if let Some(to) = self.state_leaf_of_expr(&assign.right, &machine) {
                    self.emit(&machine, to, ctx, assign, file);
                    return;
                }
                // Not a literal construction: try value-flow before warning.
                if self.default_reset_targets(&assign.right).is_none() {
                    self.handle_dynamic_assignment(&machine, assign, ctx, file);
                    return;
                }
            }
        }

        // `*.anything = T::default()` — a struct reset that implies every
        // state field inside T lands on its enum's #[default] variant.
        if let Some(reset_targets) = self.default_reset_targets(&assign.right) {
            for (machine, to) in reset_targets {
                self.emit(&machine, to, ctx, assign, file);
            }
        }
    }

    /// Value-flow for `*.state = <runtime value>`:
    /// - a binding from the event payload typed as the state enum means the
    ///   target is externally supplied → wildcard target `"*"`;
    /// - a value constrained by the conditions in force (`==`, `matches!`,
    ///   predicate calls on that exact expression) fans out to the states
    ///   the constraints allow;
    /// - otherwise the transition is dropped with a warning.
    fn handle_dynamic_assignment(
        &mut self,
        machine: &StateMachine,
        assign: &'a syn::ExprAssign,
        ctx: &Ctx<'a>,
        file: &Path,
    ) {
        if let Some(path) = expr_path_string(&assign.right) {
            // Event payload binding of the machine's enum type.
            if !path.contains('.')
                && ctx.payload_bindings.get(&path) == Some(&machine.enum_name)
            {
                self.emit(machine, ANY_STATE.to_string(), ctx, assign, file);
                return;
            }

            // Conditions constraining this exact value expression.
            let mut eval = GuardEval::NoConstraint;
            for condition in &ctx.conditions {
                eval = and(eval, self.eval_value_condition(condition, &path, machine, 0));
            }
            if let GuardEval::Known(targets) = eval {
                for to in targets {
                    self.emit(machine, to, ctx, assign, file);
                }
                return;
            }
        }

        self.warnings.push(Warning {
            file: file.to_path_buf(),
            line: assign.span().start().line,
            message: format!(
                "transition of `{}` dropped: target state is dynamic \
                 (assigned from a runtime value)",
                machine.enum_name
            ),
        });
    }

    /// What `condition` says about the *value* at `path` (dotted expression
    /// path). The mirror of [`Self::eval_condition`], keyed by exact path so
    /// constraints on `known.status` never leak onto `draft.status`.
    fn eval_value_condition(
        &self,
        condition: &syn::Expr,
        path: &str,
        machine: &StateMachine,
        depth: usize,
    ) -> GuardEval {
        if depth >= MAX_PREDICATE_DEPTH {
            return GuardEval::Unresolved;
        }
        match condition {
            syn::Expr::Macro(expr_macro) => {
                let Some(args) = as_matches_macro(&expr_macro.mac) else {
                    return GuardEval::NoConstraint;
                };
                if expr_path_string(&args.expr).as_deref() != Some(path) {
                    return GuardEval::NoConstraint;
                }
                let states = self.state_leaves_of_pattern(&args.pat, machine);
                if states.is_empty() {
                    GuardEval::NoConstraint
                } else {
                    GuardEval::Known(states)
                }
            }
            syn::Expr::Binary(binary) => match binary.op {
                syn::BinOp::And(_) => and(
                    self.eval_value_condition(&binary.left, path, machine, depth),
                    self.eval_value_condition(&binary.right, path, machine, depth),
                ),
                syn::BinOp::Or(_) => or(
                    self.eval_value_condition(&binary.left, path, machine, depth),
                    self.eval_value_condition(&binary.right, path, machine, depth),
                ),
                syn::BinOp::Eq(_) | syn::BinOp::Ne(_) => {
                    let value = if expr_path_string(&binary.left).as_deref() == Some(path) {
                        self.state_leaf_of_expr(&binary.right, machine)
                    } else if expr_path_string(&binary.right).as_deref() == Some(path) {
                        self.state_leaf_of_expr(&binary.left, machine)
                    } else {
                        None
                    };
                    let Some(variant) = value else {
                        return GuardEval::NoConstraint;
                    };
                    if matches!(binary.op, syn::BinOp::Eq(_)) {
                        GuardEval::Known(vec![variant])
                    } else {
                        GuardEval::Known(
                            machine
                                .variants
                                .iter()
                                .filter(|v| **v != variant)
                                .cloned()
                                .collect(),
                        )
                    }
                }
                _ => GuardEval::NoConstraint,
            },
            syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Not(_)) => {
                match self.eval_value_condition(&unary.expr, path, machine, depth) {
                    GuardEval::Known(states) => GuardEval::Known(
                        machine
                            .variants
                            .iter()
                            .filter(|v| !states.contains(v))
                            .cloned()
                            .collect(),
                    ),
                    other => other,
                }
            }
            // `is_this_runs_answer(&value)` — resolve the predicate's body
            // against its parameter.
            syn::Expr::Call(call) => {
                let syn::Expr::Path(func) = &*call.func else {
                    return GuardEval::NoConstraint;
                };
                let position = call
                    .args
                    .iter()
                    .position(|arg| expr_path_string(arg).as_deref() == Some(path));
                let Some(position) = position else {
                    return GuardEval::NoConstraint;
                };
                let segments: Vec<String> = func
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                let (callee_self, callee_name) = match segments.as_slice() {
                    [name] => (None, name.as_str()),
                    [ty, name] => (Some(ty.as_str()), name.as_str()),
                    _ => return GuardEval::NoConstraint,
                };
                let Some(function) = self.index.find_fn(callee_self, callee_name) else {
                    return GuardEval::Unresolved;
                };
                let Some(param) = function.params.get(position) else {
                    return GuardEval::Unresolved;
                };
                let Some(syn::Stmt::Expr(trailing, None)) = function.block.stmts.last() else {
                    return GuardEval::Unresolved;
                };
                match self.eval_value_condition(trailing, param, machine, depth + 1) {
                    GuardEval::NoConstraint => GuardEval::Unresolved,
                    resolved => resolved,
                }
            }
            // `value.is_final()` — a predicate method on the state enum.
            syn::Expr::MethodCall(call) => {
                if expr_path_string(&call.receiver).as_deref() != Some(path) {
                    return GuardEval::NoConstraint;
                }
                self.eval_predicate(&call.method.to_string(), machine, depth)
            }
            syn::Expr::Paren(paren) => {
                self.eval_value_condition(&paren.expr, path, machine, depth)
            }
            syn::Expr::Group(group) => {
                self.eval_value_condition(&group.expr, path, machine, depth)
            }
            syn::Expr::Block(block) => match block.block.stmts.last() {
                Some(syn::Stmt::Expr(trailing, None)) => {
                    self.eval_value_condition(trailing, path, machine, depth)
                }
                _ => GuardEval::NoConstraint,
            },
            _ => GuardEval::NoConstraint,
        }
    }

    /// Machines reset by an `= T::default()` assignment, with the state each
    /// one lands on.
    fn default_reset_targets(&self, rhs: &syn::Expr) -> Option<Vec<(StateMachine, String)>> {
        let syn::Expr::Call(call) = rhs else { return None };
        let syn::Expr::Path(path) = &*call.func else { return None };
        let segments: Vec<String> = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        let [type_name, method] = segments.last_chunk::<2>()?;
        if method != "default" {
            return None;
        }

        let strct = self.index.structs.get(type_name)?;
        let targets: Vec<(StateMachine, String)> = self
            .machines
            .iter()
            .filter(|machine| {
                strct
                    .fields
                    .iter()
                    .any(|(name, ty)| *name == machine.field_name && *ty == machine.enum_name)
            })
            .filter_map(|machine| {
                let default = self
                    .index
                    .enum_decls(&machine.enum_name)
                    .iter()
                    .find_map(|decl| decl.default_variant.clone())?;
                Some((machine.clone(), default))
            })
            .collect();

        (!targets.is_empty()).then_some(targets)
    }

    fn emit(
        &mut self,
        machine: &StateMachine,
        to: String,
        ctx: &Ctx<'a>,
        assign: &syn::ExprAssign,
        file: &Path,
    ) {
        let line = assign.span().start().line;

        let Some(events) = &ctx.events else {
            self.warnings.push(Warning {
                file: file.to_path_buf(),
                line,
                message: format!(
                    "transition to `{to}` dropped: could not infer the triggering event"
                ),
            });
            return;
        };

        match self.source_states(ctx, machine) {
            GuardEval::NoConstraint => {
                // No state evidence: the transition fires from any state.
                for event in events {
                    self.push(machine, ANY_STATE.to_string(), event.clone(), to.clone(), ctx.arm);
                }
            }
            GuardEval::Known(from_states) => {
                for event in events {
                    for from in &from_states {
                        self.push(machine, from.clone(), event.clone(), to.clone(), ctx.arm);
                    }
                }
            }
            GuardEval::Unresolved => {
                self.warnings.push(Warning {
                    file: file.to_path_buf(),
                    line,
                    message: format!(
                        "transition to `{to}` dropped: source-state condition could not \
                         be resolved statically"
                    ),
                });
            }
        }
    }

    fn push(&mut self, machine: &StateMachine, from: String, event: String, to: String, arm: usize) {
        self.out.push((
            RawTransition {
                machine: machine.enum_name.clone(),
                field: machine.field_name.clone(),
                from,
                event,
                to,
                effects: Vec::new(),
            },
            arm,
        ));
    }
}

/// All closure bodies found inside an expression (e.g. the predicate of a
/// `list.iter_mut().find(|d| ...)` chain), without entering nested closures'
/// own subtrees twice.
fn closure_bodies(expr: &syn::Expr) -> Vec<&syn::Expr> {
    struct Closures<'ast> {
        found: Vec<&'ast syn::Expr>,
    }
    impl<'ast> syn::visit::Visit<'ast> for Closures<'ast> {
        fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
            self.found.push(&closure.body);
            syn::visit::visit_expr_closure(self, closure);
        }
    }
    let mut visitor = Closures { found: Vec::new() };
    syn::visit::Visit::visit_expr(&mut visitor, expr);
    visitor.found
}

/// Conjunction of two evaluations: constraints intersect; concrete knowledge
/// wins over an unresolved conjunct (the emitted set may then be a superset
/// of the truth, which is the right bias for documentation).
fn and(left: GuardEval, right: GuardEval) -> GuardEval {
    match (left, right) {
        (GuardEval::Known(a), GuardEval::Known(b)) => {
            GuardEval::Known(a.into_iter().filter(|v| b.contains(v)).collect())
        }
        (GuardEval::Known(a), _) | (_, GuardEval::Known(a)) => GuardEval::Known(a),
        (GuardEval::Unresolved, _) | (_, GuardEval::Unresolved) => GuardEval::Unresolved,
        (GuardEval::NoConstraint, GuardEval::NoConstraint) => GuardEval::NoConstraint,
    }
}

/// Disjunction of two evaluations: only two concrete sides stay concrete.
fn or(left: GuardEval, right: GuardEval) -> GuardEval {
    match (left, right) {
        (GuardEval::Known(a), GuardEval::Known(b)) => {
            let mut union = a;
            for state in b {
                if !union.contains(&state) {
                    union.push(state);
                }
            }
            GuardEval::Known(union)
        }
        (GuardEval::NoConstraint, GuardEval::NoConstraint) => GuardEval::NoConstraint,
        _ => GuardEval::Unresolved,
    }
}

/// Result of reading an event-arm pattern.
enum EventLabels {
    /// Concrete leaf labels (`PauseTapped`, possibly several via `|`).
    Labels(Vec<String>),
    /// Wrapper variant or catch-all binding: keep the surrounding context.
    Delegating,
    /// Pattern matches something that is not an event (e.g. a literal).
    None,
}
