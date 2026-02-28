import type { Metadata } from 'next';

export const metadata: Metadata = {
  title: 'GPUI Desktop App Tutorial - February 2026',
  description: 'Learn how to build desktop applications using Zed\'s GPUI Rust crate',
};

export default function GPUITutorialPage() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-950 via-gray-900 to-gray-950">
      <div className="max-w-5xl mx-auto px-6 py-16">
        {/* Header */}
        <header className="mb-16">
          <div className="inline-block px-4 py-2 mb-4 text-sm font-medium text-purple-400 bg-purple-950/50 rounded-full border border-purple-800/30">
            February 20, 2026
          </div>
          <h1 className="text-5xl font-bold text-white mb-4 bg-gradient-to-r from-purple-400 via-pink-400 to-orange-400 bg-clip-text text-transparent">
            Building Desktop Apps with GPUI
          </h1>
          <p className="text-xl text-gray-400">
            A comprehensive guide to using Zed's GPUI Rust crate for creating high-performance desktop applications
          </p>
        </header>

        {/* Introduction */}
        <section className="mb-16 p-8 bg-gray-900/50 rounded-2xl border border-gray-800">
          <h2 className="text-3xl font-bold text-white mb-4">What is GPUI?</h2>
          <p className="text-gray-300 mb-4">
            GPUI is a GPU-accelerated UI framework written in Rust by the Zed team. It powers the Zed editor and provides a declarative, reactive approach to building native desktop applications with exceptional performance.
          </p>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mt-6">
            <div className="p-4 bg-purple-950/30 rounded-lg border border-purple-800/30">
              <div className="text-2xl mb-2">⚡</div>
              <h3 className="font-semibold text-white mb-2">GPU Accelerated</h3>
              <p className="text-sm text-gray-400">Hardware-accelerated rendering for smooth 60fps+ UIs</p>
            </div>
            <div className="p-4 bg-cyan-950/30 rounded-lg border border-cyan-800/30">
              <div className="text-2xl mb-2">🦀</div>
              <h3 className="font-semibold text-white mb-2">Pure Rust</h3>
              <p className="text-sm text-gray-400">Memory-safe, zero-cost abstractions, no GC pauses</p>
            </div>
            <div className="p-4 bg-pink-950/30 rounded-lg border border-pink-800/30">
              <div className="text-2xl mb-2">🎨</div>
              <h3 className="font-semibold text-white mb-2">Declarative</h3>
              <p className="text-sm text-gray-400">React-like component model with type safety</p>
            </div>
          </div>
        </section>

        {/* Getting Started */}
        <section className="mb-16">
          <h2 className="text-3xl font-bold text-white mb-6">Getting Started</h2>
          
          <div className="space-y-6">
            <div className="p-6 bg-gray-900/50 rounded-xl border border-gray-800">
              <h3 className="text-xl font-semibold text-white mb-3">1. Add GPUI to Your Project</h3>
              <pre className="bg-black/50 p-4 rounded-lg overflow-x-auto">
                <code className="text-sm text-gray-300">{`[dependencies]
gpui = { git = "https://github.com/zed-industries/zed" }
smallvec = "1.11"`}</code>
              </pre>
            </div>

            <div className="p-6 bg-gray-900/50 rounded-xl border border-gray-800">
              <h3 className="text-xl font-semibold text-white mb-3">2. Create Your First Component</h3>
              <pre className="bg-black/50 p-4 rounded-lg overflow-x-auto">
                <code className="text-sm text-gray-300">{`use gpui::*;

struct MyApp;

impl Render for MyApp {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x1a1a1a))
            .child("Hello, GPUI!")
    }
}`}</code>
              </pre>
            </div>

            <div className="p-6 bg-gray-900/50 rounded-xl border border-gray-800">
              <h3 className="text-xl font-semibold text-white mb-3">3. Launch Your App</h3>
              <pre className="bg-black/50 p-4 rounded-lg overflow-x-auto">
                <code className="text-sm text-gray-300">{`fn main() {
    App::new().run(|cx: &mut AppContext| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(
                Bounds::centered(None, size(px(800.0), px(600.0)), cx)
            )),
            ..Default::default()
        };
        
        cx.open_window(options, |cx| {
            cx.new_view(|_cx| MyApp)
        }).unwrap();
    });
}`}</code>
              </pre>
            </div>
          </div>
        </section>

        {/* Advanced Example */}
        <section className="mb-16">
          <h2 className="text-3xl font-bold text-white mb-6">Advanced: Animated Glow Card</h2>
          <p className="text-gray-300 mb-6">
            Let's build a beautiful card with a multi-color gradient border and floating action buttons. This example demonstrates:
          </p>
          <ul className="list-disc list-inside text-gray-300 mb-6 space-y-2">
            <li>Linear gradients for neon glow effects</li>
            <li>Absolute positioning for floating elements</li>
            <li>Interactive hover states</li>
            <li>SVG icon rendering</li>
          </ul>

          <div className="p-6 bg-gray-900/50 rounded-xl border border-gray-800">
            <pre className="bg-black/50 p-4 rounded-lg overflow-x-auto max-h-[600px]">
              <code className="text-sm text-gray-300">{`use gpui::*;
use smallvec::smallvec;

struct GlowCard;

impl Render for GlowCard {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .size_full()
            .bg(rgb(0x050505))
            .justify_center()
            .items_center()
            .child(
                div()
                    .w(px(500.0))
                    .h(px(300.0))
                    .rounded_xl()
                    .bg(linear_gradient(
                        140.0,
                        smallvec![
                            GradientStop { position: 0.0, color: rgb(0xa855f7) },
                            GradientStop { position: 0.3, color: rgb(0x06b6d4) },
                            GradientStop { position: 0.6, color: rgb(0xec4899) },
                            GradientStop { position: 1.0, color: rgb(0xf97316) },
                        ],
                    ))
                    .p(px(2.0))
                    .shadow_2xl()
                    .child(
                        div()
                            .size_full()
                            .relative()
                            .rounded_xl()
                            .bg(rgb(0x000000))
                            .child(
                                div()
                                    .absolute()
                                    .bottom_4()
                                    .right_4()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(icon_button("expand"))
                                    .child(icon_button("sparkle"))
                            )
                    )
            )
    }
}

fn icon_button(icon_type: &str) -> impl IntoElement {
    let path = match icon_type {
        "expand" => "M15 3h6v6M14 10l6.1-6.1M9 21H3v-6M10 14l-6.1 6.1",
        _ => "M9.9 14.1L5 19M20 10c0 5.5-4.5 10-10 10S0 15.5 0 10",
    };
    
    div()
        .w_10()
        .h_10()
        .flex()
        .items_center()
        .justify_center()
        .rounded_xl()
        .bg(rgba(0.2, 0.1, 0.3, 0.6))
        .hover(|s| s.bg(rgba(0.3, 0.2, 0.4, 0.8)))
        .cursor_pointer()
        .child(
            svg()
                .path(path)
                .size_5()
                .text_color(rgb(0xffffff))
        )
}`}</code>
            </pre>
          </div>
        </section>

        {/* Key Concepts */}
        <section className="mb-16">
          <h2 className="text-3xl font-bold text-white mb-6">Key Concepts</h2>
          
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div className="p-6 bg-gray-900/50 rounded-xl border border-gray-800">
              <h3 className="text-xl font-semibold text-white mb-3">Layout System</h3>
              <p className="text-gray-300 mb-3">GPUI uses a flexbox-inspired layout system:</p>
              <ul className="text-sm text-gray-400 space-y-1">
                <li>• <code className="text-purple-400">flex()</code> - Enable flex layout</li>
                <li>• <code className="text-purple-400">flex_col()</code> - Column direction</li>
                <li>• <code className="text-purple-400">justify_center()</code> - Center main axis</li>
                <li>• <code className="text-purple-400">items_center()</code> - Center cross axis</li>
              </ul>
            </div>

            <div className="p-6 bg-gray-900/50 rounded-xl border border-gray-800">
              <h3 className="text-xl font-semibold text-white mb-3">Styling</h3>
              <p className="text-gray-300 mb-3">Type-safe styling with method chaining:</p>
              <ul className="text-sm text-gray-400 space-y-1">
                <li>• <code className="text-cyan-400">bg()</code> - Background color</li>
                <li>• <code className="text-cyan-400">rounded_xl()</code> - Border radius</li>
                <li>• <code className="text-cyan-400">shadow_2xl()</code> - Box shadow</li>
                <li>• <code className="text-cyan-400">hover()</code> - Hover states</li>
              </ul>
            </div>

            <div className="p-6 bg-gray-900/50 rounded-xl border border-gray-800">
              <h3 className="text-xl font-semibold text-white mb-3">Positioning</h3>
              <p className="text-gray-300 mb-3">Absolute and relative positioning:</p>
              <ul className="text-sm text-gray-400 space-y-1">
                <li>• <code className="text-pink-400">relative()</code> - Position context</li>
                <li>• <code className="text-pink-400">absolute()</code> - Absolute positioning</li>
                <li>• <code className="text-pink-400">top_4()</code>, <code className="text-pink-400">right_4()</code> - Offsets</li>
              </ul>
            </div>

            <div className="p-6 bg-gray-900/50 rounded-xl border border-gray-800">
              <h3 className="text-xl font-semibold text-white mb-3">Colors & Gradients</h3>
              <p className="text-gray-300 mb-3">Rich color system with gradients:</p>
              <ul className="text-sm text-gray-400 space-y-1">
                <li>• <code className="text-orange-400">rgb(0xffffff)</code> - Hex colors</li>
                <li>• <code className="text-orange-400">rgba(r, g, b, a)</code> - With alpha</li>
                <li>• <code className="text-orange-400">linear_gradient()</code> - Linear gradients</li>
                <li>• <code className="text-orange-400">radial_gradient()</code> - Radial gradients</li>
              </ul>
            </div>
          </div>
        </section>

        {/* Resources */}
        <section className="mb-16">
          <h2 className="text-3xl font-bold text-white mb-6">Resources</h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <a href="https://github.com/zed-industries/zed" className="p-6 bg-gray-900/50 rounded-xl border border-gray-800 hover:border-purple-600 transition-colors">
              <h3 className="text-lg font-semibold text-white mb-2">📚 Zed Repository</h3>
              <p className="text-sm text-gray-400">Official GPUI source code and examples</p>
            </a>
            <a href="https://zed.dev" className="p-6 bg-gray-900/50 rounded-xl border border-gray-800 hover:border-cyan-600 transition-colors">
              <h3 className="text-lg font-semibold text-white mb-2">🌐 Zed Website</h3>
              <p className="text-sm text-gray-400">Learn more about the Zed editor</p>
            </a>
          </div>
        </section>

        {/* Footer */}
        <footer className="text-center text-gray-500 text-sm">
          <p>Built with DX • February 20, 2026</p>
        </footer>
      </div>
    </div>
  );
}
