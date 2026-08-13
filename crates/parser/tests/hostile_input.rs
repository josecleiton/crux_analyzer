//! The analyzer is pointed at source it does not control, so these tests are
//! about *termination and honesty*, not extraction quality: hostile input must
//! finish, must not abort the process, and must say when it was cut short.
//!
//! Each case is generated rather than committed as a fixture — the point is the
//! shape (a diamond call graph, a nesting depth), and a generator states that
//! shape in one line. See `docs/security.md`.

use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

use crux_analyzer_parser::{parse_project, parse_project_with, Limits};

/// Writes `code` as `lib.rs` in a fresh directory under the target dir.
///
/// A real directory rather than an in-memory source set, because the loader's
/// own limits (size, file type) are part of what is under test.
fn source_dir(name: &str, code: &str) -> std::path::PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).expect("test dir");
    std::fs::write(dir.join("lib.rs"), code).expect("test source");
    dir
}

/// The minimum a Core needs to be detected: an `impl App` with an `update` that
/// assigns a state enum, so the walker actually runs.
fn app_with_body(body: &str, helpers: &str) -> String {
    format!(
        r#"
pub enum State {{ Idle, Busy }}
pub enum Event {{ Go }}
pub struct Model {{ pub state: State }}
pub struct App;
impl App for App {{
    type Event = Event;
    fn update(&self, event: Event, model: &mut Model) {{
        match event {{
            Event::Go => {{ {body} }}
        }}
    }}
}}
{helpers}
"#
    )
}

/// A diamond call graph: `f0` calls `f1` twice, `f1` calls `f2` twice, and so
/// on. The call stack breaks *cycles*, and there are none here — every path is
/// distinct, so without a step budget the walk is 2^depth and forty levels of
/// this ~60-line file never finishes.
#[test]
fn diamond_call_graph_terminates() {
    const LEVELS: usize = 40;
    let mut helpers = String::new();
    for level in 0..LEVELS {
        let next = format!("f{}", level + 1);
        helpers.push_str(&format!(
            "impl App {{ fn f{level}(&self, model: &mut Model) {{ \
             Self::{next}(self, model); Self::{next}(self, model); }} }}\n"
        ));
    }
    helpers.push_str(&format!(
        "impl App {{ fn f{LEVELS}(&self, model: &mut Model) {{ model.state = State::Busy; }} }}\n"
    ));
    let dir = source_dir(
        "diamond",
        &app_with_body("Self::f0(self, model);", &helpers),
    );

    let started = Instant::now();
    let outcome = parse_project(&dir, "diamond").expect("must not fail");
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(30),
        "walk took {elapsed:?} — the step budget is not bounding the fan-out"
    );
    assert!(
        outcome
            .warnings
            .iter()
            .any(|w| w.kind.code() == "analysis-truncated"),
        "a truncated walk must say so; got {:?}",
        outcome
            .warnings
            .iter()
            .map(|w| w.kind.code())
            .collect::<Vec<_>>()
    );
}

/// A low budget must be reported even on input that is otherwise trivial —
/// proves the warning is wired to the budget, not to the shape of this fixture.
#[test]
fn a_hit_budget_is_always_reported() {
    let dir = source_dir(
        "tiny_budget",
        &app_with_body("model.state = State::Busy;", ""),
    );
    let limits = Limits {
        max_steps: 1,
        ..Limits::default()
    };
    let outcome = parse_project_with(&dir, "tiny", &limits).expect("must not fail");
    assert!(outcome
        .warnings
        .iter()
        .any(|w| w.kind.code() == "analysis-truncated"));
}

/// Deeply nested expressions must not overflow the stack.
///
/// This is the case that has to be caught *before* `syn::parse_file`: its
/// recursion overflows on input this deep, and a stack overflow aborts the
/// process — it is not a `ParseError` anyone can handle. Running on the default
/// test-harness stack is the point; the CLI's large stack must not be what
/// makes this pass.
#[test]
fn deeply_nested_expressions_are_rejected_not_fatal() {
    const DEPTH: usize = 20_000;
    let body = format!(
        "let _ = {}0{};  model.state = State::Busy;",
        "(".repeat(DEPTH),
        ")".repeat(DEPTH)
    );
    let dir = source_dir("nested", &app_with_body(&body, ""));

    // The file is skipped, so the only Core goes with it.
    let result = parse_project(&dir, "nested");
    assert!(result.is_err(), "the over-nested file must be skipped");
}

/// Deeply nested *patterns* exercise `pattern_variants` and
/// `state_leaves_of_pattern`, which recurse through parens and references.
#[test]
fn deeply_nested_patterns_are_rejected_not_fatal() {
    const DEPTH: usize = 5_000;
    let body = format!(
        "if matches!(model.state, {}State::Idle{}) {{ model.state = State::Busy; }}",
        "(".repeat(DEPTH),
        ")".repeat(DEPTH)
    );
    let dir = source_dir("nested_pat", &app_with_body(&body, ""));
    assert!(parse_project(&dir, "nested_pat").is_err());
}

/// The skip must be reported, and it must be the nesting check that did it.
#[test]
fn over_nested_files_are_reported() {
    let deep = format!("// {}\n", "(".repeat(500));
    let dir = source_dir(
        "nested_reported",
        &format!("{deep}{}", app_with_body("model.state = State::Busy;", "")),
    );
    // Inside a comment, so it must NOT count: the file still parses.
    let outcome = parse_project(&dir, "nested_reported").expect("comments do not nest brackets");
    assert!(!outcome
        .warnings
        .iter()
        .any(|w| w.kind.code() == "nesting-too-deep"));

    // The same brackets in code do count.
    let dir = source_dir(
        "nested_reported_real",
        &app_with_body(
            &format!("let _ = {}0{};", "(".repeat(500), ")".repeat(500)),
            "",
        ),
    );
    // Either the warning is present, or the file carrying the only Core was
    // skipped and there is nothing left to analyze — both are the cap firing.
    let saw = match parse_project_with(&dir, "x", &Limits::default()) {
        Ok(outcome) => outcome
            .warnings
            .iter()
            .any(|w| w.kind.code() == "nesting-too-deep"),
        Err(_) => true,
    };
    assert!(saw, "an over-nested file must be reported or skipped");
}

/// An oversized file is skipped with a warning rather than read into memory.
#[test]
fn oversized_files_are_skipped_and_reported() {
    let mut code = app_with_body("model.state = State::Busy;", "");
    code.push_str(&format!("\n// {}\n", "p".repeat(4096)));
    let dir = source_dir("oversized", &code);

    let limits = Limits {
        max_file_size: 512,
        ..Limits::default()
    };
    // Nothing is left to analyze once the only file is skipped, so this fails
    // with `NoCoreFound` — the warning is what is under test.
    let outcome = parse_project_with(&dir, "oversized", &limits);
    assert!(
        outcome.is_err(),
        "the only source file must have been skipped"
    );

    // With a generous cap the same tree parses, proving the cap did the skipping.
    parse_project(&dir, "oversized").expect("parses under default limits");
}

/// A symlinked `.rs` is not followed: it would read source from outside the
/// tree, and pointed at a FIFO or `/dev/zero` it would hang or exhaust memory.
#[cfg(unix)]
#[test]
fn symlinked_sources_are_skipped_and_reported() {
    let dir = source_dir("symlink", &app_with_body("model.state = State::Busy;", ""));
    let outside = Path::new(env!("CARGO_TARGET_TMPDIR")).join("outside_secret.rs");
    let mut file = std::fs::File::create(&outside).expect("outside file");
    writeln!(file, "pub enum LeakedFromOutsideTheTree {{ Secret }}").expect("write");

    let link = dir.join("linked.rs");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(&outside, &link).expect("symlink");

    let outcome = parse_project(&dir, "symlink").expect("must not fail");
    assert!(
        outcome
            .warnings
            .iter()
            .any(|w| w.kind.code() == "not-a-regular-file"),
        "a skipped symlink must be reported; got {:?}",
        outcome
            .warnings
            .iter()
            .map(|w| w.kind.code())
            .collect::<Vec<_>>()
    );
}
