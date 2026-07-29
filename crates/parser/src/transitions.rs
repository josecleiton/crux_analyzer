//! Transition extraction: walks a Core's `update` and every helper it calls,
//! carrying the current event label(s) and source-state set, and records a
//! transition at each state assignment.

use std::path::{Path, PathBuf};

use syn::spanned::Spanned;

use crate::ast_util::{
    as_matches_macro, enum_variant_of_expr, is_catch_all, last_field_name, pattern_variants,
};
use crate::core_finder::CoreInfo;
use crate::index::CrateIndex;
use crate::state_enum::StateMachine;
use crate::Warning;

/// A transition attributed to a specific state machine (enum).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawTransition {
    pub machine: String,
    pub from: String,
    pub event: String,
    pub to: String,
}

/// Event labels currently in scope. `None` = not statically known.
type EventCtx = Option<Vec<String>>;
/// Source states currently in scope for the machine. `None` = not known.
type FromCtx = Option<Vec<String>>;

#[derive(Clone)]
struct Ctx {
    events: EventCtx,
    from: FromCtx,
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
            from: None,
        },
        update.self_ty.clone(),
        update.file,
    );
    walker.out
}

struct Walker<'a> {
    index: &'a CrateIndex<'a>,
    core: &'a CoreInfo,
    machines: &'a [StateMachine],
    warnings: &'a mut Vec<Warning>,
    out: Vec<RawTransition>,
    /// Functions currently being walked — breaks recursion cycles while still
    /// allowing the same helper to be re-walked under a different context.
    call_stack: Vec<(Option<String>, String)>,
}

impl<'a> Walker<'a> {
    fn machine_for_field(&self, field: &str) -> Option<&'a StateMachine> {
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

    fn walk_block(&mut self, block: &syn::Block, ctx: &Ctx, self_ty: Option<String>, file: &Path) {
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

    fn walk_expr(&mut self, expr: &syn::Expr, ctx: &Ctx, self_ty: &Option<String>, file: &Path) {
        match expr {
            syn::Expr::Assign(assign) => {
                self.handle_assignment(assign, ctx, file);
                self.walk_expr(&assign.right, ctx, self_ty, file);
            }
            syn::Expr::Match(expr_match) => self.walk_match(expr_match, ctx, self_ty, file),
            syn::Expr::If(expr_if) => {
                // `if matches!(state, A | B) { ... }` narrows the source set
                // inside the then-branch (only through `&&` conjunctions —
                // never through `!` or `||`).
                let narrowed = self.source_states_in_condition(&expr_if.cond);
                let then_ctx = Ctx {
                    events: ctx.events.clone(),
                    from: narrowed.or_else(|| ctx.from.clone()),
                };
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
    fn follow_call(&mut self, func: &syn::Expr, ctx: &Ctx, self_ty: &Option<String>) {
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
        expr_match: &syn::ExprMatch,
        ctx: &Ctx,
        self_ty: &Option<String>,
        file: &Path,
    ) {
        // A match on the state field drives the `from` context per arm.
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
        expr_match: &syn::ExprMatch,
        machine: StateMachine,
        ctx: &Ctx,
        self_ty: &Option<String>,
        file: &Path,
    ) {
        let all_variants = machine.variants.clone();
        let mut seen: Vec<String> = Vec::new();

        for arm in &expr_match.arms {
            let mut variants = Vec::new();
            pattern_variants(&arm.pat, &mut variants);
            let arm_states: Vec<String> = variants
                .into_iter()
                .filter(|(e, _)| *e == machine.enum_name)
                .map(|(_, v)| v)
                .collect();

            let from = if !arm_states.is_empty() {
                arm_states
            } else if is_catch_all(&arm.pat) {
                // `_` matches whatever earlier arms did not.
                all_variants
                    .iter()
                    .filter(|v| !seen.contains(v))
                    .cloned()
                    .collect()
            } else {
                Vec::new()
            };

            seen.extend(from.iter().cloned());

            let arm_ctx = Ctx {
                events: ctx.events.clone(),
                from: if from.is_empty() { ctx.from.clone() } else { Some(from) },
            };
            if let Some((_, guard)) = &arm.guard {
                self.walk_expr(guard, ctx, self_ty, file);
            }
            self.walk_expr(&arm.body, &arm_ctx, self_ty, file);
        }
    }

    fn walk_match_on_event(
        &mut self,
        expr_match: &syn::ExprMatch,
        ctx: &Ctx,
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

            // A guard can narrow the source states via `matches!`.
            let guard_from = arm
                .guard
                .as_ref()
                .and_then(|(_, guard)| self.source_states_in_condition(guard));

            let arm_ctx = Ctx {
                events,
                from: guard_from.or_else(|| ctx.from.clone()),
            };
            if let Some((_, guard)) = &arm.guard {
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

    /// Source states asserted by a condition — only `matches!(state_field, …)`
    /// reached through `&&` conjunctions and parens counts; negations,
    /// disjunctions and predicate methods yield nothing.
    fn source_states_in_condition(&self, cond: &syn::Expr) -> FromCtx {
        match cond {
            syn::Expr::Macro(expr_macro) => {
                let args = as_matches_macro(&expr_macro.mac)?;
                let field = last_field_name(&args.expr)?;
                let machine = self.machine_for_field(&field)?;
                let mut variants = Vec::new();
                pattern_variants(&args.pat, &mut variants);
                let states: Vec<String> = variants
                    .into_iter()
                    .filter(|(e, _)| *e == machine.enum_name)
                    .map(|(_, v)| v)
                    .collect();
                (!states.is_empty()).then_some(states)
            }
            syn::Expr::Binary(binary) if matches!(binary.op, syn::BinOp::And(_)) => {
                let left = self.source_states_in_condition(&binary.left);
                let right = self.source_states_in_condition(&binary.right);
                match (left, right) {
                    (Some(l), Some(r)) => {
                        // Both sides constrain the state: intersect.
                        Some(l.into_iter().filter(|v| r.contains(v)).collect())
                    }
                    (Some(l), None) => Some(l),
                    (None, Some(r)) => Some(r),
                    (None, None) => None,
                }
            }
            syn::Expr::Paren(paren) => self.source_states_in_condition(&paren.expr),
            syn::Expr::Group(group) => self.source_states_in_condition(&group.expr),
            _ => None,
        }
    }

    // ---- transition emission ------------------------------------------------

    fn handle_assignment(&mut self, assign: &syn::ExprAssign, ctx: &Ctx, file: &Path) {
        let Some(field) = last_field_name(&assign.left) else {
            return;
        };
        let Some(machine) = self.machine_for_field(&field) else {
            return;
        };
        let Some((enum_name, to)) = enum_variant_of_expr(&assign.right) else {
            return;
        };
        if enum_name != machine.enum_name {
            return;
        }

        let line = assign.span().start().line;
        match (&ctx.events, &ctx.from) {
            (Some(events), Some(from)) => {
                for event in events {
                    for source in from {
                        self.out.push(RawTransition {
                            machine: machine.enum_name.clone(),
                            from: source.clone(),
                            event: event.clone(),
                            to: to.clone(),
                        });
                    }
                }
            }
            (None, _) => self.warnings.push(Warning {
                file: file.to_path_buf(),
                line,
                message: format!(
                    "transition to `{to}` dropped: could not infer the triggering event"
                ),
            }),
            (_, None) => self.warnings.push(Warning {
                file: file.to_path_buf(),
                line,
                message: format!(
                    "transition to `{to}` dropped: could not infer the source state \
                     (e.g. guarded by a predicate method)"
                ),
            }),
        }
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
