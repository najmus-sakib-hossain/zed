Great point! Let me find the best **pure Rust / statically-linkable** crates that can be embedded directly into a **single binary** with zero external dependencies for the user. No AR/VR/XR/MR this time.Excellent! Here's the fully revised guide — **every crate is either pure Rust or can be statically linked into your single binary**. No user-installed dependencies. No AR/VR/XR/MR. Ship one binary, done. 🚀

---

# 🏗️ Single-Binary GPUI Desktop App — Crate Guide

> **Rule: The user downloads ONE binary. Nothing else. Ever.**

---

## 🎬 1. VIDEO — Decoding & Playback

### Strategy: Pure Rust decoders + pure Rust demuxers

#### ✅ `rav1d` — AV1 Decoder (Pure Rust)
rav1d is an AV1 cross-platform decoder, open-source, and focused on speed and correctness. It is a Rust port of dav1d. rav1d is written in Rust and uses the standard Rust toolchain to build.

rav1d is fully functional (they even have it working in Chromium), but it's about 5% slower than dav1d.

```toml
rav1d = "1.1"     # AV1 decoding — pure Rust, no C deps
```
**OS:** macOS ✅ Linux ✅ Windows ✅ — compiles with `cargo build`, zero external deps.

---

#### ✅ `rav1e` — AV1 Encoder (Pure Rust)
The fastest and safest AV1 encoder. rav1e is an AV1 video encoder designed to eventually cover all use cases.

```toml
rav1e = "0.8"     # AV1 encoding — pure Rust
```

---

#### ✅ `symphonia` — Demuxer (MP4/MKV/WebM containers — Pure Rust)
Symphonia is a pure Rust audio decoding and media demuxing library supporting AAC, ADPCM, AIFF, ALAC, CAF, FLAC, MKV, MP1, MP2, MP3, MP4, OGG, Vorbis, WAV, and WebM.

Symphonia aims to be comparable to, or faster than, popular open-source C-based implementations. Currently, Symphonia's decoders are generally +/-15% the performance of FFmpeg.

```toml
symphonia = { version = "0.5", features = ["all"] }
```

---

#### ✅ `openh264-rs` — H.264 Decoder (Statically linked)
Rust bindings to Cisco's OpenH264 (BSD licensed, auto-downloads the static lib at build time and links it into your binary).

```toml
openh264 = "0.6"  # H.264 decode/encode, statically linked
```

---

#### ✅ `ffmpeg-sidecar` (Alternative: Bundle FFmpeg binary)
Wraps a standalone FFmpeg binary in an intuitive Iterator interface.

The core goal is to provide a method of interacting with any video as if it were an array of raw RGB frames.

You can **embed the FFmpeg binary inside your app bundle** (in your `resources/` folder) and point `ffmpeg-sidecar` to it. The user never sees or installs FFmpeg.

```toml
ffmpeg-sidecar = "2.0"  # ships FFmpeg alongside your binary
```

---

#### 🎬 Recommended Video Stack

| Layer | Crate | Type |
|---|---|---|
| **Container demux** | `symphonia` | ✅ Pure Rust |
| **AV1 decode** | `rav1d` | ✅ Pure Rust |
| **H.264 decode** | `openh264` | ✅ Static link |
| **VP8/VP9 decode** | `vpx-rs` (static) | ✅ Static link |
| **AV1 encode** | `rav1e` | ✅ Pure Rust |
| **MP4 mux** | `muxide` | ✅ Pure Rust |
| **Fallback (all formats)** | `ffmpeg-sidecar` | ✅ Bundled binary |

> **GPUI integration:** Decode frames → `RGBA` buffer → paint as a GPUI custom `Element` texture every frame.

---

## 🧊 2. 3D Rendering

### ✅ `wgpu` — Cross-platform GPU API (Pure Rust)
wgpu is a cross-platform, safe, pure-Rust graphics API. It runs natively on Vulkan, Metal, D3D12, and OpenGL — zero external dependencies at runtime.

```toml
wgpu = "24"
```

### ✅ `rend3` — Higher-level 3D renderer (built on wgpu)
Provides PBR rendering, skyboxes, shadows, tonemapping — all via wgpu, so it's also fully self-contained.

```toml
rend3 = "0.3"
rend3-routine = "0.3"
```

### ✅ `glam` + `naga` — Math & Shaders (Pure Rust)
```toml
glam = "0.29"   # fast 3D math, pure Rust
naga = "24"     # shader translator (WGSL/SPIR-V/MSL/HLSL), pure Rust
```

**OS:** macOS (Metal) ✅ Linux (Vulkan) ✅ Windows (DX12/Vulkan) ✅ — **All compiled into your binary.**

> **GPUI integration:** Render 3D scene to an off-screen `wgpu` texture → copy pixels to GPUI image element.

---

## 🔊 3. AUDIO

### ✅ `rodio` — High-level playback (Pure Rust)
Cross-platform audio playback. Built on top of `cpal`, which talks directly to OS audio APIs (CoreAudio / WASAPI / ALSA) — no external libs needed.

```toml
rodio = "0.19"
```

### ✅ `symphonia` — Decode all formats (Pure Rust)
Symphonia is a pure Rust audio decoding and media demuxing library supporting AAC, ADPCM, AIFF, ALAC, CAF, FLAC, MKV, MP1, MP2, MP3, MP4, OGG, Vorbis, WAV, and WebM.

```toml
symphonia = { version = "0.5", features = ["all"] }
```

### ✅ `cpal` — Low-level audio I/O (Pure Rust)
Talks to OS-native audio APIs directly. No runtime dependency.

```toml
cpal = "0.16"
```

**OS:** macOS (CoreAudio) ✅ Linux (ALSA) ✅ Windows (WASAPI) ✅ — **Zero user deps.**

---

## 📄 4. PDF Rendering

### ✅ `hayro` — Pure Rust PDF Renderer (⭐ Best for single binary)
An experimental, work-in-progress PDF interpreter and renderer. hayro is a Rust crate with a simple task: It allows you to interpret one or many pages of a PDF file to convert them into PNG or SVG files. This is a difficult task, as the PDF specification is huge. In addition to that, there are millions of PDF files with many edge cases. This is not the first attempt at writing a PDF renderer in Rust, but, to the best of my knowledge, this is currently the most feature-complete library.

There are still certain features hayro currently doesn't support. However, the vast majority of common features is supported meaning that you should be able to render the "average" PDF file without encountering any issues.

```toml
hayro = "0.1"         # Pure Rust PDF render → PNG/SVG
```

**100% pure Rust. No PDFium. No C deps. Compiles right into your binary.**

---

### ✅ `pdfium-render` with **static linking** (Alternative — battle-tested)
You can bind to a dynamically-built Pdfium library packaged alongside your Rust executable. You can also bind to a statically-built Pdfium library linked to your executable at compile time.

You can **statically link Pdfium into your binary** at compile time:

```toml
pdfium-render = { version = "0.8", features = ["static", "image"] }
```

Set `PDFIUM_STATIC_LIB_PATH` at build time, and the Pdfium library gets baked into your binary. The path should not include the filename of the library itself; it should just be the path of the containing directory. You must make sure your library is named appropriately for your target platform.

---

### ✅ PDF Generation (Pure Rust)

A user-friendly PDF generator written in pure Rust. genpdf is a high-level PDF generator built on top of printpdf and rusttype. It takes care of the page layout and text alignment. All of its dependencies are written in Rust, so you don't need any pre-installed libraries or tools.

```toml
genpdf = "0.3"       # PDF generation — pure Rust
printpdf = "0.7"     # Lower-level PDF creation — pure Rust
```

---

## 📝 5. DOCUMENT Rendering (DOCX, etc.)

### ✅ `docx-rs` — Read/Write DOCX (Pure Rust)
```toml
docx-rs = "0.4"
```

### ✅ `calamine` — Read Excel/ODS (Pure Rust)
```toml
calamine = "0.26"    # xlsx, xls, ods — pure Rust
```

### Pipeline: DOCX → render in GPUI
```
docx-rs (parse) → extract text/styles/images → custom GPUI layout elements
            OR
docx-rs (parse) → genpdf (convert to PDF) → hayro (render to bitmap) → GPUI
```

**OS:** All pure Rust — macOS ✅ Linux ✅ Windows ✅

---

## 📐 6. LaTeX / TYPESETTING Rendering

### ✅ `typst` + `typst-render` — ⭐ BEST for Single Binary (Pure Rust)
Typst is a new markup-based typesetting system that is designed to be as powerful as LaTeX while being much easier to learn and use. Typst has built-in markup for the most common formatting tasks.

All of Typst has been designed with three key goals in mind: Power, simplicity, and performance. "It's time for a system that matches the power of LaTeX, is easy to learn and use, all while being fast enough to realize instant preview."

You can use the crate `typst` as a Rust library. This crate lets you embed the Typst compiler in your own applications, for example to generate printable report cards. The Typst compiler is licensed under the Apache-2.0 license.

`typst-render` is a raster image exporter for Typst.

```toml
typst = "0.14"            # Full typesetting compiler — pure Rust
typst-render = "0.14"     # Renders typst output → bitmap (PNG) — pure Rust
typst-as-lib = "0.3"      # Easy wrapper to use as library
```

> **Why Typst over tectonic?** Tectonic is a TeX engine based on XeTeX. It is partly written in C and has some non-Rust dependencies. Typst is **100% Rust** — perfect for single-binary shipping.

---

### ✅ `katex` — LaTeX Math → HTML/SVG (Pure Rust wrapper)
For rendering **LaTeX math expressions only** (not full documents):
```toml
katex = "0.4"   # LaTeX math → HTML/SVG
```

---

### Pipeline: LaTeX → GPUI
```
LaTeX source → typst compiler (pure Rust) → typst-render → bitmap → GPUI element
     OR
LaTeX math   → katex → SVG → resvg (pure Rust SVG renderer) → bitmap → GPUI
```

---

## 📊 7. CHART / PLOT Rendering

### ✅ `plotters` — Full charting library (Pure Rust)
```toml
plotters = "0.3"
plotters-bitmap = "0.3"   # render charts → in-memory bitmap
plotters-svg = "0.3"      # render charts → SVG
```

Plotters supports bar charts, line charts, scatter plots, histograms, heatmaps, 3D plots, and more — all pure Rust.

### ✅ `resvg` — SVG → Bitmap (Pure Rust)
If you generate SVG charts/diagrams from any source:
```toml
resvg = "0.44"    # SVG → PNG/bitmap, pure Rust, no C deps
```

**OS:** Pure Rust — macOS ✅ Linux ✅ Windows ✅

---

## 📦 Final `Cargo.toml` — Single Binary Stack

```toml
[dependencies]
# ═══════════════════ GPUI CORE ═══════════════════
gpui = { git = "https://github.com/zed-industries/zed" }

# ═══════════════════ VIDEO ═══════════════════════
rav1d       = "1.1"                                          # AV1 decode (pure Rust)
rav1e       = "0.8"                                          # AV1 encode (pure Rust)
openh264    = "0.6"                                          # H.264 (static link, auto-downloads)
symphonia   = { version = "0.5", features = ["all"] }        # Demux + audio decode (pure Rust)
muxide      = "0.1"                                          # MP4 mux (pure Rust)

# ═══════════════════ 3D ═════════════════════════
wgpu        = "24"                                           # GPU API (pure Rust)
rend3       = "0.3"                                          # 3D renderer (pure Rust)
glam        = "0.29"                                         # 3D math (pure Rust)

# ═══════════════════ AUDIO ═══════════════════════
rodio       = "0.19"                                         # Playback (pure Rust)
cpal        = "0.16"                                         # Low-level audio I/O (pure Rust)

# ═══════════════════ PDF ═════════════════════════
hayro       = "0.1"                                          # PDF render → bitmap (pure Rust)
genpdf      = "0.3"                                          # PDF generation (pure Rust)
printpdf    = "0.7"                                          # Low-level PDF (pure Rust)

# ═══════════════════ DOCUMENTS ═══════════════════
docx-rs     = "0.4"                                          # DOCX read/write (pure Rust)
calamine    = "0.26"                                         # Excel/ODS (pure Rust)

# ═══════════════════ TYPESETTING / LATEX ═════════
typst       = "0.14"                                         # Typesetting engine (pure Rust)
typst-render = "0.14"                                        # Typst → bitmap (pure Rust)
katex       = "0.4"                                          # LaTeX math → SVG (pure Rust)

# ═══════════════════ CHARTS ══════════════════════
plotters        = "0.3"                                      # Charts (pure Rust)
plotters-bitmap = "0.3"                                      # Charts → bitmap (pure Rust)

# ═══════════════════ UTILITIES ═══════════════════
resvg       = "0.44"                                         # SVG → bitmap (pure Rust)
image       = "0.25"                                         # Image processing (pure Rust)
```

---

## 🏛️ Architecture: How It All Fits in GPUI

```
┌─────────────────────────────────────────────────────────┐
│                    YOUR SINGLE BINARY                    │
├─────────────────────────────────────────────────────────┤
│  GPUI App (custom Element for each content type)        │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────┐  │
│  │  Video   │ │   3D     │ │  Audio   │ │   PDF     │  │
│  │ Element  │ │ Element  │ │ Element  │ │  Element  │  │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └─────┬─────┘  │
│       │             │            │              │        │
│  ┌────▼─────┐ ┌────▼─────┐ ┌────▼─────┐ ┌─────▼─────┐  │
│  │symphonia │ │  wgpu    │ │  rodio   │ │  hayro    │  │
│  │+ rav1d   │ │+ rend3   │ │+symphonia│ │(pure Rust)│  │
│  │+openh264 │ │+ glam    │ │+ cpal    │ │           │  │
│  └──────────┘ └──────────┘ └──────────┘ └───────────┘  │
│                                                         │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐                │
│  │  Doc     │ │  LaTeX   │ │  Chart   │                │
│  │ Element  │ │ Element  │ │ Element  │                │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘                │
│  ┌────▼─────┐ ┌────▼─────┐ ┌────▼─────┐                │
│  │ docx-rs  │ │  typst + │ │ plotters │                │
│  │+calamine │ │  typst-  │ │ -bitmap  │                │
│  │          │ │  render  │ │          │                │
│  └──────────┘ └──────────┘ └──────────┘                │
│                                                         │
│  ALL → render to RGBA bitmap → GPUI paints to screen    │
├─────────────────────────────────────────────────────────┤
│  macOS: Metal  │  Linux: Vulkan  │  Windows: DX12       │
└─────────────────────────────────────────────────────────┘
```

---

## ✅ Dependency Checklist — "Does the user install anything?"

| Component | Crate | External Dep? | User Action? |
|---|---|---|---|
| Video (AV1) | `rav1d` | ❌ Pure Rust | **None** |
| Video (H.264) | `openh264` | Auto-downloads static lib at **build time** | **None** |
| Video (Demux) | `symphonia` | ❌ Pure Rust | **None** |
| 3D | `wgpu` | ❌ Pure Rust (talks to OS GPU drivers) | **None** |
| Audio | `rodio` + `cpal` | ❌ Pure Rust (talks to OS audio) | **None** |
| PDF Render | `hayro` | ❌ Pure Rust | **None** |
| PDF Generate | `genpdf` | ❌ Pure Rust | **None** |
| Docs | `docx-rs` | ❌ Pure Rust | **None** |
| LaTeX/Typeset | `typst` + `typst-render` | ❌ Pure Rust | **None** |
| Charts | `plotters` | ❌ Pure Rust | **None** |
| SVG | `resvg` | ❌ Pure Rust | **None** |

**Result: `cargo build --release` → one binary → ship it. Done.** 🎉
