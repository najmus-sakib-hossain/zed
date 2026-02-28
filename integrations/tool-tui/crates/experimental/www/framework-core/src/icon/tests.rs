//! Tests for icon component system

use super::*;
use crate::icon;

#[test]
fn test_icon_component_new() {
    let icon = IconComponent::new("heroicons:home");
    assert_eq!(icon.name, "heroicons:home");
    assert_eq!(icon.size, 24);
    assert_eq!(icon.color, None);
    assert_eq!(icon.class, None);
}

#[test]
fn test_icon_component_with_size() {
    let icon = IconComponent::new("heroicons:home").with_size(32);
    assert_eq!(icon.size, 32);
}

#[test]
fn test_icon_component_with_color() {
    let icon = IconComponent::new("heroicons:home").with_color("#FF0000");
    assert_eq!(icon.color, Some("#FF0000".to_string()));
}

#[test]
fn test_icon_component_with_class() {
    let icon = IconComponent::new("heroicons:home").with_class("icon-large");
    assert_eq!(icon.class, Some("icon-large".to_string()));
}

#[test]
fn test_parse_name_with_set() {
    let icon = IconComponent::new("heroicons:home");
    let (set, name) = icon.parse_name();
    assert_eq!(set, "heroicons");
    assert_eq!(name, "home");
}

#[test]
fn test_parse_name_without_set() {
    let icon = IconComponent::new("home");
    let (set, name) = icon.parse_name();
    assert_eq!(set, "lucide");
    assert_eq!(name, "home");
}

#[test]
fn test_set_and_icon_name() {
    let icon = IconComponent::new("mdi:account");
    assert_eq!(icon.set(), "mdi");
    assert_eq!(icon.icon_name(), "account");
}

#[test]
fn test_parse_icon_components_basic() {
    let source = r#"<dx-icon name="heroicons:home" />"#;
    let icons = parse_icon_components(source);
    assert_eq!(icons.len(), 1);
    assert_eq!(icons[0].name, "heroicons:home");
}

#[test]
fn test_parse_icon_components_with_size() {
    let source = r#"<dx-icon name="mdi:star" size="32" />"#;
    let icons = parse_icon_components(source);
    assert_eq!(icons.len(), 1);
    assert_eq!(icons[0].name, "mdi:star");
    assert_eq!(icons[0].size, 32);
}

#[test]
fn test_parse_icon_components_with_color() {
    let source = r#"<dx-icon name="lucide:heart" color="red" />"#;
    let icons = parse_icon_components(source);
    assert_eq!(icons.len(), 1);
    assert_eq!(icons[0].color, Some("red".to_string()));
}

#[test]
fn test_parse_icon_components_with_class() {
    let source = r#"<dx-icon name="heroicons:menu" class="nav-icon" />"#;
    let icons = parse_icon_components(source);
    assert_eq!(icons.len(), 1);
    assert_eq!(icons[0].class, Some("nav-icon".to_string()));
}

#[test]
fn test_parse_icon_components_multiple() {
    let source = concat!(
        r#"<dx-icon name="heroicons:home" />"#,
        "\n",
        r#"<dx-icon name="mdi:star" size="32" />"#,
        "\n",
        r#"<dx-icon name="lucide:heart" color="red" />"#
    );
    let icons = parse_icon_components(source);
    assert_eq!(icons.len(), 3);
}

#[test]
fn test_parse_icon_components_deduplication() {
    let source = concat!(
        r#"<dx-icon name="heroicons:home" />"#,
        "\n",
        r#"<dx-icon name="heroicons:home" />"#,
        "\n",
        r#"<dx-icon name="mdi:star" />"#
    );
    let icons = parse_icon_components(source);
    assert_eq!(icons.len(), 2);
}

#[test]
fn test_extract_icon_names() {
    let source =
        concat!(r#"<dx-icon name="heroicons:home" />"#, "\n", r#"<dx-icon name="mdi:star" />"#);
    let names = extract_icon_names(source);
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"heroicons:home".to_string()));
    assert!(names.contains(&"mdi:star".to_string()));
}

#[test]
fn test_extract_icons_by_set() {
    let source = concat!(
        r#"<dx-icon name="heroicons:home" />"#,
        "\n",
        r#"<dx-icon name="heroicons:user" />"#,
        "\n",
        r#"<dx-icon name="mdi:star" />"#
    );
    let by_set = extract_icons_by_set(source);
    assert_eq!(by_set.len(), 2);
    assert_eq!(by_set.get("heroicons").unwrap().len(), 2);
    assert_eq!(by_set.get("mdi").unwrap().len(), 1);
}

#[test]
fn test_icon_macro_basic() {
    let icon = icon!("heroicons:home");
    assert_eq!(icon.name, "heroicons:home");
    assert_eq!(icon.size, 24);
}

#[test]
fn test_icon_macro_with_size() {
    let icon = icon!("mdi:star", size = 32);
    assert_eq!(icon.name, "mdi:star");
    assert_eq!(icon.size, 32);
}

#[test]
fn test_icon_macro_with_color() {
    let icon = icon!("lucide:heart", color = "red");
    assert_eq!(icon.name, "lucide:heart");
    assert_eq!(icon.color, Some("red".to_string()));
}

#[test]
fn test_icon_macro_with_class() {
    let icon = icon!("heroicons:menu", class = "nav-icon");
    assert_eq!(icon.name, "heroicons:menu");
    assert_eq!(icon.class, Some("nav-icon".to_string()));
}
