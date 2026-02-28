# GPUI Framework API Summary

> Comprehensive reference for building shadcn-ui style components in Rust/GPUI.  
> Source: GPUI v0.2.2 (Zed's GPU-accelerated UI framework)

---

## 1. Elements & Components Creation

### Architecture Overview
GPUI is a **hybrid immediate/retained mode** GPU-accelerated UI framework. It uses **Taffy v0.9.0** (CSS flexbox/grid layout engine) internally and renders via **Metal** (macOS) or **Blade** (cross-platform GPU).

Three core registers:
- **Entities** (`Entity<T>`) — State management (like `Rc` but requires `App` context)
- **Views** — Entities that implement `Render` (declarative UI)
- **Elements** — Imperative UI building blocks with a 3-phase lifecycle

### Application Startup
```rust
Application::new().run(|cx: &mut App| {
    cx.open_window(WindowOptions::default(), |window, cx| {
        cx.new(|cx| MyView { /* ... */ })
    });
});
```

### `Render` Trait — Stateful Views (Components with Identity)
```rust
pub trait Render: 'static + Sized {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement;
}
```
- Takes `&mut self` — has persistent state across frames
- Gets `Context<Self>` — entity-specific services (observe, subscribe, emit, notify)
- Views are created with `cx.new(|cx| MyView { ... })` returning `Entity<MyView>`
- Distinguish "views" from plain entities — only views can be root of windows

### `RenderOnce` Trait — Stateless Components (Consumed on Render)
```rust
pub trait RenderOnce: 'static {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement;
}
```
- Takes `self` (ownership) — consumed when rendered, no persistent state
- Gets `&mut App` — general app context, no entity-specific services
- Use `#[derive(IntoElement)]` on types implementing `RenderOnce`
- Ideal for reusable, pure presentational components (like shadcn-ui primitives)

### Creating a View Component
```rust
struct Counter {
    count: usize,
}

impl Render for Counter {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(format!("Count: {}", self.count))
            .child(
                div()
                    .id("increment")
                    .child("Click me")
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.count += 1;
                        cx.notify(); // triggers re-render
                    }))
            )
    }
}
```

### Creating a RenderOnce Component (shadcn-ui style)
```rust
#[derive(IntoElement)]
struct Button {
    label: SharedString,
    variant: ButtonVariant,
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id("button")
            .px_4()
            .py_2()
            .rounded_md()
            .cursor_pointer()
            .bg(match self.variant {
                ButtonVariant::Primary => hsla(0.6, 0.8, 0.5, 1.0),
                ButtonVariant::Secondary => hsla(0.0, 0.0, 0.9, 1.0),
            })
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .child(self.label)
    }
}
```

### Element Constructors (Functions)
| Function | Signature | Purpose |
|----------|-----------|---------|
| `div()` | `-> Div` | All-in-one element for complex UIs |
| `canvas(prepaint, paint)` | `-> Canvas<T>` | Custom drawing with prepaint/paint phases |
| `deferred(child)` | `-> Deferred` | Delays paint until after ancestors |
| `anchored()` | `-> Anchored` | Avoids overflowing window bounds |
| `svg()` | `-> Svg` | SVG rendering |
| `img()` | `-> Img` | Image rendering |
| `list()` | `-> List` | Virtualized list |
| `uniform_list()` | `-> UniformList` | Virtualized list with uniform item height |

---

## 2. Styling System

### `Styled` Trait (3,010 methods!)
```rust
pub trait Styled: Sized {
    // Required: access to style state
    fn style(&mut self) -> &mut StyleRefinement;
    
    // 3,009 provided Tailwind-style utility methods
}
```

**Implemented for:** `Div`, `Anchored`, `Img`, `Svg`, `Stateful<E>` where E: Styled

### `Style` Struct (38 fields)
The CSS styling applied to an element, representing the final resolved style:

```rust
pub struct Style {
    // Layout
    pub display: Display,              // block, flex, grid, hidden
    pub position: Position,            // relative, absolute
    pub visibility: Visibility,        // visible, hidden
    pub overflow: Point<Overflow>,     // x/y overflow behavior
    pub flex_direction: FlexDirection,  // row, column, row-reverse, column-reverse
    pub flex_wrap: FlexWrap,           // nowrap, wrap, wrap-reverse
    pub flex_basis: Length,            // initial main axis size
    pub flex_grow: f32,                // growth rate (default 0.0)
    pub flex_shrink: f32,              // shrink rate (default 1.0)
    pub align_items: Option<AlignItems>,
    pub align_self: Option<AlignSelf>,
    pub align_content: Option<AlignContent>,
    pub justify_content: Option<JustifyContent>,
    
    // Sizing
    pub size: Size<Length>,            // width/height
    pub min_size: Size<Length>,        // min-width/min-height
    pub max_size: Size<Length>,        // max-width/max-height
    pub aspect_ratio: Option<f32>,     // width/height ratio
    
    // Spacing
    pub margin: Edges<Length>,         // outer spacing
    pub padding: Edges<DefiniteLength>,// inner spacing
    pub gap: Size<DefiniteLength>,     // flex/grid gap
    pub inset: Edges<Length>,          // position offsets (top/right/bottom/left)
    
    // Visual
    pub background: Option<Fill>,      // solid color or gradient
    pub border_widths: Edges<AbsoluteLength>,
    pub border_color: Option<Hsla>,
    pub border_style: BorderStyle,
    pub corner_radii: Corners<AbsoluteLength>,
    pub box_shadow: Vec<BoxShadow>,
    pub opacity: Option<f32>,
    pub mouse_cursor: Option<CursorStyle>,
    
    // Text
    pub text: TextStyleRefinement,     // font, size, color, weight, etc.
    
    // Grid
    pub grid_cols: Option<u16>,
    pub grid_rows: Option<u16>,
    pub grid_location: Option<GridLocation>,
    
    // Scrolling
    pub scrollbar_width: AbsoluteLength,
    pub allow_concurrent_scroll: bool,
    pub restrict_scroll_to_axis: bool,
    
    // Debug
    pub debug: bool,
    pub debug_below: bool,
}
```

### StyleRefinement
Optional overlay on `Style` — used for hover, active, focus states:
```rust
impl Refineable for Style {
    type Refinement = StyleRefinement;
}
```

---

## 3. Complete Style Properties Reference

### Display & Visibility
| Method | CSS Equivalent |
|--------|---------------|
| `.block()` | `display: block` |
| `.flex()` | `display: flex` |
| `.grid()` | `display: grid` |
| `.hidden()` | `display: none` |
| `.visible()` | `visibility: visible` |
| `.invisible()` | `visibility: hidden` |

### Position
| Method | CSS Equivalent |
|--------|---------------|
| `.relative()` | `position: relative` |
| `.absolute()` | `position: absolute` |
| `.top_N()`, `.bottom_N()`, `.left_N()`, `.right_N()` | Position offsets |

### Flexbox
| Method | CSS Equivalent |
|--------|---------------|
| `.flex_row()` | `flex-direction: row` |
| `.flex_col()` | `flex-direction: column` |
| `.flex_row_reverse()` | `flex-direction: row-reverse` |
| `.flex_col_reverse()` | `flex-direction: column-reverse` |
| `.flex_wrap()` | `flex-wrap: wrap` |
| `.flex_wrap_reverse()` | `flex-wrap: wrap-reverse` |
| `.flex_nowrap()` | `flex-wrap: nowrap` |
| `.flex_1()` | `flex: 1 1 0%` |
| `.flex_auto()` | `flex: 1 1 auto` |
| `.flex_initial()` | `flex: 0 1 auto` |
| `.flex_none()` | `flex: none` |
| `.flex_grow()` | `flex-grow: 1` |
| `.flex_shrink()` | `flex-shrink: 1` |
| `.flex_shrink_0()` | `flex-shrink: 0` |
| `.flex_basis(impl Into<Length>)` | `flex-basis: <value>` |

### Alignment
| Method | CSS Equivalent |
|--------|---------------|
| `.items_start()` | `align-items: flex-start` |
| `.items_center()` | `align-items: center` |
| `.items_end()` | `align-items: flex-end` |
| `.items_baseline()` | `align-items: baseline` |
| `.justify_start()` | `justify-content: flex-start` |
| `.justify_center()` | `justify-content: center` |
| `.justify_end()` | `justify-content: flex-end` |
| `.justify_between()` | `justify-content: space-between` |
| `.justify_around()` | `justify-content: space-around` |
| `.content_start()` | `align-content: flex-start` |
| `.content_center()` | `align-content: center` |
| `.content_end()` | `align-content: flex-end` |
| `.content_between()` | `align-content: space-between` |
| `.content_around()` | `align-content: space-around` |
| `.content_evenly()` | `align-content: space-evenly` |
| `.content_stretch()` | `align-content: stretch` |

### Sizing (Tailwind Scale: 1 unit = 0.25rem)
Each has positive, negative, fraction, and special variants:
```
.w(length), .w_0() through .w_128(), .w_auto(), .w_px(), .w_full()
.w_1_2(), .w_1_3(), .w_2_3(), .w_1_4(), .w_3_4(), .w_1_5(), .w_1_6(), .w_5_6(), .w_1_12()
.h(length), .h_0() through .h_128(), .h_auto(), .h_px(), .h_full()
.size(length), .size_0() through .size_128(), .size_auto(), .size_full()
.min_w(length), .min_w_0() through .min_w_128()
.min_h(length), .min_h_0() through .min_h_128()
.max_w(length), .max_w_0() through .max_w_128()
.max_h(length), .max_h_0() through .max_h_128()
```

### Spacing (Padding & Margin)
```
// Padding - all same scale as sizing
.p(length), .p_0() through .p_128()
.px(length), .py(length)           // horizontal/vertical
.pt(), .pb(), .pl(), .pr()          // individual sides

// Margin - same pattern
.m(length), .m_0() through .m_128()
.mx(length), .my(length)
.mt(), .mb(), .ml(), .mr()
```

### Gap
```
.gap(length), .gap_0() through .gap_128()
.gap_x(length), .gap_y(length)
```

### Background & Colors
```rust
.bg(fill: impl Into<Fill>)  // accepts Hsla, Rgba, gradients
```

### Border
```
.border(length), .border_0() through .border_32()
.border_t(length), .border_b(length), .border_l(length), .border_r(length)
.border_x(length), .border_y(length)
.border_color(impl Into<Hsla>)
.border_dashed()
```

### Border Radius
| Method | Value |
|--------|-------|
| `.rounded_none()` | 0 |
| `.rounded_sm()` | 0.125rem |
| `.rounded_md()` | 0.375rem |
| `.rounded_lg()` | 0.5rem |
| `.rounded_xl()` | 0.75rem |
| `.rounded_2xl()` | 1rem |
| `.rounded_full()` | 9999px |
| Plus: `.rounded_t_*()`, `.rounded_b_*()`, `.rounded_l_*()`, `.rounded_r_*()` |

### Shadow
| Method | Description |
|--------|-------------|
| `.shadow_none()` | No shadow |
| `.shadow_2xs()` | Extra extra small |
| `.shadow_xs()` | Extra small |
| `.shadow_sm()` | Small |
| `.shadow_md()` | Medium |
| `.shadow_lg()` | Large |
| `.shadow_xl()` | Extra large |
| `.shadow_2xl()` | Extra extra large |
| `.shadow(Vec<BoxShadow>)` | Custom shadows |

### Overflow
| Method | Effect |
|--------|--------|
| `.overflow_hidden()` | Both axes |
| `.overflow_x_hidden()` | X axis only |
| `.overflow_y_hidden()` | Y axis only |

### Cursor
```
.cursor(CursorStyle), .cursor_default(), .cursor_pointer(), .cursor_text(),
.cursor_move(), .cursor_not_allowed(), .cursor_context_menu(), .cursor_crosshair(),
.cursor_vertical_text(), .cursor_alias(), .cursor_copy(), .cursor_no_drop(),
.cursor_grab(), .cursor_grabbing(), .cursor_ew_resize(), .cursor_ns_resize(),
.cursor_nesw_resize(), .cursor_nwse_resize(), .cursor_col_resize(), .cursor_row_resize(),
.cursor_n_resize(), .cursor_e_resize(), .cursor_s_resize(), .cursor_w_resize()
```

### Text Styling
| Method | Effect |
|--------|--------|
| `.text_color(impl Into<Hsla>)` | Text color |
| `.text_bg(impl Into<Hsla>)` | Text background |
| `.text_size(impl Into<AbsoluteLength>)` | Custom text size |
| `.text_xs()` | 0.75rem |
| `.text_sm()` | 0.875rem |
| `.text_base()` | 1rem |
| `.text_lg()` | 1.125rem |
| `.text_xl()` | 1.25rem |
| `.text_2xl()` | 1.5rem |
| `.text_3xl()` | 1.875rem |
| `.font_weight(FontWeight)` | Font weight |
| `.font_family(impl Into<SharedString>)` | Font family |
| `.font(Font)` | Full font specification |
| `.line_height(impl Into<DefiniteLength>)` | Line height |
| `.italic()` / `.not_italic()` | Italic toggle |
| `.underline()` | Underline decoration |
| `.line_through()` | Strikethrough |
| `.text_decoration_none()` | Remove decorations |
| `.text_decoration_color(impl Into<Hsla>)` | Decoration color |
| `.text_decoration_solid()` / `.text_decoration_wavy()` | Decoration style |
| `.text_decoration_0/1/2/4/8()` | Decoration thickness |
| `.whitespace_normal()` / `.whitespace_nowrap()` | Whitespace handling |
| `.text_ellipsis()` | Overflow ellipsis |
| `.truncate()` | nowrap + ellipsis combo |
| `.line_clamp(lines: usize)` | Clamp to N lines |
| `.text_left()` / `.text_center()` / `.text_right()` | Text alignment |
| `.text_overflow(TextOverflow)` | Custom overflow |
| `.text_align(TextAlign)` | Custom alignment |

### Grid
```
.grid_cols(cols: u16), .grid_rows(rows: u16)
.col_start(i16), .col_end(i16), .col_span(u16), .col_span_full()
.row_start(i16), .row_end(i16), .row_span(u16), .row_span_full()
.col_start_auto(), .col_end_auto(), .row_start_auto(), .row_end_auto()
```

### Opacity
```rust
.opacity(opacity: f32)  // 0.0 to 1.0
```

### Debug
```
.debug()       // Red outline around this element
.debug_below() // Red outline around this + all children
```

---

## 4. Event Handling

### `InteractiveElement` Trait (38 methods)
```rust
pub trait InteractiveElement: Sized {
    fn interactivity(&mut self) -> &mut Interactivity;
    
    // Identity & Grouping
    fn group(self, name: impl Into<SharedString>) -> Self;
    fn id(self, id: impl Into<ElementId>) -> Stateful<Self>;  // PROMOTES to Stateful!
    
    // Style States (take closure returning StyleRefinement)
    fn hover(self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self;
    fn group_hover(self, group_name: impl Into<SharedString>, f: ...) -> Self;
    fn focus(self, f: ...) -> Self;
    fn in_focus(self, f: ...) -> Self;
    
    // Mouse Events (listener: Fn(&Event, &mut Window, &mut App))
    fn on_mouse_down(self, button: MouseButton, listener: ...) -> Self;
    fn on_mouse_up(self, button: MouseButton, listener: ...) -> Self;
    fn on_mouse_move(self, listener: ...) -> Self;
    fn on_mouse_down_out(self, listener: ...) -> Self;
    fn on_mouse_up_out(self, button: MouseButton, listener: ...) -> Self;
    fn on_scroll_wheel(self, listener: ...) -> Self;
    fn on_drag_move::<T>(self, listener: ...) -> Self;
    
    // Capture Phase (fires before bubble)
    fn capture_any_mouse_down(self, listener: ...) -> Self;
    fn capture_any_mouse_up(self, listener: ...) -> Self;
    
    // Keyboard Events
    fn on_key_down(self, listener: ...) -> Self;
    fn on_key_up(self, listener: ...) -> Self;
    fn capture_key_down(self, listener: ...) -> Self;
    fn capture_key_up(self, listener: ...) -> Self;
    fn on_modifiers_changed(self, listener: ...) -> Self;
    
    // Actions (command pattern)
    fn on_action::<A: Action>(self, listener: ...) -> Self;
    fn capture_action::<A: Action>(self, listener: ...) -> Self;
    fn on_boxed_action(self, action: &dyn Action, listener: ...) -> Self;
    
    // Drag & Drop
    fn drag_over::<S: 'static>(self, f: ...) -> Self;
    fn group_drag_over::<S: 'static>(self, group_name: ..., f: ...) -> Self;
    fn on_drop::<T: 'static>(self, listener: ...) -> Self;
    fn can_drop(self, predicate: ...) -> Self;
    
    // Focus
    fn track_focus(self, handle: &FocusHandle) -> Self;
    fn key_context(self, ctx: impl TryInto<KeyContext>) -> Self;
    
    // Tab Navigation
    fn tab_stop(self, bool) -> Self;
    fn tab_index(self, isize) -> Self;
    fn tab_group(self) -> Self;
    
    // Other
    fn occlude(self) -> Self;          // block mouse behind
    fn block_mouse_except_scroll(self) -> Self;
}
```

### `StatefulInteractiveElement` Trait (14 methods)
Only available after calling `.id()` on an `InteractiveElement`:

```rust
pub trait StatefulInteractiveElement: InteractiveElement {
    // Focus
    fn focusable(self) -> Self;        // makes element focusable
    
    // Scrolling
    fn overflow_scroll(self) -> Self;
    fn overflow_x_scroll(self) -> Self;
    fn overflow_y_scroll(self) -> Self;
    fn scrollbar_width(self, width: impl Into<Pixels>) -> Self;
    fn track_scroll(self, handle: &ScrollHandle) -> Self;
    fn anchor_scroll(self, anchor: ScrollAnchor) -> Self;
    
    // State-dependent Styles
    fn active(self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self;
    fn group_active(self, name: impl Into<SharedString>, f: ...) -> Self;
    
    // Click (HIGH-LEVEL - preferred over mouse_down/up)
    fn on_click(self, listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self;
    
    // Drag
    fn on_drag(self, value: T, constructor: ...) -> Self;
    
    // Hover (boolean callback)
    fn on_hover(self, listener: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self;
    
    // Tooltips
    fn tooltip(self, build_tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self;
    fn hoverable_tooltip(self, build_tooltip: ...) -> Self;
}
```

### Event Listener Pattern
All event listeners follow this signature:
```rust
impl Fn(&EventType, &mut Window, &mut App) + 'static
```

### Important: `.id()` Promotes to Stateful
```rust
// This is an InteractiveElement (no state tracking):
div().hover(|s| s.bg(red()))

// After .id(), it becomes Stateful — unlocks on_click, active, tooltip, etc.:
div().id("my-button").on_click(|_event, _window, _cx| { /* ... */ })
```

---

## 5. State Management

### Entity System
```rust
// Create an entity
let entity: Entity<MyState> = cx.new(|cx| MyState { count: 0 });

// Read state (immutable)
let count = entity.read(cx).count;

// Update state (mutable)
entity.update(cx, |state, cx| {
    state.count += 1;
    cx.notify(); // trigger re-render of observing views
});
```

### Observation Pattern (Property Changes)
```rust
// In a View's constructor or method:
cx.observe(&other_entity, |this, observed, cx| {
    // Called whenever observed entity calls cx.notify()
    let value = observed.read(cx).some_field;
    this.cached_value = value;
    cx.notify(); // re-render self
});
```

### Event Subscription Pattern (Typed Events)
```rust
// Define events
struct MyEvent { message: String }
impl EventEmitter<MyEvent> for MyEmitter {}

// Emit
cx.emit(MyEvent { message: "hello".into() });

// Subscribe
cx.subscribe(&emitter_entity, |this, emitter, event: &MyEvent, cx| {
    println!("{}", event.message);
});
```

### `Context<T>` vs `App`
- `Context<T>` — Entity-specific context; wraps `App` plus the entity's handle. Available in `Render::render()` and entity methods. Has `.notify()`, `.emit()`, `.observe()`, `.subscribe()`.
- `App` — Global app context. Available everywhere. Has `.new()`, `.open_window()`, global state, etc.

### FluentBuilder Trait
Enables imperative conditionals in fluent chains:
```rust
div()
    .when(is_active, |el| el.bg(blue()))
    .when_some(maybe_color, |el, color| el.bg(color))
    .map(|el| {
        if condition { el.p_4() } else { el.p_2() }
    })
```

---

## 6. Custom Elements (Element Trait)

### Three-Phase Lifecycle
```rust
pub trait Element: 'static + IntoElement {
    type RequestLayoutState: 'static;
    type PrepaintState: 'static;
    
    fn id(&self) -> Option<ElementId>;
    
    // Phase 1: Request layout from Taffy
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState);
    
    // Phase 2: Bounds resolved, prepare for painting
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState;
    
    // Phase 3: Paint to screen
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    );
    
    fn into_any(self) -> AnyElement;
}
```

### Canvas Element (Quick Custom Drawing)
```rust
canvas(
    // Prepaint: calculate state
    |bounds, window, cx| {
        let center = bounds.center();
        center
    },
    // Paint: draw using state from prepaint
    |bounds, center, window, cx| {
        window.paint_quad(PaintQuad { /* ... */ });
    },
)
```

### IntoElement Trait
```rust
pub trait IntoElement: Sized {
    type Element: Element;
    fn into_element(self) -> Self::Element;
    fn into_any_element(self) -> AnyElement;
}
```
**Implemented for:** `&'static str`, `String`, `SharedString`, `Div`, `Svg`, `Img`, `Anchored`, `Deferred`, `Stateful<E>`, `AnyView`, `AnyElement`, etc.

### ParentElement Trait
```rust
pub trait ParentElement: Sized {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>);
    fn child(self, child: impl IntoElement) -> Self;
    fn children(self, children: impl IntoIterator<Item = impl IntoElement>) -> Self;
}
```
**Implemented for:** `Div`, `Anchored`, `Stateful<E>` where E: ParentElement

---

## 7. Key Structs Reference

### `Div`
The all-in-one element. Private fields, but implements:
- `Element` (3-phase lifecycle)
- `InteractiveElement` (all 38 event methods)
- `IntoElement`
- `ParentElement` (`.child()`, `.children()`)
- `Styled` (all 3,010 style methods)

Custom methods:
```rust
impl Div {
    pub fn on_children_prepainted(self, listener: impl Fn(Vec<Bounds<Pixels>>, &mut Window, &mut App) + 'static) -> Self;
    pub fn image_cache(self, cache: impl ImageCacheProvider) -> Self;
}
```

### `Stateful<E>`
Wraps an element after `.id()` is called. Unlocks `StatefulInteractiveElement`:
```rust
div().id("my-id")  // returns Stateful<Div>
    .on_click(...)
    .active(|s| s.bg(darker()))
    .tooltip(|window, cx| /* ... */)
```

### `Entity<T>`
Strong typed reference to GPUI-managed state:
```rust
pub struct Entity<T> { /* ... */ }

impl<T> Entity<T> {
    fn read(&self, cx: &App) -> &T;
    fn update<R>(&self, cx: &mut App, f: impl FnOnce(&mut T, &mut Context<T>) -> R) -> R;
}
```

### `Window`
Holds state for a specific window. Key methods:
```rust
impl Window {
    // View Management
    pub fn replace_root<E: Render>(&mut self, cx: &mut App, build_view: ...) -> Entity<E>;
    pub fn root<E: Render>(&self) -> Option<Option<Entity<E>>>;
    
    // Focus Management
    pub fn focused(&self, cx: &App) -> Option<FocusHandle>;
    pub fn focus(&mut self, handle: &FocusHandle);
    pub fn blur(&mut self);
    pub fn focus_next(&mut self);
    pub fn focus_prev(&mut self);
    
    // Window Operations
    pub fn refresh(&mut self);                    // schedule redraw
    pub fn remove_window(&mut self);              // close window
    pub fn set_window_title(&mut self, title: &str);
    pub fn toggle_fullscreen(&mut self);
    pub fn window_bounds(&self) -> WindowBounds;
    pub fn viewport_size(&self) -> Size<Pixels>;
    pub fn scale_factor(&self) -> f32;
    pub fn mouse_position(&self) -> Point<Pixels>;
    pub fn modifiers(&self) -> Modifiers;
    
    // Observation & Events
    pub fn observe<T>(&mut self, entity: &Entity<T>, cx: &mut App, callback: ...) -> Subscription;
    pub fn subscribe<E: EventEmitter<Evt>>(&mut self, entity: &Entity<E>, cx: &mut App, callback: ...) -> Subscription;
    pub fn observe_release<T>(&self, entity: &Entity<T>, cx: &mut App, on_release: ...) -> Subscription;
    
    // Async & Scheduling
    pub fn defer(&self, cx: &mut App, f: impl FnOnce(&mut Window, &mut App) + 'static);
    pub fn on_next_frame(&self, callback: impl FnOnce(&mut Window, &mut App) + 'static);
    pub fn request_animation_frame(&self);
    pub fn spawn<R>(&self, cx: &App, f: AsyncFnOnce(&mut AsyncWindowContext) -> R) -> Task<R>;
    
    // Layout
    pub fn request_layout(&self, style: Style, children: impl IntoIterator<Item = LayoutId>) -> LayoutId;
    pub fn compute_layout(&mut self, layout_id: LayoutId, available_space: Size<AvailableSpace>);
    pub fn layout_bounds(&self, layout_id: LayoutId) -> Bounds<Pixels>;
    
    // Rendering Helpers
    pub fn text_style(&self) -> TextStyle;
    pub fn rem_size(&self) -> Pixels;
    pub fn with_text_style(&mut self, style: Option<&TextStyleRefinement>, f: impl FnOnce(&mut Window, &mut App));
    pub fn with_content_mask(&mut self, mask: Option<ContentMask<Pixels>>, f: impl FnOnce(&mut Window, &mut App));
    
    // Drawing Primitives
    pub fn paint_quad(&mut self, quad: PaintQuad);
    pub fn paint_path(&mut self, path: Path<Pixels>);
    pub fn paint_svg(&mut self, ...);
    pub fn paint_image(&mut self, ...);
    pub fn paint_shadows(&mut self, ...);
    pub fn paint_layer(&mut self, bounds: Bounds<Pixels>, f: impl FnOnce(&mut Window, &mut App));
    
    // Hitbox
    pub fn insert_hitbox(&mut self, bounds: Bounds<Pixels>, opaque: bool) -> Hitbox;
    
    // Actions & Keybindings
    pub fn dispatch_action(&mut self, action: Box<dyn Action>, cx: &mut App);
    pub fn is_action_available(&self, action: &dyn Action, cx: &App) -> bool;
    pub fn bindings_for_action(&self, action: &dyn Action, cx: &App) -> Vec<KeyBinding>;
}
```

### Color Types
```rust
// Primary color type
pub struct Hsla { h: f32, s: f32, l: f32, a: f32 }

// Color constructor helpers
hsla(h, s, l, a) -> Hsla          // HSL with alpha
rgb(hex: u32) -> Hsla             // e.g., rgb(0xFF0000)
rgba(hex: u32) -> Hsla            // e.g., rgba(0xFF0000FF)
black() -> Hsla
white() -> Hsla
red() -> Hsla
green() -> Hsla
blue() -> Hsla
yellow() -> Hsla
opaque_grey(lightness, opacity) -> Hsla
transparent_black() -> Hsla
transparent_white() -> Hsla
```

### Unit Types
```rust
pub struct Pixels(pub f32);
pub struct Rems(pub f32);
pub struct Percentage(pub f32);

// Constructors
px(value: f32) -> Pixels
rems(value: f32) -> Rems
relative(fraction: f32) -> Percentage  // 0.0 to 1.0
percentage(value: f32) -> Percentage   // 0 to 100

// Length Enums (for style properties)
pub enum Length { Definite(DefiniteLength), Auto }
pub enum DefiniteLength { Absolute(AbsoluteLength), Fraction(f32) }
pub enum AbsoluteLength { Rems(f32), Pixels(f32) }
```

### Geometry Types
```rust
pub struct Bounds<T> { origin: Point<T>, size: Size<T> }
pub struct Point<T> { x: T, y: T }
pub struct Size<T> { width: T, height: T }
pub struct Edges<T> { top: T, right: T, bottom: T, left: T }
pub struct Corners<T> { top_left: T, top_right: T, bottom_left: T, bottom_right: T }

// Constructors
point(x, y) -> Point
size(width, height) -> Size
bounds(origin, size) -> Bounds
```

### Focus & Scroll Handles
```rust
pub struct FocusHandle { /* ... */ }  // Track/manipulate focused element
pub struct ScrollHandle { /* ... */ }  // Track/manipulate scroll position
```

---

## 8. Animation System

### `Animation` Struct
```rust
pub struct Animation {
    pub duration: Duration,     // how long to animate
    pub oneshot: bool,          // false = repeat
    pub easing: Rc<dyn Fn(f32) -> f32>,  // easing function (0→1 input/output)
}

impl Animation {
    pub fn new(duration: Duration) -> Self;     // linear easing, oneshot by default
    pub fn repeat(self) -> Self;                // loop the animation
    pub fn with_easing(self, easing: impl Fn(f32) -> f32 + 'static) -> Self;
}
```

### `AnimationExt` Trait
Implemented for ALL types that implement `IntoElement`:
```rust
pub trait AnimationExt {
    fn with_animation(
        self,
        id: impl Into<ElementId>,
        animation: Animation,
        animator: impl Fn(Self, f32) -> Self + 'static,  // f32 is delta 0→1
    ) -> AnimationElement<Self>;
    
    fn with_animations(
        self,
        id: impl Into<ElementId>,
        animations: Vec<Animation>,
        animator: impl Fn(Self, usize, f32) -> Self + 'static,  // usize = animation index
    ) -> AnimationElement<Self>;
}
```

### Built-in Easing Functions
```rust
bounce()           // bouncy easing
ease_in_out()      // smooth acceleration/deceleration
ease_out_quint()   // fast start, smooth end
linear()           // constant speed
quadratic()        // quadratic curve
pulsating_between(start: f32, end: f32)  // oscillate between values
```

### Animation Example
```rust
div()
    .with_animation(
        "fade-in",
        Animation::new(Duration::from_millis(300))
            .with_easing(ease_in_out()),
        |el, delta| el.opacity(delta),  // delta goes from 0.0 to 1.0
    )
```

---

## 9. Prelude Re-exports

```rust
use gpui::prelude::*;
```

Includes:
- **Traits:** `AppContext` (as _), `BorrowAppContext`, `Context`, `Element`, `InteractiveElement`, `IntoElement`, `ParentElement`, `Render`, `RenderOnce`, `StatefulInteractiveElement`, `Styled`, `StyledImage`, `VisualContext`
- **Utilities:** `FluentBuilder` (`.when()`, `.when_some()`, `.map()`)
- **Derive Macros:** `#[derive(IntoElement)]`, `#[derive(Refineable)]`, `#[derive(VisualContext)]`
- **Marker Trait:** `Refineable` (for style cascading)

---

## 10. Windows & Views

### Opening a Window
```rust
Application::new().run(|cx: &mut App| {
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(800.), px(600.)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("My App".into()),
                ..Default::default()
            }),
            ..Default::default()
        },
        |window, cx| cx.new(|cx| MyRootView::new(cx)),
    );
});
```

### View Composition Pattern
```rust
struct MainView {
    sidebar: Entity<Sidebar>,
    content: Entity<Content>,
}

impl Render for MainView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .size_full()
            .child(self.sidebar.clone())    // Entity<T: Render> impls IntoElement
            .child(self.content.clone())
    }
}
```

### `AnyView` — Type-Erased View
```rust
let any_view: AnyView = entity.into();
div().child(any_view)
```

---

## Quick Reference: Building a shadcn-ui Button

```rust
use gpui::prelude::*;
use gpui::*;

#[derive(Clone, Copy, PartialEq)]
enum ButtonVariant { Default, Destructive, Outline, Secondary, Ghost, Link }

#[derive(Clone, Copy, PartialEq)]
enum ButtonSize { Default, Sm, Lg, Icon }

#[derive(IntoElement)]
struct Button {
    label: SharedString,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl Button {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::Default,
            size: ButtonSize::Default,
            disabled: false,
            on_click: None,
        }
    }
    
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
    
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }
    
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    
    pub fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let base = div()
            .id("button")
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .font_weight(FontWeight::MEDIUM)
            .cursor_pointer()
            .text_sm();
        
        // Apply size
        let sized = match self.size {
            ButtonSize::Default => base.h_10().px_4().py_2(),
            ButtonSize::Sm => base.h_9().rounded_md().px_3(),
            ButtonSize::Lg => base.h_11().rounded_md().px_8(),
            ButtonSize::Icon => base.h_10().w_10(),
        };
        
        // Apply variant colors
        let styled = match self.variant {
            ButtonVariant::Default => sized
                .bg(hsla(0.0, 0.0, 0.09, 1.0))
                .text_color(hsla(0.0, 0.0, 0.98, 1.0))
                .hover(|s| s.bg(hsla(0.0, 0.0, 0.09, 0.9))),
            ButtonVariant::Destructive => sized
                .bg(hsla(0.0, 0.84, 0.60, 1.0))
                .text_color(hsla(0.0, 0.0, 0.98, 1.0))
                .hover(|s| s.bg(hsla(0.0, 0.84, 0.60, 0.9))),
            ButtonVariant::Outline => sized
                .border_1()
                .border_color(hsla(0.0, 0.0, 0.90, 1.0))
                .bg(hsla(0.0, 0.0, 1.0, 1.0))
                .hover(|s| s.bg(hsla(0.0, 0.0, 0.96, 1.0))),
            ButtonVariant::Secondary => sized
                .bg(hsla(0.0, 0.0, 0.96, 1.0))
                .text_color(hsla(0.0, 0.0, 0.09, 1.0))
                .hover(|s| s.bg(hsla(0.0, 0.0, 0.96, 0.8))),
            ButtonVariant::Ghost => sized
                .hover(|s| s.bg(hsla(0.0, 0.0, 0.96, 1.0))),
            ButtonVariant::Link => sized
                .text_color(hsla(0.0, 0.0, 0.09, 1.0))
                .underline()
                .hover(|s| s), // already has underline
        };
        
        // Apply disabled state and click handler
        let interactive = if self.disabled {
            styled.opacity(0.5).cursor_default()
        } else if let Some(handler) = self.on_click {
            styled.on_click(handler)
                .active(|s| s.opacity(0.9))
        } else {
            styled
        };
        
        interactive.child(self.label)
    }
}
```

### Usage
```rust
impl Render for MyView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .gap_2()
            .p_4()
            .child(Button::new("Click me").on_click(|_, _, _| println!("clicked!")))
            .child(Button::new("Danger").variant(ButtonVariant::Destructive))
            .child(Button::new("Outline").variant(ButtonVariant::Outline).size(ButtonSize::Sm))
            .child(Button::new("Disabled").disabled(true))
    }
}
```

---

## Key Patterns Summary

1. **Fluent Builder API** — All styling/events are chained: `div().flex().items_center().p_4().bg(blue())`
2. **`.id()` Promotion** — `div()` → `InteractiveElement`; `div().id("x")` → `StatefulInteractiveElement` (unlocks click, active, tooltip)
3. **`Render` vs `RenderOnce`** — `Render` = stateful views (entities); `RenderOnce` = stateless components (consumed on render)
4. **State Flow**: `cx.notify()` → triggers re-render of observers; `cx.emit(event)` → fires typed event to subscribers
5. **Style Cascading**: `hover(|s| s.bg(...))`, `active(|s| s.bg(...))`, `focus(|s| s.bg(...))` — closures receive `StyleRefinement`
6. **All listener signatures**: `impl Fn(&EventType, &mut Window, &mut App) + 'static`
7. **Conditional rendering**: `.when(bool, |el| el.some_style())`, `.when_some(option, |el, val| ...)`
8. **Color system**: HSLA primary (`hsla(h, s, l, a)`), with `rgb()`, `rgba()` helpers
9. **Units**: Tailwind scale (1 = 0.25rem), plus `px()`, `rems()`, `relative()`, `auto()`
10. **Animation**: `element.with_animation(id, Animation::new(duration).with_easing(ease_in_out()), |el, t| el.opacity(t))`
