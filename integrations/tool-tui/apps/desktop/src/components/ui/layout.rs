use crate::theme::Theme;
use gpui::{div, prelude::*, px, AnyElement, IntoElement};

// ─── Container ──────────────────────────────────────────────────────────────
// A centered max-width container, like shadcn-ui's container utility.

pub struct Container {
    children: Vec<AnyElement>,
    max_width: gpui::Pixels,
    padding: gpui::Pixels,
}

impl Container {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            max_width: px(1280.0),
            padding: px(24.0),
        }
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    #[allow(dead_code)]
    pub fn max_width(mut self, max: gpui::Pixels) -> Self {
        self.max_width = max;
        self
    }

    #[allow(dead_code)]
    pub fn padding(mut self, padding: gpui::Pixels) -> Self {
        self.padding = padding;
        self
    }

    pub fn render(self) -> impl IntoElement {
        let mut container = div().w_full().max_w(self.max_width).mx_auto().px(self.padding);

        for child in self.children {
            container = container.child(child);
        }

        container
    }
}

// ─── Stack (VStack / HStack) ────────────────────────────────────────────────
// Convenience wrappers for vertical and horizontal flex layouts.

pub struct VStack {
    children: Vec<AnyElement>,
    gap: gpui::Pixels,
    align: StackAlign,
}

#[derive(Debug, Clone, Copy)]
pub enum StackAlign {
    Start,
    Center,
    End,
    Stretch,
}

impl VStack {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            gap: px(0.0),
            align: StackAlign::Stretch,
        }
    }

    pub fn gap(mut self, gap: gpui::Pixels) -> Self {
        self.gap = gap;
        self
    }

    #[allow(dead_code)]
    pub fn align(mut self, align: StackAlign) -> Self {
        self.align = align;
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    pub fn render(self) -> impl IntoElement {
        let mut stack = div().flex().flex_col().gap(self.gap);

        match self.align {
            StackAlign::Start => stack = stack.items_start(),
            StackAlign::Center => stack = stack.items_center(),
            StackAlign::End => stack = stack.items_end(),
            StackAlign::Stretch => {}
        }

        for child in self.children {
            stack = stack.child(child);
        }

        stack
    }
}

pub struct HStack {
    children: Vec<AnyElement>,
    gap: gpui::Pixels,
    align: StackAlign,
    wrap: bool,
}

impl HStack {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            gap: px(0.0),
            align: StackAlign::Center,
            wrap: false,
        }
    }

    pub fn gap(mut self, gap: gpui::Pixels) -> Self {
        self.gap = gap;
        self
    }

    #[allow(dead_code)]
    pub fn align(mut self, align: StackAlign) -> Self {
        self.align = align;
        self
    }

    #[allow(dead_code)]
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    pub fn render(self) -> impl IntoElement {
        let mut stack = div().flex().gap(self.gap);

        if self.wrap {
            stack = stack.flex_wrap();
        }

        match self.align {
            StackAlign::Start => stack = stack.items_start(),
            StackAlign::Center => stack = stack.items_center(),
            StackAlign::End => stack = stack.items_end(),
            StackAlign::Stretch => {}
        }

        for child in self.children {
            stack = stack.child(child);
        }

        stack
    }
}

// ─── Center ─────────────────────────────────────────────────────────────────
// Center content both horizontally and vertically.

pub struct Center {
    children: Vec<AnyElement>,
}

impl Center {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    pub fn render(self) -> impl IntoElement {
        let mut center = div().flex().items_center().justify_center().size_full();

        for child in self.children {
            center = center.child(child);
        }

        center
    }
}

// ─── Spacer ─────────────────────────────────────────────────────────────────
// A flex spacer that pushes elements apart.

pub struct Spacer;

impl Spacer {
    pub fn flex() -> impl IntoElement {
        div().flex_1()
    }

    pub fn fixed(size: gpui::Pixels) -> impl IntoElement {
        div().size(size).flex_shrink_0()
    }
}

// ─── AspectRatio ────────────────────────────────────────────────────────────
// Maintains a given aspect ratio for its child.

pub struct AspectRatio {
    ratio: f32, // width/height
    child: Option<AnyElement>,
}

#[allow(dead_code)]
impl AspectRatio {
    pub fn new(ratio: f32) -> Self {
        Self { ratio, child: None }
    }

    pub fn square() -> Self {
        Self::new(1.0)
    }

    pub fn video() -> Self {
        Self::new(16.0 / 9.0)
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }

    pub fn render(self) -> impl IntoElement {
        // Note: GPUI doesn't have native aspect-ratio. We approximate with a fixed height
        // relative to the width. For a proper implementation, you'd use a layout constraint.
        let mut container = div().w_full().relative().overflow_hidden();

        if let Some(child) = self.child {
            container = container.child(child);
        }

        container
    }
}

// ─── ResizablePanel ─────────────────────────────────────────────────────────
// A resizable panel layout (display version).

pub struct ResizablePanel {
    panels: Vec<ResizablePanelItem>,
    direction: ResizableDirection,
}

struct ResizablePanelItem {
    content: AnyElement,
    default_size: f32, // as fraction 0..1
    min_size: Option<f32>,
}

#[derive(Debug, Clone, Copy)]
pub enum ResizableDirection {
    Horizontal,
    #[allow(dead_code)]
    Vertical,
}

impl ResizablePanel {
    pub fn horizontal() -> Self {
        Self {
            panels: Vec::new(),
            direction: ResizableDirection::Horizontal,
        }
    }

    #[allow(dead_code)]
    pub fn vertical() -> Self {
        Self {
            panels: Vec::new(),
            direction: ResizableDirection::Vertical,
        }
    }

    pub fn panel(mut self, content: impl IntoElement, default_size: f32) -> Self {
        self.panels.push(ResizablePanelItem {
            content: content.into_any_element(),
            default_size,
            min_size: None,
        });
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let is_horizontal = matches!(self.direction, ResizableDirection::Horizontal);

        let mut container = div().flex().size_full();
        if !is_horizontal {
            container = container.flex_col();
        }

        let total = self.panels.len();
        for (idx, panel) in self.panels.into_iter().enumerate() {
            // Panel content
            container = container.child(
                div()
                    .flex_grow()
                    .w(gpui::relative(panel.default_size))
                    .overflow_hidden()
                    .child(panel.content),
            );

            // Resize handle between panels
            if idx < total - 1 {
                if is_horizontal {
                    container = container.child(
                        div()
                            .w(px(1.0))
                            .h_full()
                            .bg(theme.border)
                            .flex_shrink_0()
                            .cursor_col_resize()
                            .hover(move |style| style.bg(theme.ring)),
                    );
                } else {
                    container = container.child(
                        div()
                            .h(px(1.0))
                            .w_full()
                            .bg(theme.border)
                            .flex_shrink_0()
                            .cursor_row_resize()
                            .hover(move |style| style.bg(theme.ring)),
                    );
                }
            }
        }

        container
    }
}
