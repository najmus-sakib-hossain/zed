use std::{cell::RefCell, ops::Deref, rc::Rc};

use gpui::{
    App, Bounds, ContentMask, DismissEvent, Element, ElementId, Entity, EventEmitter, FocusHandle,
    Focusable, GlobalElementId, Hitbox, InteractiveElement, IntoElement, LayoutId, MouseDownEvent,
    ParentElement as _, Pixels, Render, Size, Style, Styled as _, Window, canvas, div,
};

use wry::{
    Rect,
    dpi::{self, LogicalPosition, LogicalSize},
};

pub struct EmbeddedWebView {
    focus_handle: FocusHandle,
    webview: Rc<wry::WebView>,
    visible: bool,
    bounds: Rc<RefCell<Bounds<Pixels>>>,
}

impl Drop for EmbeddedWebView {
    fn drop(&mut self) {
        // On Windows, WebView2 teardown can race with the parent HWND being destroyed.
        // Calling into WebView2 here can produce noisy COM errors like
        // "Invalid window handle" / "Class not registered".
        //
        // Non-Windows: keep the explicit hide behavior.
        #[cfg(not(target_os = "windows"))]
        self.hide();
    }
}

impl EmbeddedWebView {
    pub fn from_webview(webview: wry::WebView, cx: &mut gpui::Context<Self>) -> Self {
        let _ = webview.set_bounds(Rect::default());

        Self {
            focus_handle: cx.focus_handle(),
            visible: true,
            bounds: Rc::new(RefCell::new(Bounds::default())),
            webview: Rc::new(webview),
        }
    }

    #[cfg_attr(target_os = "windows", allow(dead_code))]
    pub fn new(url: &str, window: &Window, cx: &mut gpui::Context<Self>) -> Self {
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            let _ = gtk::init();
        }

        let builder =
            wry::WebViewBuilder::new()
                .with_url(url)
                .with_on_page_load_handler(|event, url| {
                    let phase = match event {
                        wry::PageLoadEvent::Started => "started",
                        wry::PageLoadEvent::Finished => "finished",
                    };
                    tracing::info!(target: "dx_webview", phase, url = %url);
                });

        // Windows: prefer the WebView2 *controller* (build), not a child HWND.
        // In many GPU-rendered apps, child HWNDs can end up occluded.
        #[cfg(target_os = "windows")]
        let webview = builder.build(window).expect("webview should build");

        // macOS: child view embedding is fine.
        #[cfg(target_os = "macos")]
        let webview = builder.build_as_child(window).expect("webview should build as child");

        // Linux/FreeBSD: Wry supports X11 via build().
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let webview = builder.build(window).expect("webview should build");

        Self::from_webview(webview, cx)
    }

    #[cfg(not(target_os = "windows"))]
    pub fn hide(&mut self) {
        let _ = self.webview.focus_parent();
        let _ = self.webview.set_visible(false);
        self.visible = false;
    }

    pub fn visible(&self) -> bool {
        self.visible
    }
}

impl Deref for EmbeddedWebView {
    type Target = wry::WebView;

    fn deref(&self) -> &Self::Target {
        &self.webview
    }
}

impl Focusable for EmbeddedWebView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for EmbeddedWebView {}

impl Render for EmbeddedWebView {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let webview_rc = self.webview.clone();
        let focus_handle = self.focus_handle.clone();
        let bounds_rc = self.bounds.clone();

        div()
            .track_focus(&focus_handle)
            .size_full()
            .child(
                canvas(
                    move |bounds, _, _cx| {
                        *bounds_rc.borrow_mut() = bounds;
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .child(WebViewElement::new(webview_rc, cx.entity().clone(), window, cx))
    }
}

pub struct WebViewElement {
    parent: Entity<EmbeddedWebView>,
    view: Rc<wry::WebView>,
}

impl WebViewElement {
    pub fn new(
        view: Rc<wry::WebView>,
        parent: Entity<EmbeddedWebView>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self {
        Self { view, parent }
    }
}

impl IntoElement for WebViewElement {
    type Element = WebViewElement;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for WebViewElement {
    type RequestLayoutState = ();
    type PrepaintState = Option<Hitbox>;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            size: Size::full(),
            flex_shrink: 1.,
            ..Default::default()
        };

        let id = window.request_layout(style, [], cx);
        (id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if !self.parent.read(cx).visible() {
            return None;
        }

        // IMPORTANT (Windows/WebView2): `set_bounds` expects *logical* coordinates.
        // Wry converts them to physical pixels using the webview HWND DPI internally.
        self.view
            .set_bounds(Rect {
                size: dpi::Size::Logical(LogicalSize {
                    width: bounds.size.width.into(),
                    height: bounds.size.height.into(),
                }),
                position: dpi::Position::Logical(LogicalPosition::new(
                    bounds.origin.x.into(),
                    bounds.origin.y.into(),
                )),
            })
            .unwrap();

        Some(window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal))
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        let bounds = hitbox.clone().map(|h| h.bounds).unwrap_or(bounds);
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            let webview = self.view.clone();
            window.on_mouse_event(move |event: &MouseDownEvent, _, _, _| {
                if !bounds.contains(&event.position) {
                    let _ = webview.focus_parent();
                }
            });
        });
    }
}
