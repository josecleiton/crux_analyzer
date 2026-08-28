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

use std::cell::Cell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use syn::spanned::Spanned;

use crate::ast_util::{
    arm_pattern_and_guard, as_matches_macro, enum_variant_of_expr, enum_variant_path,
    expr_path_string, is_catch_all, last_field_name, pattern_variants, receiver_path,
    receivers_may_alias,
};
use crate::core_finder::CoreInfo;
use crate::index::CrateIndex;
use crate::state_enum::StateMachine;
use crate::{Limits, Warning, WarningKind};

/// The wildcard source: the transition fires from any state.
pub(crate) const ANY_STATE: &str = "*";

/// Maximum predicate-method resolution depth (predicates calling predicates).
const MAX_PREDICATE_DEPTH: usize = 3;

/// How deep a callback expression is followed looking for the event it builds.
/// Far above `|out| Event::Recorder(RecorderEvent::Loaded(out))`; past it the
/// callback simply reads as unresolved. See `docs/security.md`.
const MAX_EVENT_VALUE_DEPTH: usize = 32;

/// The walk's remaining allowance, and which limit stopped it.
///
/// Interior mutability because the evaluators that recurse (`eval_condition`,
/// `eval_value_condition`, `state_leaves_of_pattern`) take `&self` — charging
/// them through a `Cell` keeps one budget for the whole walk without turning
/// every read-only evaluator into a `&mut self` method.
///
/// Every recursive entry point is wrapped `enter() … leave()`. `enter` returning
/// `false` means a limit fired: stop descending, and let the recorded limit
/// name become an `analysis-truncated` warning at the end of `extract`.
struct Budget {
    steps: Cell<u64>,
    depth: Cell<usize>,
    max_depth: usize,
    max_call_depth: usize,
    /// Name of the first limit that fired, for the warning.
    hit: Cell<Option<&'static str>>,
}

impl Budget {
    fn new(limits: &Limits) -> Self {
        Self {
            steps: Cell::new(limits.max_steps),
            depth: Cell::new(0),
            max_depth: limits.max_depth,
            max_call_depth: limits.max_call_depth,
            hit: Cell::new(None),
        }
    }

    /// Charges one step and one level of nesting.
    fn enter(&self) -> bool {
        let steps = self.steps.get();
        if steps == 0 {
            self.record("max-steps");
            return false;
        }
        let depth = self.depth.get();
        if depth >= self.max_depth {
            self.record("max-depth");
            return false;
        }
        self.steps.set(steps - 1);
        self.depth.set(depth + 1);
        true
    }

    fn leave(&self) {
        self.depth.set(self.depth.get() - 1);
    }

    fn record(&self, limit: &'static str) {
        if self.hit.get().is_none() {
            self.hit.set(Some(limit));
        }
    }
}

/// A transition attributed to a specific state machine (enum + field —
/// the same enum can drive more than one machine through different fields).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawTransition {
    pub machine: String,
    pub field: String,
    pub from: String,
    pub event: String,
    pub to: String,
    /// Effects requested on a code path this transition is on.
    pub effects: Vec<RawEffect>,
}

/// An effect request as the source shows it: the operation, the capability it
/// travels through, and the event the shell answers with.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RawEffect {
    pub label: String,
    pub capability: Option<String>,
    pub resolves_with: Vec<String>,
    /// Set when the request sits on a branch the transition does not imply.
    pub conditional: bool,
}

/// Where in the branch tree of an event arm something was found: the chain of
/// alternatives (`if`/`else` branches, `match` arms) entered to reach it.
///
/// Two paths on the same chain — one a prefix of the other — describe code that
/// runs together; paths that fork apart describe alternatives that never do.
/// That is what keeps an effect requested in one branch off the transitions of
/// its sibling.
type BranchPath = Vec<usize>;

/// Whether `path` runs on the same chain of alternatives as `other`.
fn same_chain(path: &[usize], other: &[usize]) -> bool {
    path.starts_with(other) || other.starts_with(path)
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

/// What a `match`-on-state arm establishes: for this machine, on *this* object,
/// the state is one of these.
#[derive(Clone)]
struct Fact {
    machine: String,
    field: String,
    /// Receiver of the matched expression — `model.session` for
    /// `match model.session.state`. `None` for a bare binding.
    receiver: Option<String>,
    states: Vec<String>,
}

/// What a source-state evaluation is *about*.
///
/// Source evidence used to be keyed by field name alone, which made a guard on
/// one record's `status` speak for every other record's `status`. Carrying the
/// subject is what lets a guard be recognised as being about a different object
/// and so as no evidence at all.
#[derive(Clone, Copy)]
struct SourceScope<'s> {
    /// Receiver of the assignment being explained (`draft` for
    /// `draft.status = …`). `None` = unknown, which stays permissive.
    subject: Option<&'s str>,
    /// Extra field spellings accepted as the state field — `"self"` inside
    /// predicate methods on the state enum.
    self_fields: &'s [&'s str],
}

#[derive(Clone)]
struct Ctx<'a> {
    /// Event labels currently in scope. `None` = not statically known.
    events: Option<Vec<String>>,
    /// Guard / `if` conditions currently in force.
    conditions: Vec<&'a syn::Expr>,
    /// Facts from `match`-on-state arms.
    facts: Vec<Fact>,
    /// Bindings introduced by the current event arm's pattern:
    /// binding name → payload type (`Event::Updated { status }` → status:
    /// JobStatus). Valid within the arm body only.
    payload_bindings: HashMap<String, String>,
    /// The event arm this code belongs to — transitions and effects found
    /// under the same arm are associated with each other.
    arm: usize,
    /// Which alternatives were entered to reach this code, innermost last.
    branch: BranchPath,
    /// The events a request made here can be answered with, when the request
    /// site declares them (`…then_send(Event::X)`, an event passed alongside the
    /// operation, or every event the result-mapping callback builds). Empty =
    /// nothing said so.
    resolution: Vec<String>,
}

pub(crate) fn extract(
    index: &CrateIndex,
    core: &CoreInfo,
    machines: &[StateMachine],
    limits: &Limits,
    warnings: &mut Vec<Warning>,
) -> Vec<RawTransition> {
    let Some(update) = index.find_fn(Some(&core.name), "update") else {
        warnings.push(Warning {
            file: PathBuf::new(),
            line: 0,
            kind: WarningKind::NoUpdateMethod {
                core: core.name.clone(),
            },
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
        branch_counter: 0,
        call_stack: vec![(Some(core.name.clone()), "update".to_string())],
        budget: Budget::new(limits),
    };
    walker.walk_block(
        update.block,
        &Ctx {
            events: None,
            conditions: Vec::new(),
            facts: Vec::new(),
            payload_bindings: HashMap::new(),
            arm: 0,
            branch: BranchPath::new(),
            resolution: Vec::new(),
        },
        update.self_ty.clone(),
        update.file,
    );

    // Attach the effects observed under each event arm to its transitions.
    let truncated = walker.budget.hit.get();
    let effects_by_arm = walker.effects_by_arm;
    let mut out = walker.out;
    for (transition, arm, branch) in &mut out {
        if let Some(effects) = effects_by_arm.get(arm) {
            transition.effects = attach(effects, branch);
        }
    }

    // A limit that fired means the model may be missing transitions. Saying so
    // is the honesty rule applied to resources — and it makes `--deny-warnings`
    // fail the run rather than publish a quietly partial diagram.
    if let Some(limit) = truncated {
        warnings.push(Warning {
            file: update.file.to_path_buf(),
            line: 0,
            kind: WarningKind::AnalysisTruncated {
                core: core.name.clone(),
                limit: limit.to_string(),
            },
        });
    }

    out.into_iter()
        .map(|(transition, _, _)| transition)
        .collect()
}

/// The effects of one event arm that belong to a transition found at `branch`.
///
/// An effect on the transition's own chain of alternatives fires with it; one
/// found *deeper* — on a branch the transition itself does not imply — is kept
/// but marked conditional, because dropping it would hide a real request and
/// stating it plainly would claim more than the source does. Effects on a
/// forked-off branch belong to that branch's transitions, not to this one.
fn attach(effects: &[(RawEffect, BranchPath)], branch: &BranchPath) -> Vec<RawEffect> {
    let mut out: Vec<RawEffect> = Vec::new();
    for (effect, effect_branch) in effects {
        if !same_chain(effect_branch, branch) {
            continue;
        }
        let conditional = effect_branch.len() > branch.len();
        // The same request can be reached both on the transition's own path and
        // on a branch below it; certainty wins over the conditional sighting.
        if let Some(kept) = out.iter_mut().find(|kept| {
            kept.label == effect.label
                && kept.capability == effect.capability
                && kept.resolves_with == effect.resolves_with
        }) {
            kept.conditional &= conditional;
            continue;
        }
        out.push(RawEffect {
            conditional,
            ..effect.clone()
        });
    }
    out
}

struct Walker<'w, 'a> {
    index: &'w CrateIndex<'a>,
    core: &'w CoreInfo,
    machines: &'w [StateMachine],
    warnings: &'w mut Vec<Warning>,
    out: Vec<(RawTransition, usize, BranchPath)>,
    /// Effect requests observed under each event arm, with the branch each was
    /// found on — [`attach`] decides which transitions they belong to.
    effects_by_arm: HashMap<usize, Vec<(RawEffect, BranchPath)>>,
    arm_counter: usize,
    /// Hands out branch ids; each alternative entered gets a fresh one.
    branch_counter: usize,
    /// Functions currently being walked — breaks recursion cycles while still
    /// allowing the same helper to be re-walked under a different context.
    call_stack: Vec<(Option<String>, String)>,
    /// What this walk is still allowed to explore. The call stack breaks
    /// *cycles*; only this bounds the exponential fan-out of a diamond call
    /// graph, and the nesting depth of hostile input.
    budget: Budget,
}

impl<'w, 'a> Walker<'w, 'a> {
    fn machine_for_field(&self, field: &str) -> Option<&'w StateMachine> {
        self.machines.iter().find(|m| m.field_name == field)
    }

    /// The machine a field name refers to, disambiguated by an enum named at
    /// the site.
    ///
    /// Two machines may share a field name — `model.recording.state` and
    /// `model.session.state` — and then the field alone does not say which one
    /// is being driven. Whatever enum the site mentions does: the variant being
    /// assigned, or the variants the arms match on. Without this, every
    /// transition of the machine that sorts second was judged against the one
    /// that sorts first, dropped as dynamic, and the machine vanished from the
    /// model for having no transitions left.
    fn machine_for_field_and_enum(
        &self,
        field: &str,
        enum_name: Option<&str>,
    ) -> Option<&'w StateMachine> {
        if let Some(enum_name) = enum_name {
            let named = self
                .machines
                .iter()
                .find(|m| m.field_name == field && m.enum_name == enum_name);
            if named.is_some() {
                return named;
            }
        }
        self.machine_for_field(field)
    }

    /// The state enum an arm list matches on, when the arms name exactly one
    /// enum. Two different enums in one match is not evidence about either.
    fn enum_matched_by(&self, arms: &[syn::Arm]) -> Option<String> {
        let mut found: Option<String> = None;
        for arm in arms {
            let (pat, _) = arm_pattern_and_guard(arm);
            let mut variants = Vec::new();
            pattern_variants(pat, &mut variants);
            for (enum_name, _) in variants {
                match &found {
                    Some(seen) if seen != &enum_name => return None,
                    Some(_) => {}
                    None => found = Some(enum_name),
                }
            }
        }
        found
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

    /// Every expression descent is charged here — this is the single choke
    /// point that bounds both the walk's total work and its nesting depth.
    fn walk_expr(
        &mut self,
        expr: &'a syn::Expr,
        ctx: &Ctx<'a>,
        self_ty: &Option<String>,
        file: &Path,
    ) {
        if !self.budget.enter() {
            return;
        }
        self.walk_expr_inner(expr, ctx, self_ty, file);
        self.budget.leave();
    }

    fn walk_expr_inner(
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
                // Then and else are alternatives: a request in one is not a
                // request in the other.
                let mut then_ctx = self.enter_branch(ctx);
                then_ctx.conditions.push(&expr_if.cond);
                self.walk_expr(&expr_if.cond, ctx, self_ty, file);
                self.walk_block(&expr_if.then_branch, &then_ctx, self_ty.clone(), file);
                if let Some((_, else_expr)) = &expr_if.else_branch {
                    let else_ctx = self.enter_branch(ctx);
                    self.walk_expr(else_expr, &else_ctx, self_ty, file);
                }
            }
            syn::Expr::Call(call) => {
                // `AudioOperation::Start(..)` — a tuple-variant effect.
                if let syn::Expr::Path(path) = &*call.func {
                    self.record_effect_path(&path.path, ctx);
                }
                // An event handed to the same call is the callback the shell
                // answers with: `request(AudioOperation::Start, Event::Started)`.
                // A request built by a helper declares it one call away instead
                // (`audio_command(op)` whose body does the `then_send`), so the
                // callee is consulted when the call site itself says nothing.
                let resolved = self
                    .with_resolution(ctx, &call.args)
                    .or_else(|| self.with_callee_resolution(ctx, &call.func, self_ty));
                let call_ctx = resolved.as_ref().unwrap_or(ctx);
                for arg in &call.args {
                    self.walk_expr(arg, call_ctx, self_ty, file);
                }
                self.follow_call(&call.func, ctx, self_ty);
            }
            syn::Expr::MethodCall(method_call) => {
                // `…request_from_shell(op).then_send(Event::Started)`: the
                // callback is declared on the chain that built the request, so
                // it reaches the operation through the receiver.
                let resolved = self.with_resolution(ctx, &method_call.args);
                if resolved.is_none() {
                    self.check_resolution_sink(method_call, file);
                }
                let call_ctx = resolved.as_ref().unwrap_or(ctx);
                self.walk_expr(&method_call.receiver, call_ctx, self_ty, file);
                for arg in &method_call.args {
                    self.walk_expr(arg, call_ctx, self_ty, file);
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

    /// `Self::helper`, `Type::helper` or `helper` → the callee this call names.
    fn callee_key(
        &self,
        func: &syn::Expr,
        self_ty: &Option<String>,
    ) -> Option<(Option<String>, String)> {
        let syn::Expr::Path(path) = func else {
            return None;
        };
        let segments: Vec<String> = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();

        match segments.as_slice() {
            [name] => Some((None, name.clone())),
            [ty, name] if ty == "Self" => Some((self_ty.clone(), name.clone())),
            [ty, name] => Some((Some(ty.clone()), name.clone())),
            _ => None,
        }
    }

    /// Follows `Self::helper(...)`, `Type::helper(...)` or `helper(...)` into
    /// the callee's body, keeping the current context.
    fn follow_call(&mut self, func: &'a syn::Expr, ctx: &Ctx<'a>, self_ty: &Option<String>) {
        let Some((callee_self, callee_name)) = self.callee_key(func, self_ty) else {
            return;
        };

        let key = (callee_self.clone(), callee_name.clone());
        if self.call_stack.contains(&key) {
            return; // recursion cycle
        }
        // A cycle-free call chain can still be arbitrarily long, and each level
        // multiplies the work below it.
        if self.call_stack.len() >= self.budget.max_call_depth {
            self.budget.record("max-call-depth");
            return;
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
            let matched = self.enum_matched_by(&expr_match.arms);
            if let Some(machine) = self.machine_for_field_and_enum(&field, matched.as_deref()) {
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

        // Anything else: walk generically. The arms are still alternatives.
        self.walk_expr(&expr_match.expr, ctx, self_ty, file);
        for arm in &expr_match.arms {
            let (_, guard) = arm_pattern_and_guard(arm);
            if let Some(guard) = guard {
                self.walk_expr(guard, ctx, self_ty, file);
            }
            let arm_ctx = self.enter_branch(ctx);
            self.walk_expr(&arm.body, &arm_ctx, self_ty, file);
        }
    }

    fn arms_reference_enum(&self, arms: &[syn::Arm], enum_name: &str) -> bool {
        arms.iter().any(|arm| {
            let mut variants = Vec::new();
            pattern_variants(arm_pattern_and_guard(arm).0, &mut variants);
            variants.iter().any(|(e, _)| e == enum_name)
        })
    }

    fn arms_reference_events(&self, arms: &[syn::Arm]) -> bool {
        arms.iter().any(|arm| {
            let mut variants = Vec::new();
            pattern_variants(arm_pattern_and_guard(arm).0, &mut variants);
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
            let (arm_pat, arm_guard) = arm_pattern_and_guard(arm);
            let arm_states = self.state_leaves_of_pattern(arm_pat, &machine);

            let states = if !arm_states.is_empty() {
                arm_states
            } else if is_catch_all(arm_pat) {
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

            let mut arm_ctx = self.enter_branch(ctx);
            if !states.is_empty() {
                arm_ctx.facts.push(Fact {
                    machine: machine.enum_name.clone(),
                    field: machine.field_name.clone(),
                    // Which object was matched: `match other.state` narrows
                    // `other`, not whatever this arm goes on to assign.
                    receiver: receiver_path(&expr_match.expr),
                    states,
                });
            }
            if let Some(guard) = arm_guard {
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
            let (arm_pat, arm_guard) = arm_pattern_and_guard(arm);
            let labels = self.event_labels(arm_pat);
            let events = match labels {
                EventLabels::Labels(labels) => Some(labels),
                // Wrapper variants (`Event::Recording(e)`) and catch-all
                // bindings delegate: the inner match resolves the label.
                EventLabels::Delegating => ctx.events.clone(),
                EventLabels::None => None,
            };

            let mut arm_ctx = self.enter_branch(ctx);
            arm_ctx.events = events;
            // Each event arm gets its own effect scope and payload bindings.
            self.arm_counter += 1;
            arm_ctx.arm = self.arm_counter;
            arm_ctx
                .payload_bindings
                .extend(self.payload_bindings(arm_pat));
            if let Some(guard) = arm_guard {
                arm_ctx.conditions.push(guard);
                self.walk_expr(guard, ctx, self_ty, file);
            }
            self.walk_expr(&arm.body, &arm_ctx, self_ty, file);
        }
    }

    /// Bindings introduced by an event-arm pattern, with their payload types:
    /// `Event::Updated { id, status }` → `{id: String, status: JobStatus}`;
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

    /// Records `Enum::Variant` as an effect when the path names a request: a
    /// variant, declared, on an enum the `Effect` root wraps directly.
    ///
    /// Two things this deliberately does not record, both of which spell
    /// themselves the same way a request does — a variant of a payload enum
    /// reached deeper in the closure, and an associated function on an operation
    /// enum. See [`CoreInfo::is_effect_request_enum`] and
    /// [`CoreInfo::declares_variant`].
    fn record_effect_path(&mut self, path: &syn::Path, ctx: &Ctx<'a>) {
        let Some((enum_name, variant)) = enum_variant_path(path) else {
            return;
        };
        if self.core.is_effect_request_enum(&enum_name)
            && self.core.declares_variant(&enum_name, &variant)
        {
            let capability = self.core.capability_of(&enum_name);
            self.record(ctx, format!("{enum_name}::{variant}"), capability);
        }
    }

    /// Crux's bare `render()` — an effect with no operation enum, so no
    /// capability and nothing to send back.
    fn record_effect(&mut self, ctx: &Ctx<'a>, label: String) {
        self.record(ctx, label, None);
    }

    fn record(&mut self, ctx: &Ctx<'a>, label: String, capability: Option<String>) {
        let effect = RawEffect {
            label,
            capability,
            resolves_with: ctx.resolution.clone(),
            // Decided per transition in `attach`: the same request is certain
            // for the transitions on its own branch and conditional for those
            // above it.
            conditional: false,
        };
        let effects = self.effects_by_arm.entry(ctx.arm).or_default();
        // Keyed by branch as well as by request: the same operation in two
        // sibling branches is two sightings, and collapsing them would hand
        // one branch's transitions the other's effect.
        if !effects
            .iter()
            .any(|(kept, branch)| kept == &effect && branch == &ctx.branch)
        {
            effects.push((effect, ctx.branch.clone()));
        }
    }

    /// A fresh alternative: everything walked under the returned context is on
    /// a branch of its own.
    fn enter_branch(&mut self, ctx: &Ctx<'a>) -> Ctx<'a> {
        self.branch_counter += 1;
        let mut branch_ctx = ctx.clone();
        branch_ctx.branch.push(self.branch_counter);
        branch_ctx
    }

    /// The context to walk a call's arguments and receiver in, when the call
    /// declares which events answer the request made inside it.
    ///
    /// `None` when it declares none — the caller then keeps its own context,
    /// which also keeps an enclosing callback in force for a nested call.
    fn with_resolution(
        &self,
        ctx: &Ctx<'a>,
        args: &syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>,
    ) -> Option<Ctx<'a>> {
        let events = self.declared_events(args);
        if events.is_empty() {
            return None;
        }
        let mut resolved = ctx.clone();
        resolved.resolution = events;
        Some(resolved)
    }

    /// The same, for a request whose callback is declared inside the helper it
    /// delegates to:
    ///
    /// ```text
    /// Self::audio_command(AudioOperation::Start)   // <- the operation
    /// // fn audio_command(op) { Command::request_from_shell(op).then_send(…) }
    /// ```
    ///
    /// Only the callee's own body is read — no further calls are followed — so
    /// the callback belongs to the request the caller wrote, and the scan cannot
    /// wander off into the call graph.
    fn with_callee_resolution(
        &self,
        ctx: &Ctx<'a>,
        func: &syn::Expr,
        self_ty: &Option<String>,
    ) -> Option<Ctx<'a>> {
        let (callee_self, callee_name) = self.callee_key(func, self_ty)?;
        let callee = self.index.find_fn(callee_self.as_deref(), &callee_name)?;
        let mut events = Vec::new();
        self.collect_callback_events_in_block(callee.block, 0, &mut events);
        if events.is_empty() {
            return None;
        }
        let mut resolved = ctx.clone();
        resolved.resolution = events;
        Some(resolved)
    }

    /// Every event declared by a `then_send` inside a block.
    fn collect_callback_events_in_block(
        &self,
        block: &syn::Block,
        depth: usize,
        out: &mut Vec<String>,
    ) {
        if depth >= MAX_EVENT_VALUE_DEPTH || !self.budget.enter() {
            return;
        }
        for statement in &block.stmts {
            match statement {
                syn::Stmt::Expr(expr, _) => {
                    self.collect_callback_events(expr, depth + 1, out);
                }
                syn::Stmt::Local(local) => {
                    if let Some(init) = &local.init {
                        self.collect_callback_events(&init.expr, depth + 1, out);
                    }
                }
                _ => {}
            }
        }
        self.budget.leave();
    }

    /// Finds the `then_send` calls in an expression and collects what they
    /// declare. Structural recursion only — the callback's own body is read by
    /// [`Self::collect_event_values`].
    fn collect_callback_events(&self, expr: &syn::Expr, depth: usize, out: &mut Vec<String>) {
        if depth >= MAX_EVENT_VALUE_DEPTH {
            return;
        }
        let deeper = depth + 1;
        match expr {
            syn::Expr::MethodCall(call) => {
                if call.method == "then_send" {
                    for arg in &call.args {
                        self.collect_event_values(arg, deeper, out);
                    }
                }
                self.collect_callback_events(&call.receiver, deeper, out);
                for arg in &call.args {
                    self.collect_callback_events(arg, deeper, out);
                }
            }
            syn::Expr::Call(call) => {
                for arg in &call.args {
                    self.collect_callback_events(arg, deeper, out);
                }
            }
            syn::Expr::Match(expr_match) => {
                for arm in &expr_match.arms {
                    self.collect_callback_events(&arm.body, deeper, out);
                }
            }
            syn::Expr::If(expr_if) => {
                self.collect_callback_events_in_block(&expr_if.then_branch, deeper, out);
                if let Some((_, else_expr)) = &expr_if.else_branch {
                    self.collect_callback_events(else_expr, deeper, out);
                }
            }
            syn::Expr::Block(block) => {
                self.collect_callback_events_in_block(&block.block, deeper, out)
            }
            syn::Expr::Return(ret) => {
                if let Some(inner) = &ret.expr {
                    self.collect_callback_events(inner, deeper, out);
                }
            }
            syn::Expr::Paren(paren) => self.collect_callback_events(&paren.expr, deeper, out),
            syn::Expr::Group(group) => self.collect_callback_events(&group.expr, deeper, out),
            _ => {}
        }
    }

    /// Every event the arguments of one call build, in first-seen order.
    fn declared_events(
        &self,
        args: &syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>,
    ) -> Vec<String> {
        let mut events = Vec::new();
        for arg in args {
            self.collect_event_values(arg, 0, &mut events);
        }
        events
    }

    /// Warns when a callback sink declares an answer this analysis cannot read:
    /// `then_send` is crux's own "answer with this", so an argument that builds
    /// no event is evidence we have and cannot resolve.
    fn check_resolution_sink(&mut self, call: &syn::ExprMethodCall, file: &Path) {
        if call.method != "then_send" || call.args.is_empty() {
            return;
        }
        self.warnings.push(Warning {
            file: file.to_path_buf(),
            line: call.span().start().line,
            kind: WarningKind::UnresolvedEffectCallback,
        });
    }

    /// Every event label an expression *constructs*, as opposed to matches.
    ///
    /// `Event::Started` yields `Started`; a wrapper delegates to what it wraps
    /// (`Event::Recorder(RecorderEvent::Started)` → `Started`); a callback
    /// yields everything its body can build, because a closure that matches on
    /// the shell's result answers with a different event per outcome and all of
    /// them are real:
    ///
    /// ```text
    /// Command::request_from_shell(op).then_send(move |result| match result {
    ///     AudioResult::Started { id } => RecordingEvent::RecordingStarted { id },
    ///     AudioResult::Failed(message) => RecordingEvent::RecordingFailed { message },
    /// })
    /// ```
    ///
    /// Depth-guarded like the pattern walker: expressions nest without limit in
    /// hostile input.
    fn collect_event_values(&self, expr: &syn::Expr, depth: usize, out: &mut Vec<String>) {
        if depth >= MAX_EVENT_VALUE_DEPTH {
            return;
        }
        let deeper = depth + 1;
        // A constructed variant ends the descent: what it carries is a payload,
        // not another answer — except for a wrapper, which *is* the delegation.
        if let Some((enum_name, variant)) = enum_variant_of_expr(expr) {
            if self.is_event_enum(&enum_name) {
                if self.is_wrapper_variant(&enum_name, &variant) {
                    if let syn::Expr::Call(call) = expr {
                        for arg in &call.args {
                            self.collect_event_values(arg, deeper, out);
                        }
                    }
                    if let syn::Expr::Struct(strct) = expr {
                        for field in &strct.fields {
                            self.collect_event_values(&field.expr, deeper, out);
                        }
                    }
                    return;
                }
                if !out.contains(&variant) {
                    out.push(variant);
                }
                return;
            }
        }

        match expr {
            syn::Expr::Closure(closure) => self.collect_event_values(&closure.body, deeper, out),
            syn::Expr::Paren(paren) => self.collect_event_values(&paren.expr, deeper, out),
            syn::Expr::Group(group) => self.collect_event_values(&group.expr, deeper, out),
            syn::Expr::Reference(reference) => {
                self.collect_event_values(&reference.expr, deeper, out)
            }
            syn::Expr::Match(expr_match) => {
                for arm in &expr_match.arms {
                    self.collect_event_values(&arm.body, deeper, out);
                }
            }
            syn::Expr::If(expr_if) => {
                for statement in &expr_if.then_branch.stmts {
                    self.collect_event_statement(statement, deeper, out);
                }
                if let Some((_, else_expr)) = &expr_if.else_branch {
                    self.collect_event_values(else_expr, deeper, out);
                }
            }
            syn::Expr::Block(block) => {
                for statement in &block.block.stmts {
                    self.collect_event_statement(statement, deeper, out);
                }
            }
            syn::Expr::Call(call) => {
                for arg in &call.args {
                    self.collect_event_values(arg, deeper, out);
                }
            }
            syn::Expr::MethodCall(call) => {
                for arg in &call.args {
                    self.collect_event_values(arg, deeper, out);
                }
            }
            syn::Expr::Return(ret) => {
                if let Some(inner) = &ret.expr {
                    self.collect_event_values(inner, deeper, out);
                }
            }
            _ => {}
        }
    }

    /// A callback's body is usually `let event = match … ;` followed by the
    /// event it built, so statements count as much as trailing expressions.
    fn collect_event_statement(&self, statement: &syn::Stmt, depth: usize, out: &mut Vec<String>) {
        match statement {
            syn::Stmt::Expr(expr, _) => self.collect_event_values(expr, depth, out),
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.collect_event_values(&init.expr, depth, out);
                }
            }
            _ => {}
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
    /// The states the machine can be in where `subject` is written.
    ///
    /// `subject` is the receiver of the assignment being explained, and it is
    /// what keeps a guard about *another* record from constraining this one.
    fn source_states(
        &self,
        ctx: &Ctx<'a>,
        machine: &StateMachine,
        subject: Option<&str>,
    ) -> GuardEval {
        let mut result = GuardEval::NoConstraint;
        let scope = SourceScope {
            subject,
            self_fields: &["self"],
        };

        for fact in &ctx.facts {
            if fact.machine == machine.enum_name
                && fact.field == machine.field_name
                && receivers_may_alias(subject, fact.receiver.as_deref())
            {
                result = and(result, GuardEval::Known(fact.states.clone()));
            }
        }
        for condition in &ctx.conditions {
            result = and(result, self.eval_condition(condition, machine, scope, 0));
        }
        result
    }

    /// Whether `expr` denotes this machine's state field **on the subject
    /// object** — the test that used to be a bare field-name comparison.
    fn is_subject_state_field(
        &self,
        expr: &syn::Expr,
        machine: &StateMachine,
        scope: SourceScope,
    ) -> bool {
        let Some(field) = last_field_name(expr) else {
            return false;
        };
        if field != machine.field_name && !scope.self_fields.contains(&field.as_str()) {
            return false;
        }
        receivers_may_alias(scope.subject, receiver_path(expr).as_deref())
    }

    /// What `condition` says about `machine`'s current state on `scope.subject`.
    ///
    /// Charged and depth-guarded: `&&`/`||` chains and `!` nest arbitrarily in
    /// hostile input. Being cut off reports `Unresolved` — there *is* a
    /// condition here and we could not resolve it — which drops the transition
    /// with a warning rather than inventing a source state.
    fn eval_condition(
        &self,
        condition: &syn::Expr,
        machine: &StateMachine,
        scope: SourceScope,
        depth: usize,
    ) -> GuardEval {
        if !self.budget.enter() {
            return GuardEval::Unresolved;
        }
        let result = self.eval_condition_inner(condition, machine, scope, depth);
        self.budget.leave();
        result
    }

    fn eval_condition_inner(
        &self,
        condition: &syn::Expr,
        machine: &StateMachine,
        scope: SourceScope,
        depth: usize,
    ) -> GuardEval {
        match condition {
            syn::Expr::Macro(expr_macro) => {
                let Some(args) = as_matches_macro(&expr_macro.mac) else {
                    return GuardEval::NoConstraint;
                };
                if !self.is_subject_state_field(&args.expr, machine, scope) {
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
                if !self.is_subject_state_field(&call.receiver, machine, scope) {
                    return GuardEval::NoConstraint;
                }
                self.eval_predicate(&call.method.to_string(), machine, depth)
            }
            syn::Expr::Binary(binary) => match binary.op {
                syn::BinOp::And(_) => and(
                    self.eval_condition(&binary.left, machine, scope, depth),
                    self.eval_condition(&binary.right, machine, scope, depth),
                ),
                syn::BinOp::Or(_) => or(
                    self.eval_condition(&binary.left, machine, scope, depth),
                    self.eval_condition(&binary.right, machine, scope, depth),
                ),
                // `state == State::X` and `state != State::X` comparisons.
                syn::BinOp::Eq(_) | syn::BinOp::Ne(_) => {
                    let Some(variant) =
                        self.comparison_variant(&binary.left, &binary.right, machine, scope)
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
                    and(acc, self.eval_condition(body, machine, scope, depth))
                }),
            syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Not(_)) => {
                match self.eval_condition(&unary.expr, machine, scope, depth) {
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
            syn::Expr::Paren(paren) => self.eval_condition(&paren.expr, machine, scope, depth),
            syn::Expr::Group(group) => self.eval_condition(&group.expr, machine, scope, depth),
            // A block condition (e.g. a closure body) is its trailing expression.
            syn::Expr::Block(block) => match block.block.stmts.last() {
                Some(syn::Stmt::Expr(trailing, None)) => {
                    self.eval_condition(trailing, machine, scope, depth)
                }
                _ => GuardEval::NoConstraint,
            },
            _ => GuardEval::NoConstraint,
        }
    }

    /// `state == State::X` (either side order) → the state leaf, when the
    /// other side is the subject's state field.
    fn comparison_variant(
        &self,
        left: &syn::Expr,
        right: &syn::Expr,
        machine: &StateMachine,
        scope: SourceScope,
    ) -> Option<String> {
        let is_state_field = |expr: &syn::Expr| self.is_subject_state_field(expr, machine, scope);

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
        if !self.budget.enter() {
            return Vec::new();
        }
        let result = self.state_leaves_of_pattern_inner(pat, machine);
        self.budget.leave();
        result
    }

    /// Nests through `|`, `@`, parens and references, so a pattern of nothing
    /// but `((((…))))` recurses as deep as the input is nested.
    fn state_leaves_of_pattern_inner(&self, pat: &syn::Pat, machine: &StateMachine) -> Vec<String> {
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
            syn::Pat::Reference(reference) => self.state_leaves_of_pattern(&reference.pat, machine),
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
                let syn::Expr::Call(call) = expr else {
                    return None;
                };
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
        // Inside the body the subject is the predicate's own receiver, which the
        // call site already checked — so no subject to discriminate against here.
        let self_fields = ["self", machine.field_name.as_str()];
        let scope = SourceScope {
            subject: None,
            self_fields: &self_fields,
        };
        match self.eval_condition(trailing, machine, scope, depth + 1) {
            GuardEval::NoConstraint => GuardEval::Unresolved,
            resolved => resolved,
        }
    }

    // ---- transition emission ------------------------------------------------

    fn handle_assignment(&mut self, assign: &'a syn::ExprAssign, ctx: &Ctx<'a>, file: &Path) {
        // `*.state = Enum::Variant` — a direct transition target
        // (composite children included: `State::Active(ActiveState::Ready)`).
        if let Some(field) = last_field_name(&assign.left) {
            let assigned = enum_variant_of_expr(&assign.right).map(|(enum_name, _)| enum_name);
            if let Some(machine) = self.machine_for_field_and_enum(&field, assigned.as_deref()) {
                let machine = machine.clone();
                if let Some(to) = self.state_leaf_of_expr(&assign.right, &machine) {
                    let subject = receiver_path(&assign.left);
                    self.emit(&machine, to, ctx, assign, subject.as_deref(), file);
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
            // The reset struct is itself the object holding the state field, so
            // the whole left-hand side is the subject rather than its receiver.
            let subject = expr_path_string(&assign.left);
            for (machine, to) in reset_targets {
                self.emit(&machine, to, ctx, assign, subject.as_deref(), file);
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
        // A direct write to the state field: the object is the receiver.
        let subject = receiver_path(&assign.left);
        if let Some(path) = expr_path_string(&assign.right) {
            // Event payload binding of the machine's enum type.
            if !path.contains('.') && ctx.payload_bindings.get(&path) == Some(&machine.enum_name) {
                self.emit(
                    machine,
                    ANY_STATE.to_string(),
                    ctx,
                    assign,
                    subject.as_deref(),
                    file,
                );
                return;
            }

            // Conditions constraining this exact value expression.
            let mut eval = GuardEval::NoConstraint;
            for condition in &ctx.conditions {
                eval = and(
                    eval,
                    self.eval_value_condition(condition, &path, machine, 0),
                );
            }
            if let GuardEval::Known(targets) = eval {
                for to in targets {
                    self.emit(machine, to, ctx, assign, subject.as_deref(), file);
                }
                return;
            }
        }

        self.warnings.push(Warning {
            file: file.to_path_buf(),
            line: assign.span().start().line,
            kind: WarningKind::DynamicTarget {
                machine: machine.enum_name.clone(),
            },
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
            syn::Expr::Paren(paren) => self.eval_value_condition(&paren.expr, path, machine, depth),
            syn::Expr::Group(group) => self.eval_value_condition(&group.expr, path, machine, depth),
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
        let syn::Expr::Call(call) = rhs else {
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
        let [type_name, method] = segments.last_chunk::<2>()?;
        if method != "default" {
            return None;
        }

        // Deterministic by sorted path — see the note on the same call in
        // `state_enum.rs`. The walker does know a file, but it is the file of the
        // function being walked rather than of the expression's type, so it is
        // not the referencing file this needs.
        let strct = self.index.struct_decls(type_name).first()?;
        let targets: Vec<(StateMachine, String)> = self
            .machines
            .iter()
            .filter(|machine| {
                strct.fields.iter().any(|field| {
                    field.name == machine.field_name && field.declared == machine.enum_name
                })
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

    /// `subject` names the object whose state field this assignment writes, so
    /// that guards about a different record are not read as evidence. It is not
    /// derivable from `assign` alone: writing the field directly
    /// (`model.session.state = …`) makes it the receiver, while resetting the
    /// struct that *holds* the field (`model.session = T::default()`) makes it
    /// the left-hand side itself.
    fn emit(
        &mut self,
        machine: &StateMachine,
        to: String,
        ctx: &Ctx<'a>,
        assign: &syn::ExprAssign,
        subject: Option<&str>,
        file: &Path,
    ) {
        let line = assign.span().start().line;

        let Some(events) = &ctx.events else {
            self.warnings.push(Warning {
                file: file.to_path_buf(),
                line,
                kind: WarningKind::UnknownEvent { to: to.clone() },
            });
            return;
        };

        match self.source_states(ctx, machine, subject) {
            GuardEval::NoConstraint => {
                // No state evidence: the transition fires from any state.
                for event in events {
                    self.push(
                        machine,
                        ANY_STATE.to_string(),
                        event.clone(),
                        to.clone(),
                        ctx,
                    );
                }
            }
            // Contradictory constraints: the conditions in force intersect to
            // nothing, so no state satisfies them all. Real code does not write
            // an unreachable assignment — what it usually means is that two
            // *different* objects' same-named fields were read as one, because
            // source constraints are keyed by field name while the value
            // mirror is keyed by exact path. The assignment is real either way,
            // so it must not vanish silently. See `docs/roadmap.md` §6.
            GuardEval::Known(from_states) if from_states.is_empty() => {
                self.warnings.push(Warning {
                    file: file.to_path_buf(),
                    line,
                    kind: WarningKind::UnresolvableSource { to: to.clone() },
                });
            }
            GuardEval::Known(from_states) => {
                for event in events {
                    for from in &from_states {
                        self.push(machine, from.clone(), event.clone(), to.clone(), ctx);
                    }
                }
            }
            GuardEval::Unresolved => {
                self.warnings.push(Warning {
                    file: file.to_path_buf(),
                    line,
                    kind: WarningKind::UnresolvableSource { to: to.clone() },
                });
            }
        }
    }

    /// The assignment's branch travels with the transition: it is what tells
    /// [`attach`] which of the arm's requests this transition is on the path of.
    fn push(
        &mut self,
        machine: &StateMachine,
        from: String,
        event: String,
        to: String,
        ctx: &Ctx<'a>,
    ) {
        self.out.push((
            RawTransition {
                machine: machine.enum_name.clone(),
                field: machine.field_name.clone(),
                from,
                event,
                to,
                effects: Vec::new(),
            },
            ctx.arm,
            ctx.branch.clone(),
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
