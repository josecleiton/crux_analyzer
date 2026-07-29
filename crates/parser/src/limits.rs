//! Resource limits for analyzing untrusted source.
//!
//! The analyzer is pointed at whatever Rust it is given — a dependency, a fork's
//! pull request, a crate someone downloaded. That input controls the shape of
//! the AST, and three dimensions of it are otherwise unbounded: how much source
//! is read, how deeply expressions nest, and how many paths the call-following
//! walker explores.
//!
//! Every limit here obeys the parser honesty rule: hitting one produces a
//! [`Warning`](crate::Warning), never a silent truncation. A model that was cut
//! short says so, and `--deny-warnings` therefore makes truncation fatal in CI
//! without any extra plumbing.

/// Caps on what one analysis run may read and explore.
///
/// The defaults are far above any real Crux application — the fixture uses a
/// four-figure step count — and far below what it takes to hang a machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Largest single `.rs` file that will be read, in bytes.
    pub max_file_size: u64,
    /// Largest total volume of source read in one run, in bytes.
    pub max_total_size: u64,
    /// Expression-walking steps allowed per Core.
    ///
    /// Bounds the call-following walker, whose worst case is exponential in the
    /// call graph: a diamond of helpers (`f0` calls `f1` twice, `f1` calls `f2`
    /// twice, ...) is re-walked once per distinct path, so forty small
    /// functions describe 2^40 walks. Memoizing is not an option — a helper is
    /// legitimately re-walked under a different [`Ctx`](crate::transitions) and
    /// yields different transitions each time — so the total work is what gets
    /// bounded, and the cut is reported.
    pub max_steps: u64,
    /// Maximum nesting depth for expression, condition and pattern recursion.
    pub max_depth: usize,
    /// Maximum helper-call nesting depth while following calls.
    pub max_call_depth: usize,
    /// Maximum bracket nesting accepted in a source file.
    ///
    /// Checked against the raw text *before* `syn` sees it, because
    /// `syn::parse_file` recurses over nesting and a stack overflow aborts the
    /// process — an abort no `Result` can intercept and no depth cap inside the
    /// walkers can prevent. This is the one limit that has to run first.
    pub max_nesting: usize,
}

impl Limits {
    /// 2 MiB per file. The largest file in the private corpus is under 40 KiB.
    pub const DEFAULT_MAX_FILE_SIZE: u64 = 2 << 20;
    /// 256 MiB per run.
    pub const DEFAULT_MAX_TOTAL_SIZE: u64 = 256 << 20;
    /// 2,000,000 steps per Core.
    pub const DEFAULT_MAX_STEPS: u64 = 2_000_000;
    /// 256 levels of expression nesting.
    pub const DEFAULT_MAX_DEPTH: usize = 256;
    /// 64 levels of helper-call nesting.
    pub const DEFAULT_MAX_CALL_DEPTH: usize = 64;
    /// 192 levels of bracket nesting. `rustc` itself gives up long before this;
    /// the deepest nesting in the private corpus is under 20.
    pub const DEFAULT_MAX_NESTING: usize = 192;
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_file_size: Self::DEFAULT_MAX_FILE_SIZE,
            max_total_size: Self::DEFAULT_MAX_TOTAL_SIZE,
            max_steps: Self::DEFAULT_MAX_STEPS,
            max_depth: Self::DEFAULT_MAX_DEPTH,
            max_call_depth: Self::DEFAULT_MAX_CALL_DEPTH,
            max_nesting: Self::DEFAULT_MAX_NESTING,
        }
    }
}
