//! # Analyzer Module - The Intelligence
//!
//! Analyzes parsed AST to determine optimal runtime strategy.
//!
//! ## Decision Matrix
//! ```text
//! | Components | State Complexity | Events | Choice       |
//! |-----------|------------------|--------|--------------|
//! | < 10      | Low              | < 5    | MICRO (338B) |
//! | < 10      | Medium           | < 10   | MICRO (338B) |
//! | >= 10     | Any              | Any    | MACRO (7.5KB)|
//! | Any       | High             | Any    | MACRO (7.5KB)|
//! | Any       | Any              | >= 10  | MACRO (7.5KB)|
//! ```
//!
//! ## Philosophy
//! "The developer writes code. The compiler decides how to execute it."

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::parser::ParsedModule;

/// Runtime variant selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeVariant {
    /// Micro: 338 bytes Brotli - For simple, static-heavy apps
    Micro,
    /// Macro: 7.5 KB Brotli - For complex, interactive apps
    Macro,
}

impl RuntimeVariant {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeVariant::Micro => "micro",
            RuntimeVariant::Macro => "macro",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            RuntimeVariant::Micro => "Micro (338 bytes) - Optimized for simplicity",
            RuntimeVariant::Macro => "Macro (7.5 KB) - Optimized for complexity",
        }
    }
}

/// Complexity metrics extracted from the AST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    pub component_count: usize,
    pub total_state_vars: usize,
    pub total_props: usize,
    pub total_hooks: usize,
    pub event_handler_count: usize,
    pub max_component_depth: usize,
    pub has_async_logic: bool,
    pub has_effects: bool,
    pub total_jsx_nodes: usize,
    pub state_complexity: StateComplexity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateComplexity {
    /// No state or 1-2 simple primitive state vars
    Low,
    /// 3-5 state vars, simple types
    Medium,
    /// 6+ state vars, complex objects, or arrays
    High,
}

/// Analyze parsed modules and determine optimal runtime
pub fn analyze_and_decide(
    modules: &[ParsedModule],
    verbose: bool,
) -> Result<(ComplexityMetrics, RuntimeVariant)> {
    let metrics = compute_metrics(modules);

    if verbose {
        println!("\n  📊 Complexity Analysis:");
        println!("     Components:      {}", metrics.component_count);
        println!("     State Variables: {}", metrics.total_state_vars);
        println!("     Event Handlers:  {}", metrics.event_handler_count);
        println!("     JSX Nodes:       {}", metrics.total_jsx_nodes);
        println!("     State:           {:?}", metrics.state_complexity);
        println!();
    }

    let variant = decide_runtime(&metrics);

    if verbose {
        println!("  🎯 Decision: {}", variant.description());
        println!();
    }

    Ok((metrics, variant))
}

/// Compute complexity metrics from parsed modules
fn compute_metrics(modules: &[ParsedModule]) -> ComplexityMetrics {
    let mut component_count = 0;
    let mut total_state_vars = 0;
    let mut total_props = 0;
    let mut total_hooks = 0;
    let mut event_handler_count = 0;
    let mut has_async_logic = false;
    let mut has_effects = false;
    let mut total_jsx_nodes = 0;

    for module in modules {
        component_count += module.components.len();

        for component in &module.components {
            total_state_vars += component.state.len();
            total_props += component.props.len();
            total_hooks += component.hooks.len();

            // Count event handlers (simplified heuristic)
            event_handler_count += count_event_handlers(&component.jsx_body);

            // Count JSX nodes (simplified: count opening tags)
            total_jsx_nodes += component.jsx_body.matches('<').count();

            // Check for async logic
            if component.jsx_body.contains("async") || component.jsx_body.contains("await") {
                has_async_logic = true;
            }

            // Check for effects
            for hook in &component.hooks {
                if hook.hook_name == "useEffect" || hook.hook_name == "useLayoutEffect" {
                    has_effects = true;
                }
            }
        }
    }

    let state_complexity = classify_state_complexity(total_state_vars, modules);

    ComplexityMetrics {
        component_count,
        total_state_vars,
        total_props,
        total_hooks,
        event_handler_count,
        max_component_depth: estimate_component_depth(modules),
        has_async_logic,
        has_effects,
        total_jsx_nodes,
        state_complexity,
    }
}

/// Decide which runtime variant to use based on metrics
fn decide_runtime(metrics: &ComplexityMetrics) -> RuntimeVariant {
    // Rule 1: High state complexity -> Macro
    if metrics.state_complexity == StateComplexity::High {
        return RuntimeVariant::Macro;
    }

    // Rule 2: Many components -> Macro
    if metrics.component_count >= 10 {
        return RuntimeVariant::Macro;
    }

    // Rule 3: Many event handlers -> Macro
    if metrics.event_handler_count >= 10 {
        return RuntimeVariant::Macro;
    }

    // Rule 4: Complex async logic -> Macro
    if metrics.has_async_logic && metrics.total_hooks > 3 {
        return RuntimeVariant::Macro;
    }

    // Rule 5: Many effects -> Macro
    if metrics.has_effects && metrics.total_hooks > 5 {
        return RuntimeVariant::Macro;
    }

    // Rule 6: Deep component trees -> Macro
    if metrics.max_component_depth > 5 {
        return RuntimeVariant::Macro;
    }

    // Rule 7: Large JSX trees -> Macro
    if metrics.total_jsx_nodes > 50 {
        return RuntimeVariant::Macro;
    }

    // Default: Micro for simple apps
    RuntimeVariant::Micro
}

/// Classify state complexity based on number and types
fn classify_state_complexity(state_count: usize, modules: &[ParsedModule]) -> StateComplexity {
    if state_count == 0 || state_count <= 2 {
        return StateComplexity::Low;
    }

    if state_count >= 6 {
        return StateComplexity::High;
    }

    // Check for complex types (arrays, objects)
    for module in modules {
        for component in &module.components {
            for state in &component.state {
                let ty = &state.type_annotation;
                if ty.contains('[')
                    || ty.contains('{')
                    || ty.contains("Array")
                    || ty.contains("Map")
                    || ty.contains("Set")
                {
                    return StateComplexity::High;
                }
            }
        }
    }

    StateComplexity::Medium
}

/// Estimate component nesting depth (simplified heuristic)
fn estimate_component_depth(modules: &[ParsedModule]) -> usize {
    let mut max_depth = 1;

    for module in modules {
        for component in &module.components {
            // Count component references in JSX (simplified: uppercase tags)
            let depth = component.jsx_body.matches("<[A-Z]").count();
            if depth > max_depth {
                max_depth = depth;
            }
        }
    }

    max_depth
}

/// Count event handlers in JSX (onClick, onChange, etc.)
fn count_event_handlers(jsx: &str) -> usize {
    let patterns = [
        "onClick",
        "onChange",
        "onSubmit",
        "onInput",
        "onBlur",
        "onFocus",
        "onKeyDown",
        "onKeyUp",
        "onMouseOver",
        "onMouseOut",
    ];

    let mut count = 0;
    for pattern in &patterns {
        count += jsx.matches(pattern).count();
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_micro_decision() {
        let metrics = ComplexityMetrics {
            component_count: 3,
            total_state_vars: 2,
            total_props: 5,
            total_hooks: 1,
            event_handler_count: 3,
            max_component_depth: 2,
            has_async_logic: false,
            has_effects: false,
            total_jsx_nodes: 15,
            state_complexity: StateComplexity::Low,
        };

        assert_eq!(decide_runtime(&metrics), RuntimeVariant::Micro);
    }

    #[test]
    fn test_macro_decision_many_components() {
        let metrics = ComplexityMetrics {
            component_count: 15,
            total_state_vars: 3,
            total_props: 10,
            total_hooks: 2,
            event_handler_count: 5,
            max_component_depth: 3,
            has_async_logic: false,
            has_effects: false,
            total_jsx_nodes: 30,
            state_complexity: StateComplexity::Low,
        };

        assert_eq!(decide_runtime(&metrics), RuntimeVariant::Macro);
    }

    #[test]
    fn test_macro_decision_high_state() {
        let metrics = ComplexityMetrics {
            component_count: 5,
            total_state_vars: 8,
            total_props: 5,
            total_hooks: 3,
            event_handler_count: 4,
            max_component_depth: 2,
            has_async_logic: false,
            has_effects: false,
            total_jsx_nodes: 20,
            state_complexity: StateComplexity::High,
        };

        assert_eq!(decide_runtime(&metrics), RuntimeVariant::Macro);
    }
}
