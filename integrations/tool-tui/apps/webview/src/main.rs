mod glow_card;
mod logging;
mod platform_titlebar;
mod theme;
mod titlebar;
mod webview;
mod window_controls;

use gpui::{
    App, Application, Bounds, Context, Render, TitlebarOptions, Window, WindowBounds,
    WindowOptions, div, hsla, prelude::*, px, size,
};

use glow_card::GlowCard;
use webview::EmbeddedWebView;

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
struct RawWindowHandles {
    window: raw_window_handle::RawWindowHandle,
}

#[cfg(target_os = "windows")]
impl raw_window_handle::HasWindowHandle for RawWindowHandles {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        // SAFETY: These raw handles originate from GPUI for a live window. We only use them
        // immediately to construct a WebView, while the window is still open.
        unsafe { Ok(raw_window_handle::WindowHandle::borrow_raw(self.window)) }
    }
}

struct Dx {
    webview: Option<gpui::Entity<EmbeddedWebView>>,
    glow_card: Option<gpui::Entity<GlowCard>>,
    show_glow_card: bool,
}

impl Dx {
    fn toggle_view(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.show_glow_card = !self.show_glow_card;
        cx.notify();
    }
}

impl Render for Dx {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let webview = self.webview.clone();
        let glow_card = self.glow_card.clone();
        let show_glow = self.show_glow_card;

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(gpui::transparent_black())
            .text_color(theme::foreground())
            .child(platform_titlebar::platform_titlebar(
                px(40.),
                titlebar::DxTitleBar::new(if show_glow {
                    "GPUI Glow Card Demo"
                } else {
                    "google.com"
                }),
            ))
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .h_full()
                    .bg(gpui::transparent_black())
                    .when(show_glow, |this| {
                        this.when_some(glow_card, |this, card| this.child(card))
                    })
                    .when(!show_glow, |this| {
                        this.when_some(webview, |this, webview| this.child(webview))
                    }),
            )
            .child(
                // Toggle button
                div()
                    .absolute()
                    .bottom(px(20.0))
                    .right(px(20.0))
                    .px(px(16.0))
                    .py(px(8.0))
                    .bg(hsla(270.0 / 360.0, 0.5, 0.15, 0.8))
                    .rounded(px(8.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(hsla(270.0 / 360.0, 0.5, 0.2, 0.9)))
                    .child(div().text_sm().text_color(gpui::white()).child(if show_glow {
                        "Show WebView (Space)"
                    } else {
                        "Show Glow Card (Space)"
                    }))
                    .id("toggle-button")
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|dx, _event, window, cx| {
                            dx.toggle_view(window, cx);
                        }),
                    ),
            )
    }
}

fn window_options(cx: &mut App) -> WindowOptions {
    let bounds = Bounds::centered(None, size(px(1100.), px(720.)), cx);

    let mut options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        ..Default::default()
    };

    #[cfg(target_os = "macos")]
    {
        options.titlebar = Some(TitlebarOptions {
            title: Some("DX".into()),
            appears_transparent: false,
            traffic_light_position: Some(gpui::point(px(12.), px(12.))),
        });
    }

    #[cfg(target_os = "windows")]
    {
        options.titlebar = Some(TitlebarOptions {
            title: Some("DX".into()),
            appears_transparent: true,
            traffic_light_position: None,
        });
        options.window_decorations = Some(gpui::WindowDecorations::Client);
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        options.titlebar = Some(TitlebarOptions {
            title: Some("DX".into()),
            appears_transparent: false,
            traffic_light_position: None,
        });

        options.window_decorations = Some(gpui::WindowDecorations::Server);
    }

    options
}

fn main() {
    Application::new().run(|cx: &mut App| {
        logging::init();
        tracing::info!(target: "dx", "starting");

        let options = window_options(cx);
        let window = cx
            .open_window(options, |window, cx| {
                let _ = window;

                // Create glow card
                let glow_card = cx.new(|cx| GlowCard::new(cx));

                cx.new(|_| Dx {
                    webview: None,
                    glow_card: Some(glow_card),
                    show_glow_card: true,
                })
            })
            .expect("window should open");

        // Register keyboard shortcut for toggling
        window.update(cx, |_, _window, _cx| {}).ok();

        window.update(cx, |_, _window, cx| cx.activate(true)).ok();

        let url = "https://google.com";

        #[cfg(target_os = "windows")]
        {
            let handles = window
                .update(cx, |_, window, _cx| {
                    Ok::<_, raw_window_handle::HandleError>(RawWindowHandles {
                        window: raw_window_handle::HasWindowHandle::window_handle(window)?.as_raw(),
                    })
                })
                .expect("window should still be alive")
                .expect("window handle should be available");

            let window_for_webview = window.clone();

            // Windows/WebView2: build the webview as a *child* HWND.
            // This avoids lb-wry's parent-subclass proc (which can dereference a null pointer
            // during certain focus/destroy message sequences).
            cx.spawn(async move |cx| {
                let webview = wry::WebViewBuilder::new()
                    .with_url(url)
                    .with_on_page_load_handler(|event, url| {
                        let phase = match event {
                            wry::PageLoadEvent::Started => "started",
                            wry::PageLoadEvent::Finished => "finished",
                        };
                        tracing::info!(target: "dx_webview", phase, url = %url);
                    })
                    .build_as_child(&handles);

                let webview = match webview {
                    Ok(webview) => webview,
                    Err(error) => {
                        tracing::error!(target: "dx_webview", ?error, "failed to build webview");
                        return;
                    }
                };

                let _ = window_for_webview.update(cx, |this, _window, cx| {
                    if this.webview.is_none() {
                        this.webview =
                            Some(cx.new(|cx| EmbeddedWebView::from_webview(webview, cx)));
                    }
                });
            })
            .detach();
        }

        #[cfg(not(target_os = "windows"))]
        {
            window
                .update(cx, |this, window, cx| {
                    if this.webview.is_none() {
                        this.webview = Some(cx.new(|cx| EmbeddedWebView::new(url, window, cx)));
                    }
                })
                .ok();
        }

        tracing::info!(target: "dx", "window opened");
    });
}
