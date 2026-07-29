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

use std::path::{Path, PathBuf};

use syn::spanned::Spanned;

use crate::ast_util::{
    as_matches_macro, enum_variant_of_expr, is_catch_all, last_field_name, pattern_variants,
};
use crate::core_finder::CoreInfo;
use crate::index::CrateIndex;
use crate::state_enum::StateMachine;
use crate::Warning;

/// The wildcard source: the transition fires from any state.
pub(crate) const ANY_STATE: &str = "*";

/// Maximum predicate-method resolution depth (predicates calling predicates).
const MAX_PREDICATE_DEPTH: usize = 3;

/// A transition attributed to a specific state machine (enum).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawTransition {
    pub machine: String,
    pub from: String,
    pub event: String,
    pub to: String,
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
    /// Facts from `match`-on-state arms: (machine enum, possible states).
    facts: Vec<(String, Vec<String>)>,
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
        call_stack: vec![(Some(core.name.clone()), "update".to_string())],
    };
    walker.walk_block(
        update.block,
        &Ctx {
            events: None,
            conditions: Vec::new(),
            facts: Vec::new(),
        },
        update.self_ty.clone(),
        update.file,
    );
    walker.out
}

struct Walker<'w, 'a> {
    index: &'w CrateIndex<'a>,
    core: &'w CoreInfo,
    machines: &'w [StateMachine],
    warnings: &'w mut Vec<Warning>,
    out: Vec<RawTransition>,
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
        decl.variant_field_types[position]
            .iter()
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
        for stmt in &block.stmts {
            match stmt {
                syn::Stmt::Expr(expr, _) => self.walk_expr(expr, ctx, &self_ty, file),
                syn::Stmt::Local(local) => {
                    if let Some(init) = &local.init {
                        self.walk_expr(&init.expr, ctx, &self_ty, file);
                        if let Some((_, else_branch)) = &init.diverge {
                            self.walk_expr(else_branch, ctx, &self_ty, file);
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
            syn::Expr::Struct(strct) => {
                for field in &strct.fields {
                    self.walk_expr(&field.expr, ctx, self_ty, file);
                }
            }
            syn::Expr::Tuple(tuple) => {
                for element in &tuple.elems {
                    self.walk_expr(element, ctx, self_ty, file);
                }
            }
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
            return; // external function (e.g. crux_core::render)
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
            let mut variants = Vec::new();
            pattern_variants(&arm.pat, &mut variants);
            let arm_states: Vec<String> = variants
                .into_iter()
                .filter(|(e, _)| *e == machine.enum_name || e == "Self")
                .map(|(_, v)| v)
                .collect();

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
                arm_ctx.facts.push((machine.enum_name.clone(), states));
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
            if let Some((_, guard)) = &arm.guard {
                arm_ctx.conditions.push(guard);
                self.walk_expr(guard, ctx, self_ty, file);
            }
            self.walk_expr(&arm.body, &arm_ctx, self_ty, file);
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

        for (fact_machine, states) in &ctx.facts {
            if fact_machine == &machine.enum_name {
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
                let mut variants = Vec::new();
                pattern_variants(&args.pat, &mut variants);
                let states: Vec<String> = variants
                    .into_iter()
                    .filter(|(e, _)| *e == machine.enum_name || e == "Self")
                    .map(|(_, v)| v)
                    .collect();
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
                _ => GuardEval::NoConstraint,
            },
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
            _ => GuardEval::NoConstraint,
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
        // `*.state = Enum::Variant` — a direct transition target.
        if let Some(field) = last_field_name(&assign.left) {
            if let Some(machine) = self.machine_for_field(&field) {
                if let Some((enum_name, to)) = enum_variant_of_expr(&assign.right) {
                    if enum_name == machine.enum_name && machine.variants.contains(&to) {
                        let machine = machine.clone();
                        self.emit(&machine, to, ctx, assign, file);
                        return;
                    }
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
                    self.push(machine, ANY_STATE.to_string(), event.clone(), to.clone());
                }
            }
            GuardEval::Known(from_states) => {
                for event in events {
                    for from in &from_states {
                        self.push(machine, from.clone(), event.clone(), to.clone());
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

    fn push(&mut self, machine: &StateMachine, from: String, event: String, to: String) {
        self.out.push(RawTransition {
            machine: machine.enum_name.clone(),
            from,
            event,
            to,
        });
    }
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
