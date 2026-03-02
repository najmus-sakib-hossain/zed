# 🏗️ GPUI Media App — Pure Rust Single-Binary Desktop App

A comprehensive GPUI desktop application showcasing 7 rich media components, all running in a **single binary** with **zero external dependencies**.

## 🧩 Components

| Tab | Component | Strategy | Crates |
|-----|-----------|----------|--------|
| 🎬 Video | H.264 video player | symphonia demux → openh264 decode → RGBA → GPUI | `symphonia`, `openh264` |
| 🧊 3D | Real-time 3D renderer | wgpu offscreen render → readback → RGBA → GPUI | `wgpu`, `glam`, `bytemuck` |
| 🔊 Audio | Audio player with controls | rodio + symphonia decode → OS audio out | `rodio` |
| 📄 PDF | PDF page viewer | hayro render → PNG → RGBA → GPUI | `hayro` |
| 📝 Docs | DOCX document viewer | docx-rs parse → text extraction → GPUI layout | `docx-rs` |
| 📐 LaTeX | Typst + KaTeX renderer | typst compile → bitmap / katex → SVG → resvg → GPUI | `typst`, `katex`, `resvg` |
| 📊 Charts | Interactive charts | plotters bitmap backend → RGBA → GPUI | `plotters`, `plotters-bitmap` |

## 🏛️ Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     YOUR SINGLE BINARY                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   CONTENT DECODER/RENDERER          BRIDGE         GPUI UI      │
│   ========================    ===============    ============   │
│                                                                 │
│   symphonia + openh264  ─→  RgbaImage  ─→  img()  → VideoView  │
│   wgpu + glam           ─→  RgbaImage  ─→  img()  → ThreeDView │
│   rodio + cpal          ─→  (audio out) ─→  div()  → AudioView │
│   hayro (pure Rust)     ─→  RgbaImage  ─→  img()  → PdfView    │
│   docx-rs               ─→  text/style ─→  div()  → DocView    │
│   typst + typst-render  ─→  RgbaImage  ─→  img()  → LatexView  │
│   katex + resvg         ─→  RgbaImage  ─→  img()  → LatexView  │
│   plotters-bitmap       ─→  RgbaImage  ─→  img()  → ChartView  │
│                                                                 │
│   bitmap_utils::rgba_to_gpui_image() — the universal bridge    │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│   macOS: Metal  │  Linux: Vulkan  │  Windows: DX12/Vulkan       │
└─────────────────────────────────────────────────────────────────┘
```

## 🚀 Build & Run

```bash
cargo run --release
```

## ⚠️ Notes

- The `typst::World` trait implementation is left as a `todo!()` — use the `typst-as-lib` crate for a ready-made `World` implementation.
- GPUI is still pre-1.0 and APIs may change between versions. Always check the latest docs.
- Some crates (e.g., `hayro`) may be very new — verify availability on crates.io before building.

## 📄 License

MIT
