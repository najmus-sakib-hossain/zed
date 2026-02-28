//! Built-in lint rules
//!
//! Core rules for JavaScript/TypeScript linting.

mod eqeqeq;
mod no_alert;
mod no_console;
mod no_constant_condition;
mod no_debugger;
mod no_duplicate_keys;
mod no_empty;
mod no_eval;
mod no_sparse_arrays;
mod no_unreachable;
mod no_unsafe_finally;
mod no_unused_vars;
mod no_var;
mod no_with;
mod prefer_const;

#[cfg(test)]
mod tests;

pub use eqeqeq::Eqeqeq;
pub use no_alert::NoAlert;
pub use no_console::NoConsole;
pub use no_constant_condition::NoConstantCondition;
pub use no_debugger::NoDebugger;
pub use no_duplicate_keys::NoDuplicateKeys;
pub use no_empty::NoEmpty;
pub use no_eval::NoEval;
pub use no_sparse_arrays::NoSparseArrays;
pub use no_unreachable::NoUnreachable;
pub use no_unsafe_finally::NoUnsafeFinally;
pub use no_unused_vars::NoUnusedVars;
pub use no_var::NoVar;
pub use no_with::NoWith;
pub use prefer_const::PreferConst;

use super::Rule;

/// Get all built-in rules
#[must_use]
pub fn all_rules() -> Vec<Box<dyn Rule>> {
    vec![
        // Correctness rules
        Box::new(NoEmpty::default()),
        Box::new(NoDuplicateKeys),
        Box::new(NoUnreachable),
        Box::new(NoConstantCondition),
        Box::new(NoUnsafeFinally),
        Box::new(NoSparseArrays),
        // Suspicious rules
        Box::new(NoConsole::default()),
        Box::new(NoDebugger),
        Box::new(NoAlert),
        Box::new(NoUnusedVars::default()),
        // Style rules
        Box::new(Eqeqeq::default()),
        Box::new(PreferConst::default()),
        Box::new(NoVar),
        // Security rules
        Box::new(NoEval::default()),
        Box::new(NoWith),
    ]
}

/// Get recommended rules only
#[must_use]
pub fn recommended_rules() -> Vec<Box<dyn Rule>> {
    all_rules().into_iter().filter(|r| r.meta().recommended).collect()
}

/// Get rules by category
#[must_use]
pub fn rules_by_category(category: super::Category) -> Vec<Box<dyn Rule>> {
    all_rules().into_iter().filter(|r| r.meta().category == category).collect()
}
