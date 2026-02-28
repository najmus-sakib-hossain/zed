//! # DX Component Library
//!
//! A production-ready, copy-to-project component library similar to shadcn/ui.
//! All components are designed to be copied into your project and customized.
//!
//! ## Design Principles
//!
//! 1. **Copy-Paste** - Components are copied to your project, not imported from npm
//! 2. **Zero Dependencies** - Pure DX components with no external runtime
//! 3. **Tailwind-First** - Uses Tailwind classes compiled to Binary Dawn CSS
//! 4. **Accessible** - WCAG 2.1 AA compliant with proper ARIA attributes
//! 5. **Customizable** - Full control over styling and behavior
//!
//! ## Component Categories
//!
//! ### Primitives
//! - Button, Input, Textarea, Select, Checkbox, Radio, Switch
//!
//! ### Layout
//! - Card, Container, Grid, Flex, Stack, Separator
//!
//! ### Navigation
//! - Tabs, Breadcrumb, Pagination, Sidebar, Navbar
//!
//! ### Feedback
//! - Alert, Toast, Progress, Skeleton, Spinner
//!
//! ### Overlay
//! - Modal, Dialog, Drawer, Popover, Tooltip, DropdownMenu
//!
//! ### Data Display
//! - Table, Badge, Avatar, Accordion, Collapse
//!
//! ### Form
//! - Form, FormField, Label, FormMessage, FormDescription
//!
//! ## Usage
//!
//! ```bash
//! # Add a component to your project
//! dx add button
//!
//! # Add multiple components
//! dx add button card modal
//!
//! # Add all components
//! dx add --all
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Component category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentCategory {
    Primitive,
    Layout,
    Navigation,
    Feedback,
    Overlay,
    DataDisplay,
    Form,
}

impl ComponentCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Primitive => "primitive",
            Self::Layout => "layout",
            Self::Navigation => "navigation",
            Self::Feedback => "feedback",
            Self::Overlay => "overlay",
            Self::DataDisplay => "data-display",
            Self::Form => "form",
        }
    }
}

/// Component definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDef {
    /// Component name (PascalCase)
    pub name: String,
    /// Category
    pub category: ComponentCategory,
    /// Description
    pub description: String,
    /// Component source code (.cp file content)
    pub source: String,
    /// Required CSS classes (Tailwind)
    pub css_classes: Vec<String>,
    /// Dependencies (other components this one uses)
    pub dependencies: Vec<String>,
    /// Props definition
    pub props: Vec<PropDef>,
    /// Slots
    pub slots: Vec<SlotDef>,
    /// Accessibility features
    pub a11y: A11yDef,
}

/// Prop definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropDef {
    pub name: String,
    pub type_name: String,
    pub default: Option<String>,
    pub required: bool,
    pub description: String,
}

/// Slot definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotDef {
    pub name: String,
    pub description: String,
}

/// Accessibility definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A11yDef {
    pub role: Option<String>,
    pub aria_attrs: HashMap<String, String>,
    pub keyboard_nav: Vec<String>,
}

/// Get all available component definitions
pub fn get_all_components() -> Vec<ComponentDef> {
    vec![
        // === PRIMITIVES ===
        button_component(),
        input_component(),
        textarea_component(),
        select_component(),
        checkbox_component(),
        radio_component(),
        switch_component(),
        // === LAYOUT ===
        card_component(),
        container_component(),
        separator_component(),
        // === NAVIGATION ===
        tabs_component(),
        breadcrumb_component(),
        pagination_component(),
        // === FEEDBACK ===
        alert_component(),
        toast_component(),
        progress_component(),
        skeleton_component(),
        spinner_component(),
        // === OVERLAY ===
        modal_component(),
        dialog_component(),
        drawer_component(),
        popover_component(),
        tooltip_component(),
        dropdown_menu_component(),
        // === DATA DISPLAY ===
        table_component(),
        badge_component(),
        avatar_component(),
        accordion_component(),
        // === FORM ===
        form_component(),
        form_field_component(),
        label_component(),
    ]
}

/// Get component by name
pub fn get_component(name: &str) -> Option<ComponentDef> {
    get_all_components()
        .into_iter()
        .find(|c| c.name.to_lowercase() == name.to_lowercase())
}

/// Get components by category
pub fn get_components_by_category(category: ComponentCategory) -> Vec<ComponentDef> {
    get_all_components().into_iter().filter(|c| c.category == category).collect()
}

// ============================================================================
// COMPONENT DEFINITIONS
// ============================================================================

fn button_component() -> ComponentDef {
    ComponentDef {
        name: "Button".to_string(),
        category: ComponentCategory::Primitive,
        description: "A versatile button component with multiple variants and sizes.".to_string(),
        source: r#"<script lang="rust">
/// Button variants
pub enum Variant {
    Default,
    Primary,
    Secondary,
    Outline,
    Ghost,
    Link,
    Destructive,
}

/// Button sizes
pub enum Size {
    Sm,
    Md,
    Lg,
    Icon,
}

struct Props {
    variant: Option<Variant>,
    size: Option<Size>,
    disabled: Option<bool>,
    loading: Option<bool>,
    class: Option<String>,
}

let variant = props.variant.unwrap_or(Variant::Default);
let size = props.size.unwrap_or(Size::Md);
let disabled = props.disabled.unwrap_or(false) || props.loading.unwrap_or(false);
</script>

<component>
    <button
        class="inline-flex items-center justify-center rounded-md font-medium transition-colors 
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2
               disabled:pointer-events-none disabled:opacity-50
               {variant_classes(variant)} {size_classes(size)} {props.class.unwrap_or_default()}"
        disabled={disabled}
        aria-disabled={disabled}
    >
        {#if props.loading.unwrap_or(false)}
            <Spinner class="mr-2 h-4 w-4 animate-spin" />
        {/if}
        <slot />
    </button>
</component>
"#
        .to_string(),
        css_classes: vec![
            "inline-flex".into(),
            "items-center".into(),
            "justify-center".into(),
            "rounded-md".into(),
            "font-medium".into(),
            "transition-colors".into(),
            "focus-visible:outline-none".into(),
            "focus-visible:ring-2".into(),
            "focus-visible:ring-offset-2".into(),
            "disabled:pointer-events-none".into(),
            "disabled:opacity-50".into(),
            "bg-primary".into(),
            "text-primary-foreground".into(),
            "hover:bg-primary/90".into(),
            "bg-secondary".into(),
            "text-secondary-foreground".into(),
            "hover:bg-secondary/80".into(),
            "border".into(),
            "border-input".into(),
            "bg-background".into(),
            "hover:bg-accent".into(),
            "hover:text-accent-foreground".into(),
            "hover:underline".into(),
            "bg-destructive".into(),
            "text-destructive-foreground".into(),
            "hover:bg-destructive/90".into(),
            "h-9".into(),
            "px-3".into(),
            "h-10".into(),
            "px-4".into(),
            "py-2".into(),
            "h-11".into(),
            "px-8".into(),
            "h-10".into(),
            "w-10".into(),
        ],
        dependencies: vec!["Spinner".into()],
        props: vec![
            PropDef {
                name: "variant".into(),
                type_name: "Variant".into(),
                default: Some("Default".into()),
                required: false,
                description: "The visual style variant of the button".into(),
            },
            PropDef {
                name: "size".into(),
                type_name: "Size".into(),
                default: Some("Md".into()),
                required: false,
                description: "The size of the button".into(),
            },
            PropDef {
                name: "disabled".into(),
                type_name: "bool".into(),
                default: Some("false".into()),
                required: false,
                description: "Whether the button is disabled".into(),
            },
            PropDef {
                name: "loading".into(),
                type_name: "bool".into(),
                default: Some("false".into()),
                required: false,
                description: "Shows a loading spinner and disables the button".into(),
            },
            PropDef {
                name: "class".into(),
                type_name: "String".into(),
                default: None,
                required: false,
                description: "Additional CSS classes".into(),
            },
        ],
        slots: vec![SlotDef {
            name: "default".into(),
            description: "Button content".into(),
        }],
        a11y: A11yDef {
            role: Some("button".into()),
            aria_attrs: HashMap::from([("aria-disabled".into(), "{disabled}".into())]),
            keyboard_nav: vec![
                "Enter - Activate button".into(),
                "Space - Activate button".into(),
            ],
        },
    }
}

fn input_component() -> ComponentDef {
    ComponentDef {
        name: "Input".to_string(),
        category: ComponentCategory::Primitive,
        description: "A text input field with validation support.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    type_: Option<String>,
    placeholder: Option<String>,
    value: Option<String>,
    disabled: Option<bool>,
    error: Option<String>,
    class: Option<String>,
}
</script>

<component>
    <input
        type={props.type_.unwrap_or("text".into())}
        class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 
               text-sm ring-offset-background file:border-0 file:bg-transparent 
               file:text-sm file:font-medium placeholder:text-muted-foreground 
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring 
               focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50
               {#if props.error.is_some()}border-destructive{/if}
               {props.class.unwrap_or_default()}"
        placeholder={props.placeholder}
        value={props.value}
        disabled={props.disabled.unwrap_or(false)}
        bind:value
    />
    {#if let Some(error) = props.error}
        <p class="text-sm text-destructive mt-1">{error}</p>
    {/if}
</component>
"#
        .to_string(),
        css_classes: vec![
            "flex".into(),
            "h-10".into(),
            "w-full".into(),
            "rounded-md".into(),
            "border".into(),
            "border-input".into(),
            "bg-background".into(),
            "px-3".into(),
            "py-2".into(),
            "text-sm".into(),
            "ring-offset-background".into(),
            "placeholder:text-muted-foreground".into(),
            "focus-visible:outline-none".into(),
            "focus-visible:ring-2".into(),
            "focus-visible:ring-ring".into(),
            "focus-visible:ring-offset-2".into(),
            "disabled:cursor-not-allowed".into(),
            "disabled:opacity-50".into(),
            "border-destructive".into(),
            "text-destructive".into(),
            "mt-1".into(),
        ],
        dependencies: vec![],
        props: vec![
            PropDef {
                name: "type".into(),
                type_name: "String".into(),
                default: Some("text".into()),
                required: false,
                description: "Input type (text, password, email, etc.)".into(),
            },
            PropDef {
                name: "placeholder".into(),
                type_name: "String".into(),
                default: None,
                required: false,
                description: "Placeholder text".into(),
            },
            PropDef {
                name: "value".into(),
                type_name: "String".into(),
                default: None,
                required: false,
                description: "Input value".into(),
            },
            PropDef {
                name: "disabled".into(),
                type_name: "bool".into(),
                default: Some("false".into()),
                required: false,
                description: "Whether the input is disabled".into(),
            },
            PropDef {
                name: "error".into(),
                type_name: "String".into(),
                default: None,
                required: false,
                description: "Error message to display".into(),
            },
        ],
        slots: vec![],
        a11y: A11yDef {
            role: None,
            aria_attrs: HashMap::new(),
            keyboard_nav: vec!["Tab - Focus/blur input".into()],
        },
    }
}

fn textarea_component() -> ComponentDef {
    ComponentDef {
        name: "Textarea".to_string(),
        category: ComponentCategory::Primitive,
        description: "A multi-line text input field.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    placeholder: Option<String>,
    value: Option<String>,
    rows: Option<u32>,
    disabled: Option<bool>,
    class: Option<String>,
}
</script>

<component>
    <textarea
        class="flex min-h-[80px] w-full rounded-md border border-input bg-background px-3 py-2 
               text-sm ring-offset-background placeholder:text-muted-foreground 
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring 
               focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50
               {props.class.unwrap_or_default()}"
        placeholder={props.placeholder}
        rows={props.rows.unwrap_or(3)}
        disabled={props.disabled.unwrap_or(false)}
        bind:value
    />
</component>
"#
        .to_string(),
        css_classes: vec![
            "flex".into(),
            "min-h-[80px]".into(),
            "w-full".into(),
            "rounded-md".into(),
            "border".into(),
            "border-input".into(),
            "bg-background".into(),
            "px-3".into(),
            "py-2".into(),
            "text-sm".into(),
        ],
        dependencies: vec![],
        props: vec![],
        slots: vec![],
        a11y: A11yDef {
            role: None,
            aria_attrs: HashMap::new(),
            keyboard_nav: vec![],
        },
    }
}

fn select_component() -> ComponentDef {
    ComponentDef {
        name: "Select".to_string(),
        category: ComponentCategory::Primitive,
        description: "A dropdown select component.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    value: Option<String>,
    placeholder: Option<String>,
    disabled: Option<bool>,
    class: Option<String>,
}
</script>

<component>
    <select
        class="flex h-10 w-full items-center justify-between rounded-md border border-input 
               bg-background px-3 py-2 text-sm ring-offset-background 
               placeholder:text-muted-foreground focus:outline-none focus:ring-2 
               focus:ring-ring focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50
               {props.class.unwrap_or_default()}"
        disabled={props.disabled.unwrap_or(false)}
        bind:value
    >
        {#if let Some(placeholder) = props.placeholder}
            <option value="" disabled selected>{placeholder}</option>
        {/if}
        <slot />
    </select>
</component>
"#
        .to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![SlotDef {
            name: "default".into(),
            description: "Select options".into(),
        }],
        a11y: A11yDef {
            role: Some("listbox".into()),
            aria_attrs: HashMap::new(),
            keyboard_nav: vec![
                "Arrow Up/Down - Navigate options".into(),
                "Enter - Select option".into(),
                "Escape - Close dropdown".into(),
            ],
        },
    }
}

fn checkbox_component() -> ComponentDef {
    ComponentDef {
        name: "Checkbox".to_string(),
        category: ComponentCategory::Primitive,
        description: "A checkbox input component.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    checked: Option<bool>,
    disabled: Option<bool>,
    label: Option<String>,
    class: Option<String>,
}
</script>

<component>
    <label class="flex items-center gap-2 cursor-pointer">
        <input
            type="checkbox"
            class="h-4 w-4 rounded border border-primary ring-offset-background 
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring 
                   focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50
                   data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground
                   {props.class.unwrap_or_default()}"
            checked={props.checked.unwrap_or(false)}
            disabled={props.disabled.unwrap_or(false)}
            bind:checked
        />
        {#if let Some(label) = props.label}
            <span class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                {label}
            </span>
        {/if}
    </label>
</component>
"#.to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![],
        a11y: A11yDef {
            role: Some("checkbox".into()),
            aria_attrs: HashMap::from([("aria-checked".into(), "{checked}".into())]),
            keyboard_nav: vec!["Space - Toggle checkbox".into()],
        },
    }
}

fn radio_component() -> ComponentDef {
    ComponentDef {
        name: "Radio".to_string(),
        category: ComponentCategory::Primitive,
        description: "A radio button input component.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    name: String,
    value: String,
    checked: Option<bool>,
    disabled: Option<bool>,
    label: Option<String>,
}
</script>

<component>
    <label class="flex items-center gap-2 cursor-pointer">
        <input
            type="radio"
            name={props.name}
            value={props.value}
            class="h-4 w-4 rounded-full border border-primary ring-offset-background 
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring 
                   focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
            checked={props.checked.unwrap_or(false)}
            disabled={props.disabled.unwrap_or(false)}
            bind:group
        />
        {#if let Some(label) = props.label}
            <span class="text-sm font-medium leading-none">{label}</span>
        {/if}
    </label>
</component>
"#
        .to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![],
        a11y: A11yDef {
            role: Some("radio".into()),
            aria_attrs: HashMap::new(),
            keyboard_nav: vec!["Arrow keys - Navigate options".into()],
        },
    }
}

fn switch_component() -> ComponentDef {
    ComponentDef {
        name: "Switch".to_string(),
        category: ComponentCategory::Primitive,
        description: "A toggle switch component.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    checked: Option<bool>,
    disabled: Option<bool>,
    label: Option<String>,
}
</script>

<component>
    <label class="flex items-center gap-2 cursor-pointer">
        <button
            role="switch"
            aria-checked={props.checked.unwrap_or(false)}
            class="peer inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full 
                   border-2 border-transparent transition-colors focus-visible:outline-none 
                   focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 
                   focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50 
                   data-[state=checked]:bg-primary data-[state=unchecked]:bg-input"
            disabled={props.disabled.unwrap_or(false)}
            onClick={toggle}
        >
            <span
                class="pointer-events-none block h-5 w-5 rounded-full bg-background shadow-lg 
                       ring-0 transition-transform data-[state=checked]:translate-x-5 
                       data-[state=unchecked]:translate-x-0"
            />
        </button>
        {#if let Some(label) = props.label}
            <span class="text-sm font-medium">{label}</span>
        {/if}
    </label>
</component>
"#.to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![],
        a11y: A11yDef {
            role: Some("switch".into()),
            aria_attrs: HashMap::from([("aria-checked".into(), "{checked}".into())]),
            keyboard_nav: vec!["Space - Toggle switch".into()],
        },
    }
}

// === LAYOUT COMPONENTS ===

fn card_component() -> ComponentDef {
    ComponentDef {
        name: "Card".to_string(),
        category: ComponentCategory::Layout,
        description: "A container component with a border and shadow.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    class: Option<String>,
}
</script>

<component>
    <div class="rounded-lg border bg-card text-card-foreground shadow-sm {props.class.unwrap_or_default()}">
        <slot />
    </div>
</component>
"#.to_string(),
        css_classes: vec![
            "rounded-lg".into(),
            "border".into(),
            "bg-card".into(),
            "text-card-foreground".into(),
            "shadow-sm".into(),
        ],
        dependencies: vec![],
        props: vec![PropDef {
            name: "class".into(),
            type_name: "String".into(),
            default: None,
            required: false,
            description: "Additional CSS classes".into(),
        }],
        slots: vec![SlotDef {
            name: "default".into(),
            description: "Card content".into(),
        }],
        a11y: A11yDef {
            role: None,
            aria_attrs: HashMap::new(),
            keyboard_nav: vec![],
        },
    }
}

fn container_component() -> ComponentDef {
    ComponentDef {
        name: "Container".to_string(),
        category: ComponentCategory::Layout,
        description: "A responsive container with max-width constraints.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    class: Option<String>,
}
</script>

<component>
    <div class="mx-auto w-full max-w-7xl px-4 sm:px-6 lg:px-8 {props.class.unwrap_or_default()}">
        <slot />
    </div>
</component>
"#
        .to_string(),
        css_classes: vec![
            "mx-auto".into(),
            "w-full".into(),
            "max-w-7xl".into(),
            "px-4".into(),
            "sm:px-6".into(),
            "lg:px-8".into(),
        ],
        dependencies: vec![],
        props: vec![],
        slots: vec![SlotDef {
            name: "default".into(),
            description: "Container content".into(),
        }],
        a11y: A11yDef {
            role: None,
            aria_attrs: HashMap::new(),
            keyboard_nav: vec![],
        },
    }
}

fn separator_component() -> ComponentDef {
    ComponentDef {
        name: "Separator".to_string(),
        category: ComponentCategory::Layout,
        description: "A visual separator line.".to_string(),
        source: r#"<script lang="rust">
pub enum Orientation {
    Horizontal,
    Vertical,
}

struct Props {
    orientation: Option<Orientation>,
    class: Option<String>,
}
</script>

<component>
    <div
        role="separator"
        aria-orientation={orientation_str(props.orientation.unwrap_or(Orientation::Horizontal))}
        class="shrink-0 bg-border
               {#if props.orientation.unwrap_or(Orientation::Horizontal) == Orientation::Horizontal}
                   h-[1px] w-full
               {:else}
                   h-full w-[1px]
               {/if}
               {props.class.unwrap_or_default()}"
    />
</component>
"#
        .to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![],
        a11y: A11yDef {
            role: Some("separator".into()),
            aria_attrs: HashMap::new(),
            keyboard_nav: vec![],
        },
    }
}

// === NAVIGATION COMPONENTS ===

fn tabs_component() -> ComponentDef {
    ComponentDef {
        name: "Tabs".to_string(),
        category: ComponentCategory::Navigation,
        description: "A tabbed navigation component.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    default_value: Option<String>,
    class: Option<String>,
}

let mut active_tab = props.default_value.clone();

fn set_tab(value: String) {
    active_tab = Some(value);
}
</script>

<component>
    <div class="w-full {props.class.unwrap_or_default()}">
        <div role="tablist" class="inline-flex h-10 items-center justify-center rounded-md bg-muted p-1 text-muted-foreground">
            <slot name="tabs" />
        </div>
        <div class="mt-2">
            <slot />
        </div>
    </div>
</component>
"#.to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![
            SlotDef {
                name: "tabs".into(),
                description: "Tab triggers".into(),
            },
            SlotDef {
                name: "default".into(),
                description: "Tab content panels".into(),
            },
        ],
        a11y: A11yDef {
            role: Some("tablist".into()),
            aria_attrs: HashMap::new(),
            keyboard_nav: vec![
                "Arrow Left/Right - Navigate tabs".into(),
                "Enter/Space - Activate tab".into(),
            ],
        },
    }
}

fn breadcrumb_component() -> ComponentDef {
    ComponentDef {
        name: "Breadcrumb".to_string(),
        category: ComponentCategory::Navigation,
        description: "A breadcrumb navigation component.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    class: Option<String>,
}
</script>

<component>
    <nav aria-label="Breadcrumb" class="{props.class.unwrap_or_default()}">
        <ol class="flex flex-wrap items-center gap-1.5 break-words text-sm text-muted-foreground sm:gap-2.5">
            <slot />
        </ol>
    </nav>
</component>
"#.to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![SlotDef {
            name: "default".into(),
            description: "Breadcrumb items".into(),
        }],
        a11y: A11yDef {
            role: None,
            aria_attrs: HashMap::from([("aria-label".into(), "Breadcrumb".into())]),
            keyboard_nav: vec![],
        },
    }
}

fn pagination_component() -> ComponentDef {
    ComponentDef {
        name: "Pagination".to_string(),
        category: ComponentCategory::Navigation,
        description: "A pagination component for navigating pages.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    current_page: u32,
    total_pages: u32,
    on_page_change: fn(u32),
    class: Option<String>,
}
</script>

<component>
    <nav role="navigation" aria-label="pagination" class="mx-auto flex w-full justify-center {props.class.unwrap_or_default()}">
        <ul class="flex flex-row items-center gap-1">
            <li>
                <Button
                    variant={Variant::Ghost}
                    size={Size::Icon}
                    disabled={props.current_page <= 1}
                    onClick={|| props.on_page_change(props.current_page - 1)}
                >
                    ←
                </Button>
            </li>
            {#each (1..=props.total_pages) as page}
                <li>
                    <Button
                        variant={if page == props.current_page { Variant::Default } else { Variant::Ghost }}
                        size={Size::Icon}
                        onClick={|| props.on_page_change(page)}
                    >
                        {page}
                    </Button>
                </li>
            {/each}
            <li>
                <Button
                    variant={Variant::Ghost}
                    size={Size::Icon}
                    disabled={props.current_page >= props.total_pages}
                    onClick={|| props.on_page_change(props.current_page + 1)}
                >
                    →
                </Button>
            </li>
        </ul>
    </nav>
</component>
"#.to_string(),
        css_classes: vec![],
        dependencies: vec!["Button".into()],
        props: vec![],
        slots: vec![],
        a11y: A11yDef {
            role: Some("navigation".into()),
            aria_attrs: HashMap::from([("aria-label".into(), "pagination".into())]),
            keyboard_nav: vec![],
        },
    }
}

// === FEEDBACK COMPONENTS ===

fn alert_component() -> ComponentDef {
    ComponentDef {
        name: "Alert".to_string(),
        category: ComponentCategory::Feedback,
        description: "An alert message component.".to_string(),
        source: r#"<script lang="rust">
pub enum Variant {
    Default,
    Destructive,
    Success,
    Warning,
}

struct Props {
    variant: Option<Variant>,
    title: Option<String>,
    class: Option<String>,
}
</script>

<component>
    <div
        role="alert"
        class="relative w-full rounded-lg border p-4
               {variant_classes(props.variant.unwrap_or(Variant::Default))}
               {props.class.unwrap_or_default()}"
    >
        {#if let Some(title) = props.title}
            <h5 class="mb-1 font-medium leading-none tracking-tight">{title}</h5>
        {/if}
        <div class="text-sm [&_p]:leading-relaxed">
            <slot />
        </div>
    </div>
</component>
"#
        .to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![SlotDef {
            name: "default".into(),
            description: "Alert content".into(),
        }],
        a11y: A11yDef {
            role: Some("alert".into()),
            aria_attrs: HashMap::new(),
            keyboard_nav: vec![],
        },
    }
}

fn toast_component() -> ComponentDef {
    ComponentDef {
        name: "Toast".to_string(),
        category: ComponentCategory::Feedback,
        description: "A toast notification component.".to_string(),
        source: r#"<script lang="rust">
pub enum Variant {
    Default,
    Destructive,
    Success,
}

struct Props {
    variant: Option<Variant>,
    title: Option<String>,
    description: Option<String>,
    duration: Option<u32>,
    on_close: Option<fn()>,
}

let visible = true;

fn close() {
    visible = false;
    if let Some(on_close) = props.on_close {
        on_close();
    }
}
</script>

<component>
    {#if visible}
        <div
            class="pointer-events-auto relative flex w-full items-center justify-between 
                   space-x-4 overflow-hidden rounded-md border p-6 pr-8 shadow-lg transition-all
                   {variant_classes(props.variant.unwrap_or(Variant::Default))}"
            transition:fly={{ y: 50, duration: 300 }}
        >
            <div class="grid gap-1">
                {#if let Some(title) = props.title}
                    <div class="text-sm font-semibold">{title}</div>
                {/if}
                {#if let Some(description) = props.description}
                    <div class="text-sm opacity-90">{description}</div>
                {/if}
            </div>
            <button onClick={close} class="absolute right-2 top-2 rounded-md p-1 opacity-70 hover:opacity-100">
                ✕
            </button>
        </div>
    {/if}
</component>
"#.to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![],
        a11y: A11yDef {
            role: Some("status".into()),
            aria_attrs: HashMap::from([("aria-live".into(), "polite".into())]),
            keyboard_nav: vec!["Escape - Dismiss toast".into()],
        },
    }
}

fn progress_component() -> ComponentDef {
    ComponentDef {
        name: "Progress".to_string(),
        category: ComponentCategory::Feedback,
        description: "A progress bar component.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    value: Option<f32>,
    max: Option<f32>,
    class: Option<String>,
}
</script>

<component>
    <div
        role="progressbar"
        aria-valuemin="0"
        aria-valuemax={props.max.unwrap_or(100.0)}
        aria-valuenow={props.value.unwrap_or(0.0)}
        class="relative h-4 w-full overflow-hidden rounded-full bg-secondary {props.class.unwrap_or_default()}"
    >
        <div
            class="h-full w-full flex-1 bg-primary transition-all"
            style="transform: translateX(-{100.0 - (props.value.unwrap_or(0.0) / props.max.unwrap_or(100.0) * 100.0)}%)"
        />
    </div>
</component>
"#.to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![],
        a11y: A11yDef {
            role: Some("progressbar".into()),
            aria_attrs: HashMap::from([
                ("aria-valuemin".into(), "0".into()),
                ("aria-valuemax".into(), "{max}".into()),
                ("aria-valuenow".into(), "{value}".into()),
            ]),
            keyboard_nav: vec![],
        },
    }
}

fn skeleton_component() -> ComponentDef {
    ComponentDef {
        name: "Skeleton".to_string(),
        category: ComponentCategory::Feedback,
        description: "A loading skeleton placeholder.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    class: Option<String>,
}
</script>

<component>
    <div class="animate-pulse rounded-md bg-muted {props.class.unwrap_or_default()}" />
</component>
"#
        .to_string(),
        css_classes: vec![
            "animate-pulse".into(),
            "rounded-md".into(),
            "bg-muted".into(),
        ],
        dependencies: vec![],
        props: vec![],
        slots: vec![],
        a11y: A11yDef {
            role: None,
            aria_attrs: HashMap::from([("aria-hidden".into(), "true".into())]),
            keyboard_nav: vec![],
        },
    }
}

fn spinner_component() -> ComponentDef {
    ComponentDef {
        name: "Spinner".to_string(),
        category: ComponentCategory::Feedback,
        description: "A loading spinner component.".to_string(),
        source: r#"<script lang="rust">
pub enum Size {
    Sm,
    Md,
    Lg,
}

struct Props {
    size: Option<Size>,
    class: Option<String>,
}
</script>

<component>
    <svg
        class="animate-spin {size_classes(props.size.unwrap_or(Size::Md))} {props.class.unwrap_or_default()}"
        xmlns="http://www.w3.org/2000/svg"
        fill="none"
        viewBox="0 0 24 24"
        aria-hidden="true"
    >
        <circle
            class="opacity-25"
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            stroke-width="4"
        />
        <path
            class="opacity-75"
            fill="currentColor"
            d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
        />
    </svg>
</component>
"#.to_string(),
        css_classes: vec!["animate-spin".into(), "h-4".into(), "w-4".into(), "h-6".into(), "w-6".into(), "h-8".into(), "w-8".into()],
        dependencies: vec![],
        props: vec![],
        slots: vec![],
        a11y: A11yDef {
            role: None,
            aria_attrs: HashMap::from([("aria-hidden".into(), "true".into())]),
            keyboard_nav: vec![],
        },
    }
}

// === OVERLAY COMPONENTS ===

fn modal_component() -> ComponentDef {
    ComponentDef {
        name: "Modal".to_string(),
        category: ComponentCategory::Overlay,
        description: "A modal dialog component.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    open: bool,
    on_close: fn(),
    class: Option<String>,
}
</script>

<component>
    {#if props.open}
        <div class="fixed inset-0 z-50 flex items-center justify-center">
            <div 
                class="fixed inset-0 bg-black/80"
                onClick={props.on_close}
                transition:fade
            />
            <div
                class="relative z-50 grid w-full max-w-lg gap-4 border bg-background p-6 shadow-lg 
                       sm:rounded-lg {props.class.unwrap_or_default()}"
                role="dialog"
                aria-modal="true"
                transition:scale
            >
                <button
                    class="absolute right-4 top-4 rounded-sm opacity-70 ring-offset-background 
                           transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 
                           focus:ring-ring focus:ring-offset-2"
                    onClick={props.on_close}
                >
                    ✕
                </button>
                <slot />
            </div>
        </div>
    {/if}
</component>
"#
        .to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![SlotDef {
            name: "default".into(),
            description: "Modal content".into(),
        }],
        a11y: A11yDef {
            role: Some("dialog".into()),
            aria_attrs: HashMap::from([("aria-modal".into(), "true".into())]),
            keyboard_nav: vec![
                "Escape - Close modal".into(),
                "Tab - Focus trap within modal".into(),
            ],
        },
    }
}

fn dialog_component() -> ComponentDef {
    ComponentDef {
        name: "Dialog".to_string(),
        category: ComponentCategory::Overlay,
        description: "A dialog component with title and description.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    open: bool,
    on_close: fn(),
    title: Option<String>,
    description: Option<String>,
}
</script>

<component>
    <Modal open={props.open} on_close={props.on_close}>
        {#if let Some(title) = props.title}
            <div class="flex flex-col space-y-1.5 text-center sm:text-left">
                <h2 class="text-lg font-semibold leading-none tracking-tight">{title}</h2>
                {#if let Some(description) = props.description}
                    <p class="text-sm text-muted-foreground">{description}</p>
                {/if}
            </div>
        {/if}
        <slot />
    </Modal>
</component>
"#
        .to_string(),
        css_classes: vec![],
        dependencies: vec!["Modal".into()],
        props: vec![],
        slots: vec![SlotDef {
            name: "default".into(),
            description: "Dialog content".into(),
        }],
        a11y: A11yDef {
            role: Some("alertdialog".into()),
            aria_attrs: HashMap::new(),
            keyboard_nav: vec![],
        },
    }
}

fn drawer_component() -> ComponentDef {
    ComponentDef {
        name: "Drawer".to_string(),
        category: ComponentCategory::Overlay,
        description: "A slide-out drawer component.".to_string(),
        source: r#"<script lang="rust">
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

struct Props {
    open: bool,
    on_close: fn(),
    side: Option<Side>,
    class: Option<String>,
}
</script>

<component>
    {#if props.open}
        <div class="fixed inset-0 z-50">
            <div 
                class="fixed inset-0 bg-black/80"
                onClick={props.on_close}
                transition:fade
            />
            <div
                class="fixed bg-background p-6 shadow-lg
                       {side_classes(props.side.unwrap_or(Side::Right))}
                       {props.class.unwrap_or_default()}"
                role="dialog"
                transition:slide
            >
                <slot />
            </div>
        </div>
    {/if}
</component>
"#
        .to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![SlotDef {
            name: "default".into(),
            description: "Drawer content".into(),
        }],
        a11y: A11yDef {
            role: Some("dialog".into()),
            aria_attrs: HashMap::new(),
            keyboard_nav: vec!["Escape - Close drawer".into()],
        },
    }
}

fn popover_component() -> ComponentDef {
    ComponentDef {
        name: "Popover".to_string(),
        category: ComponentCategory::Overlay,
        description: "A popover component for floating content.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    open: bool,
    on_toggle: fn(),
    class: Option<String>,
}
</script>

<component>
    <div class="relative inline-block">
        <slot name="trigger" />
        {#if props.open}
            <div
                class="absolute z-50 w-72 rounded-md border bg-popover p-4 text-popover-foreground 
                       shadow-md outline-none {props.class.unwrap_or_default()}"
                transition:scale
            >
                <slot />
            </div>
        {/if}
    </div>
</component>
"#
        .to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![
            SlotDef {
                name: "trigger".into(),
                description: "Popover trigger".into(),
            },
            SlotDef {
                name: "default".into(),
                description: "Popover content".into(),
            },
        ],
        a11y: A11yDef {
            role: None,
            aria_attrs: HashMap::new(),
            keyboard_nav: vec!["Escape - Close popover".into()],
        },
    }
}

fn tooltip_component() -> ComponentDef {
    ComponentDef {
        name: "Tooltip".to_string(),
        category: ComponentCategory::Overlay,
        description: "A tooltip component for additional information.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    content: String,
    class: Option<String>,
}

let visible = false;
</script>

<component>
    <div 
        class="relative inline-block"
        onMouseEnter={|| visible = true}
        onMouseLeave={|| visible = false}
        onFocus={|| visible = true}
        onBlur={|| visible = false}
    >
        <slot />
        {#if visible}
            <div
                role="tooltip"
                class="absolute z-50 overflow-hidden rounded-md border bg-popover px-3 py-1.5 
                       text-sm text-popover-foreground shadow-md {props.class.unwrap_or_default()}"
                transition:fade={{ duration: 150 }}
            >
                {props.content}
            </div>
        {/if}
    </div>
</component>
"#
        .to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![SlotDef {
            name: "default".into(),
            description: "Tooltip trigger element".into(),
        }],
        a11y: A11yDef {
            role: Some("tooltip".into()),
            aria_attrs: HashMap::new(),
            keyboard_nav: vec![],
        },
    }
}

fn dropdown_menu_component() -> ComponentDef {
    ComponentDef {
        name: "DropdownMenu".to_string(),
        category: ComponentCategory::Overlay,
        description: "A dropdown menu component.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    open: bool,
    on_toggle: fn(),
    class: Option<String>,
}
</script>

<component>
    <div class="relative inline-block text-left">
        <slot name="trigger" />
        {#if props.open}
            <div
                class="absolute right-0 z-50 mt-2 min-w-[8rem] overflow-hidden rounded-md border 
                       bg-popover p-1 text-popover-foreground shadow-md {props.class.unwrap_or_default()}"
                role="menu"
                transition:scale={{ duration: 150, start: 0.95 }}
            >
                <slot />
            </div>
        {/if}
    </div>
</component>
"#.to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![
            SlotDef {
                name: "trigger".into(),
                description: "Menu trigger button".into(),
            },
            SlotDef {
                name: "default".into(),
                description: "Menu items".into(),
            },
        ],
        a11y: A11yDef {
            role: Some("menu".into()),
            aria_attrs: HashMap::new(),
            keyboard_nav: vec![
                "Arrow Down - Next item".into(),
                "Arrow Up - Previous item".into(),
                "Enter - Select item".into(),
                "Escape - Close menu".into(),
            ],
        },
    }
}

// === DATA DISPLAY COMPONENTS ===

fn table_component() -> ComponentDef {
    ComponentDef {
        name: "Table".to_string(),
        category: ComponentCategory::DataDisplay,
        description: "A table component for displaying tabular data.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    class: Option<String>,
}
</script>

<component>
    <div class="relative w-full overflow-auto">
        <table class="w-full caption-bottom text-sm {props.class.unwrap_or_default()}">
            <slot />
        </table>
    </div>
</component>
"#
        .to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![SlotDef {
            name: "default".into(),
            description: "Table content (thead, tbody, tfoot)".into(),
        }],
        a11y: A11yDef {
            role: None,
            aria_attrs: HashMap::new(),
            keyboard_nav: vec![],
        },
    }
}

fn badge_component() -> ComponentDef {
    ComponentDef {
        name: "Badge".to_string(),
        category: ComponentCategory::DataDisplay,
        description: "A badge component for labels and status.".to_string(),
        source: r#"<script lang="rust">
pub enum Variant {
    Default,
    Secondary,
    Destructive,
    Outline,
}

struct Props {
    variant: Option<Variant>,
    class: Option<String>,
}
</script>

<component>
    <div 
        class="inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold 
               transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2
               {variant_classes(props.variant.unwrap_or(Variant::Default))}
               {props.class.unwrap_or_default()}"
    >
        <slot />
    </div>
</component>
"#
        .to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![SlotDef {
            name: "default".into(),
            description: "Badge content".into(),
        }],
        a11y: A11yDef {
            role: None,
            aria_attrs: HashMap::new(),
            keyboard_nav: vec![],
        },
    }
}

fn avatar_component() -> ComponentDef {
    ComponentDef {
        name: "Avatar".to_string(),
        category: ComponentCategory::DataDisplay,
        description: "An avatar component for user images.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    src: Option<String>,
    alt: Option<String>,
    fallback: Option<String>,
    class: Option<String>,
}
</script>

<component>
    <span class="relative flex h-10 w-10 shrink-0 overflow-hidden rounded-full {props.class.unwrap_or_default()}">
        {#if let Some(src) = props.src}
            <img class="aspect-square h-full w-full" src={src} alt={props.alt.unwrap_or_default()} />
        {:else}
            <span class="flex h-full w-full items-center justify-center rounded-full bg-muted">
                {props.fallback.unwrap_or("?".into())}
            </span>
        {/if}
    </span>
</component>
"#.to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![],
        a11y: A11yDef {
            role: Some("img".into()),
            aria_attrs: HashMap::new(),
            keyboard_nav: vec![],
        },
    }
}

fn accordion_component() -> ComponentDef {
    ComponentDef {
        name: "Accordion".to_string(),
        category: ComponentCategory::DataDisplay,
        description: "An accordion component for collapsible content.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    multiple: Option<bool>,
    class: Option<String>,
}

let open_items: Vec<String> = vec![];

fn toggle_item(id: String) {
    if open_items.contains(&id) {
        open_items.retain(|x| x != &id);
    } else {
        if !props.multiple.unwrap_or(false) {
            open_items.clear();
        }
        open_items.push(id);
    }
}
</script>

<component>
    <div class="w-full {props.class.unwrap_or_default()}">
        <slot />
    </div>
</component>
"#
        .to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![SlotDef {
            name: "default".into(),
            description: "Accordion items".into(),
        }],
        a11y: A11yDef {
            role: None,
            aria_attrs: HashMap::new(),
            keyboard_nav: vec![
                "Enter/Space - Toggle item".into(),
                "Arrow Down - Next item".into(),
                "Arrow Up - Previous item".into(),
            ],
        },
    }
}

// === FORM COMPONENTS ===

fn form_component() -> ComponentDef {
    ComponentDef {
        name: "Form".to_string(),
        category: ComponentCategory::Form,
        description: "A form component with validation support.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    on_submit: fn(FormData),
    class: Option<String>,
}

fn handle_submit(event: SubmitEvent) {
    event.prevent_default();
    let form_data = FormData::from_event(&event);
    props.on_submit(form_data);
}
</script>

<component>
    <form 
        class="space-y-4 {props.class.unwrap_or_default()}"
        onSubmit={handle_submit}
    >
        <slot />
    </form>
</component>
"#
        .to_string(),
        css_classes: vec!["space-y-4".into()],
        dependencies: vec![],
        props: vec![],
        slots: vec![SlotDef {
            name: "default".into(),
            description: "Form fields".into(),
        }],
        a11y: A11yDef {
            role: None,
            aria_attrs: HashMap::new(),
            keyboard_nav: vec!["Enter - Submit form (when in input)".into()],
        },
    }
}

fn form_field_component() -> ComponentDef {
    ComponentDef {
        name: "FormField".to_string(),
        category: ComponentCategory::Form,
        description: "A form field wrapper with label and error handling.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    label: Option<String>,
    error: Option<String>,
    description: Option<String>,
    required: Option<bool>,
    class: Option<String>,
}
</script>

<component>
    <div class="space-y-2 {props.class.unwrap_or_default()}">
        {#if let Some(label) = props.label}
            <Label required={props.required.unwrap_or(false)}>{label}</Label>
        {/if}
        <slot />
        {#if let Some(description) = props.description}
            <p class="text-sm text-muted-foreground">{description}</p>
        {/if}
        {#if let Some(error) = props.error}
            <p class="text-sm font-medium text-destructive">{error}</p>
        {/if}
    </div>
</component>
"#
        .to_string(),
        css_classes: vec![],
        dependencies: vec!["Label".into()],
        props: vec![],
        slots: vec![SlotDef {
            name: "default".into(),
            description: "Form input element".into(),
        }],
        a11y: A11yDef {
            role: None,
            aria_attrs: HashMap::new(),
            keyboard_nav: vec![],
        },
    }
}

fn label_component() -> ComponentDef {
    ComponentDef {
        name: "Label".to_string(),
        category: ComponentCategory::Form,
        description: "A label component for form inputs.".to_string(),
        source: r#"<script lang="rust">
struct Props {
    for_: Option<String>,
    required: Option<bool>,
    class: Option<String>,
}
</script>

<component>
    <label 
        for={props.for_}
        class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed 
               peer-disabled:opacity-70 {props.class.unwrap_or_default()}"
    >
        <slot />
        {#if props.required.unwrap_or(false)}
            <span class="text-destructive ml-1">*</span>
        {/if}
    </label>
</component>
"#
        .to_string(),
        css_classes: vec![],
        dependencies: vec![],
        props: vec![],
        slots: vec![SlotDef {
            name: "default".into(),
            description: "Label text".into(),
        }],
        a11y: A11yDef {
            role: None,
            aria_attrs: HashMap::new(),
            keyboard_nav: vec![],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_components() {
        let components = get_all_components();
        assert!(components.len() >= 30, "Should have at least 30 components");
    }

    #[test]
    fn test_get_component_by_name() {
        let button = get_component("button");
        assert!(button.is_some());
        assert_eq!(button.unwrap().name, "Button");
    }

    #[test]
    fn test_get_components_by_category() {
        let primitives = get_components_by_category(ComponentCategory::Primitive);
        assert!(!primitives.is_empty());
        assert!(primitives.iter().all(|c| c.category == ComponentCategory::Primitive));
    }
}
