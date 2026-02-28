// Transformation actions for filtered elements

/// Actions that can be taken on filtered elements
#[derive(Debug, Clone)]
pub enum FilterAction {
    /// Keep the element as-is
    Keep,

    /// Remove the element entirely
    Remove,

    /// Transform the element (e.g., alt-text only for images)
    Transform,

    /// Summarize verbose content
    Summarize,
}
