use crate::core::engine::StyleEngine;
use smallvec::SmallVec;

pub fn apply_wrappers_and_states(
    engine: &StyleEngine,
    prefix_segment: &str,
) -> (SmallVec<[String; 4]>, String, SmallVec<[String; 2]>) {
    let mut media_queries: SmallVec<[String; 4]> = SmallVec::new();
    let mut pseudo_classes = String::new();
    let mut wrappers: SmallVec<[String; 2]> = SmallVec::new();
    if !prefix_segment.is_empty() {
        for part in prefix_segment.split(':') {
            if let Some(screen_value) = engine.screens.get(part) {
                media_queries.push(format!("@media (min-width: {})", screen_value));
            } else if let Some(cq_value) = engine.container_queries.get(part) {
                media_queries.push(format!("@container (min-width: {})", cq_value));
            } else if let Some(state_value) = engine.states.get(part) {
                if state_value.contains('&') {
                    wrappers.push(state_value.to_string());
                } else {
                    pseudo_classes.push_str(state_value);
                }
            } else if part == "dark" {
                wrappers.push(".dark &".to_string());
            } else if part == "light" {
                wrappers.push(":root &".to_string());
            }
        }
    }
    (media_queries, pseudo_classes, wrappers)
}
