//! Property-based tests for HTML Rewriter compatibility.
//!
//! Tests:
//! - Property 18: HTML Transform Correctness

use dx_compat_html::{transform_html, ContentType, HTMLRewriter};
use proptest::prelude::*;

/// Generate valid HTML tag names.
fn arb_tag_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("div".to_string()),
        Just("span".to_string()),
        Just("p".to_string()),
        Just("a".to_string()),
        Just("section".to_string()),
        Just("article".to_string()),
        Just("header".to_string()),
        Just("footer".to_string()),
    ]
}

/// Generate valid attribute names.
fn arb_attr_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("class".to_string()),
        Just("id".to_string()),
        Just("data-test".to_string()),
        Just("title".to_string()),
        Just("href".to_string()),
    ]
}

/// Generate safe attribute values (no special HTML chars).
fn arb_attr_value() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{1,20}".prop_map(|s| s)
}

/// Generate safe text content.
fn arb_text_content() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ]{0,50}".prop_map(|s| s)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 18: HTML Transform Correctness
    ///
    /// For any valid HTML element:
    /// - Attribute modifications should be reflected in output
    /// - Content insertions should appear in correct positions
    /// - Element removal should remove the element
    #[test]
    fn prop_html_attribute_modification(
        tag in arb_tag_name(),
        attr_name in arb_attr_name(),
        original_value in arb_attr_value(),
        new_value in arb_attr_value(),
        content in arb_text_content(),
    ) {
        let html = format!(r#"<{} {}="{}">{}</{}>"#, tag, attr_name, original_value, content, tag);
        let selector = format!("{}[{}]", tag, attr_name);

        let new_value_clone = new_value.clone();
        let result = transform_html(&html, &selector, move |el| {
            el.set_attribute(&attr_name, &new_value_clone);
        });

        prop_assert!(result.is_ok());
        let output = result.unwrap();

        // The new value should be in the output
        prop_assert!(output.contains(&new_value), "Output should contain new attribute value");
        // The content should be preserved
        prop_assert!(output.contains(&content), "Content should be preserved");
    }

    /// Property 18b: Content insertion preserves structure
    #[test]
    fn prop_html_content_insertion(
        tag in arb_tag_name(),
        original_content in arb_text_content(),
        appended_content in arb_text_content(),
    ) {
        let html = format!("<{}>{}</{}>", tag, original_content, tag);

        let appended_clone = appended_content.clone();
        let result = transform_html(&html, &tag, move |el| {
            el.append(&appended_clone, ContentType::Text);
        });

        prop_assert!(result.is_ok());
        let output = result.unwrap();

        // Both original and appended content should be present
        prop_assert!(output.contains(&original_content), "Original content should be preserved");
        prop_assert!(output.contains(&appended_content), "Appended content should be present");
    }

    /// Property 18c: Element removal removes element
    #[test]
    fn prop_html_element_removal(
        tag in arb_tag_name(),
        content in "[a-zA-Z]{5,20}".prop_map(|s| format!("UNIQUE_{}_CONTENT", s)),
    ) {
        // Create HTML with a removable element inside a container
        let html = format!(r#"<div><{} class="remove">{}</{}><span>keep</span></div>"#, tag, content, tag);

        let result = transform_html(&html, &format!("{}.remove", tag), |el| {
            el.remove();
        });

        prop_assert!(result.is_ok());
        let output = result.unwrap();

        // The removed element's content should not be present
        prop_assert!(!output.contains(&content), "Removed element content should not be present");
        // The kept element should still be present
        prop_assert!(output.contains("keep"), "Non-removed elements should be preserved");
    }

    /// Property 18d: Multiple handlers apply correctly
    #[test]
    fn prop_html_multiple_handlers(
        class1 in arb_attr_value(),
        class2 in arb_attr_value(),
    ) {
        let html = r#"<div><span>First</span><p>Second</p></div>"#;

        let class1_clone = class1.clone();
        let class2_clone = class2.clone();

        let mut rewriter = HTMLRewriter::new();
        rewriter.on("span", move |el| {
            el.set_attribute("class", &class1_clone);
        });
        rewriter.on("p", move |el| {
            el.set_attribute("class", &class2_clone);
        });

        let result = rewriter.transform(html);

        prop_assert!(result.is_ok());
        let output = result.unwrap();

        // Both classes should be present
        prop_assert!(output.contains(&class1), "First handler should apply");
        prop_assert!(output.contains(&class2), "Second handler should apply");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_attribute_change() {
        let html = r#"<a href="http://example.com">Link</a>"#;

        let result = transform_html(html, "a", |el| {
            if let Some(href) = el.get_attribute("href") {
                el.set_attribute("href", &href.replace("http://", "https://"));
            }
        })
        .unwrap();

        assert!(result.contains("https://example.com"));
    }

    #[test]
    fn test_add_attribute() {
        let html = r#"<a href="example.com">Link</a>"#;

        let result = transform_html(html, "a", |el| {
            el.set_attribute("target", "_blank");
            el.set_attribute("rel", "noopener");
        })
        .unwrap();

        assert!(result.contains(r#"target="_blank""#));
        assert!(result.contains(r#"rel="noopener""#));
    }

    #[test]
    fn test_prepend_content() {
        let html = r#"<div>World</div>"#;

        let result = transform_html(html, "div", |el| {
            el.prepend("Hello ", ContentType::Text);
        })
        .unwrap();

        assert!(result.contains("Hello World"));
    }

    #[test]
    fn test_replace_element() {
        let html = r#"<div><old>Replace me</old></div>"#;

        let result = transform_html(html, "old", |el| {
            el.replace("<new>Replaced</new>", ContentType::Html);
        })
        .unwrap();

        assert!(result.contains("<new>Replaced</new>"));
        assert!(!result.contains("<old>"));
    }

    #[test]
    fn test_set_inner_content() {
        let html = r#"<div>Old content</div>"#;

        let result = transform_html(html, "div", |el| {
            el.set_inner_content("New content", ContentType::Text);
        })
        .unwrap();

        assert!(result.contains("New content"));
        assert!(!result.contains("Old content"));
    }
}
