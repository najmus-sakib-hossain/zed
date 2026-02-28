//! Parallel CSS generation using rayon
//!
//! Leverages multi-core CPUs to generate CSS for multiple classes simultaneously

use rayon::prelude::*;
use std::collections::HashMap;

/// Generate CSS for multiple classes in parallel
/// Uses rayon to distribute work across CPU cores
#[allow(dead_code)]
pub fn generate_css_parallel<'a, F>(classes: &[&'a str], generator: F) -> HashMap<&'a str, String>
where
    F: Fn(&str) -> Option<String> + Sync,
{
    classes
        .par_iter()
        .filter_map(|&class| generator(class).map(|css| (class, css)))
        .collect()
}

/// Generate CSS for multiple classes in parallel with a threshold
/// Only uses parallel processing if class count exceeds threshold
#[allow(dead_code)]
pub fn generate_css_adaptive<'a, F>(
    classes: &[&'a str],
    generator: F,
    parallel_threshold: usize,
) -> HashMap<&'a str, String>
where
    F: Fn(&str) -> Option<String> + Sync,
{
    if classes.len() >= parallel_threshold {
        generate_css_parallel(classes, generator)
    } else {
        // Sequential for small batches (avoid thread overhead)
        classes
            .iter()
            .filter_map(|&class| generator(class).map(|css| (class, css)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_generation() {
        let classes = vec!["flex", "block", "hidden", "text-center"];
        let generator = |class: &str| Some(format!(".{} {{ display: {}; }}", class, class));

        let results = generate_css_parallel(&classes, generator);
        assert_eq!(results.len(), 4);
        assert!(results.contains_key("flex"));
    }

    #[test]
    fn test_adaptive_generation() {
        let small = vec!["flex", "block"];
        let large = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];

        let generator = |class: &str| Some(format!(".{} {{}}", class));

        let results_small = generate_css_adaptive(&small, &generator, 5);
        assert_eq!(results_small.len(), 2);

        let results_large = generate_css_adaptive(&large, &generator, 5);
        assert_eq!(results_large.len(), 10);
    }
}
