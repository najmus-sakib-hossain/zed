# Zed (Windows) — low-resource build/run commands

```bash
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=1 CARGO_PROFILE_DEV_CODEGEN_UNITS=1 CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS=1 cargo run -p zed --locked

export CARGO_BUILD_JOBS="1"
export CARGO_INCREMENTAL="1"
export CARGO_PROFILE_DEV_CODEGEN_UNITS="1"
export CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS="1"
cargo run -p zed --locked
```

```powershell
$env:CARGO_BUILD_JOBS = "1"; $env:CARGO_INCREMENTAL = "1"; $env:CARGO_PROFILE_DEV_CODEGEN_UNITS = "1"; $env:CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS = "1"; cargo run -p zed --locked

$env:CARGO_BUILD_JOBS = "1"
$env:CARGO_INCREMENTAL = "1"
$env:CARGO_PROFILE_DEV_CODEGEN_UNITS = "1"
$env:CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS = "1"
cargo run -p zed --locked
```

This repo already supports incremental compilation, but on very low Windows devices you often need to *trade speed for reliability* by reducing parallelism and peak memory use.

These commands are designed to:
- Build only the Zed app crate (`-p zed`) instead of the whole workspace.
- Reuse incremental artifacts so future `cargo run` builds only recompile changed crates.
- Reduce peak RAM usage (slower, but less likely to fail).

## Important Windows notes (avoid common build failures)

- Run these in **“Developer PowerShell for VS”** (or otherwise ensure MSVC env vars are set), especially if you installed *Build Tools only*.
- Do **not** set `RUSTFLAGS` in your environment. This repo relies on `.cargo/config.toml` rustflags; setting `RUSTFLAGS` overrides them and can break builds.
- If you hit “path too long” errors, enable long paths:
  - Git: `git config --system core.longpaths true`
  - Windows (admin PowerShell), then reboot:
    - `New-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" -Name "LongPathsEnabled" -Value 1 -PropertyType DWORD -Force`

## One-time (recommended): stable, short target directory

Using a short, stable target directory helps both incremental builds and Windows path-length limits.

PowerShell:

```powershell
# Pick a short path on a drive with enough free space.
# (Change this if F: is not appropriate on your machine.)
$env:CARGO_TARGET_DIR = "F:\zed-target"
```

Keep the same `CARGO_TARGET_DIR` every time you build/run to avoid needless rebuilds.

## One-time (optional but very helpful): keep Cargo’s download cache stable

If you see Cargo “downloading everything again” after closing/reopening your terminal, it usually means **your Cargo cache or target directory is not being reused**.

Cargo caches downloads in `CARGO_HOME` (defaults to `%USERPROFILE%\.cargo`). If that directory is being cleaned (disk cleanup tools, corporate policies, roaming profiles, etc.), you can move it to a stable location with more space:

```powershell
# Optional: move Cargo’s registry + git dependency caches to a stable location.
# Only do this if you have a reason (e.g. your default %USERPROFILE%\.cargo gets cleaned).
$env:CARGO_HOME = "F:\cargo-home"
```

After setting `CARGO_HOME`, run a fetch once (online) so later builds can be offline:

```powershell
cargo fetch --locked
```

## Make the settings persist across terminal restarts

PowerShell environment variables like `$env:CARGO_TARGET_DIR = ...` apply only to the **current** shell. If you close the window, the next session won’t have them unless you persist them.

Recommended (PowerShell profile):

```powershell
# Opens your PowerShell profile in Notepad (create it if it doesn’t exist)
notepad $PROFILE
```

Add lines like these to the profile (adjust paths):

```powershell
$env:CARGO_TARGET_DIR = "F:\zed-target"
$env:CARGO_BUILD_JOBS = "1"
$env:CARGO_INCREMENTAL = "1"
$env:CARGO_PROFILE_DEV_CODEGEN_UNITS = "1"
$env:CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS = "1"
# Optional if you moved it:
# $env:CARGO_HOME = "F:\cargo-home"
```

## Slow-but-reliable debug build (single job, low RAM)

```powershell
$env:CARGO_BUILD_JOBS = "1"                    # compile one crate at a time
$env:CARGO_INCREMENTAL = "1"                   # keep incremental artifacts (also enabled in this repo)
$env:CARGO_PROFILE_DEV_CODEGEN_UNITS = "1"     # lower peak RAM (slower)
$env:CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS = "1"

cargo build -p zed --locked
```

## Slow-but-reliable `cargo run` (incremental; only rebuilds what changed)

This uses the same low-resource settings, and runs only the `zed` package. On subsequent runs, Cargo will only rebuild crates that changed (plus any downstream dependencies), not the whole editor from scratch.

```powershell
$env:CARGO_BUILD_JOBS = "1"
$env:CARGO_INCREMENTAL = "1"
$env:CARGO_PROFILE_DEV_CODEGEN_UNITS = "1"
$env:CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS = "1"
cargo run -p zed --locked
```

### If you want *zero* downloads during builds

After you’ve successfully run `cargo fetch --locked` once, you can force “no network” mode:

```powershell
# `--frozen` == `--locked` + `--offline`
cargo run -p zed --frozen
```

If this fails with “missing package”, run `cargo fetch --locked` again (online). The lockfile changing (e.g. switching branches/commits) can require new downloads.

### If you just want Zed to start immediately (no build step)

If you made **no Rust code changes** since the last successful build, you don’t need Cargo at all—run the already-built EXE:

```powershell
# If you set CARGO_TARGET_DIR:
& "$env:CARGO_TARGET_DIR\debug\zed.exe"

# Otherwise, from the repo root:
& ".\target\debug\zed.exe"
```

If it fails to start due to missing DLLs, fall back to `cargo run -p zed --locked` (Cargo sets up some runtime environment variables like `PATH`).

## Optional: release build (much slower to link; not ideal on low devices)

If you need `--release`, keep jobs at 1 to reduce memory spikes:

```powershell
$env:CARGO_BUILD_JOBS = "1"

cargo build -p zed --release --locked
```

## If the build fails selecting `rc.exe`

Some Windows setups need an explicit RC toolkit path (see Zed’s Windows build troubleshooting). Typical value:

```powershell
$env:ZED_RC_TOOLKIT_PATH = "C:\Program Files (x86)\Windows Kits\10\bin\<SDK_version>\x64"
```

Then retry the build/run commands above.

## If you hit `LINK : fatal error LNK1201` writing `zed.pdb`

This is usually a disk-space or file-lock issue in your target directory.

Use these steps in PowerShell:

```powershell
# 1) Check free space (you generally want many GB free for debug linking)
Get-PSDrive -Name F,C | Select-Object Name,@{Name='FreeGB';Expression={[math]::Round($_.Free/1GB,2)}}

# 2) Remove stale zed PDB if present
Remove-Item "F:\Desktop\target\debug\deps\zed.pdb" -Force -ErrorAction SilentlyContinue

# 3) Move build output to a roomier drive (recommended)
$env:CARGO_TARGET_DIR = "C:\zed-target"

# 4) Retry low-resource build
$env:CARGO_BUILD_JOBS = "1"
$env:CARGO_INCREMENTAL = "1"
$env:CARGO_PROFILE_DEV_CODEGEN_UNITS = "1"
$env:CARGO_PROFILE_DEV_BUILD_OVERRIDE_CODEGEN_UNITS = "1"
cargo run -p zed --locked
```

If this still fails, close any running `zed.exe` or `link.exe`/`mspdbsrv.exe` process and retry.

Current - Status:
```markdown
Based on a deep dive into the local gpui codebase (specifically gpui and gpui_wgpu) and the current state of the Zed editor's rendering architecture, here is the brutal truth.

GPUI is a highly optimized, immediate-mode-like UI framework built on top of WGPU. It is designed for extreme performance (120fps) and low latency, which means it currently only implements the rendering primitives it strictly needs for the Zed editor. It is not a general-purpose web browser or a fully-featured CSS engine.

Adding all the features you mentioned is possible, but it will require writing custom WGSL shaders, modifying the core rendering pipeline (gpui_wgpu), and extending the styling system (style.rs).

Here is the breakdown of each feature, its feasibility, and an estimated timeline if you were to build it yourself:

1. Gradients
Linear Gradient: ✅ Already Supported! GPUI has BackgroundTag::LinearGradient and a corresponding shader in shaders.wgsl.
Radial Gradient: ❌ Not Supported.
Feasibility: High. You would need to add BackgroundTag::RadialGradient, update the Background struct in color.rs, and write a new WGSL shader function to calculate the radial distance and interpolate color stops.
Time Estimate: 1-2 weeks for a solid implementation.
Mesh Gradient: ❌ Not Supported.
Feasibility: Medium. This is much more complex. You would need a new rendering primitive (a custom mesh with vertex colors) and a custom shader to interpolate between multiple points.
Time Estimate: 3-4 weeks.
Multiple Gradients at once: ❌ Not Supported.
Feasibility: Medium. Currently, the Background struct only accepts a single BackgroundTag. You would need to refactor Background to accept an array of backgrounds (like CSS) and update the rendering loop to composite them. Alternatively, you can stack multiple divs with different gradients and opacities as a workaround.
Time Estimate: 1-2 weeks to refactor the core styling and rendering structs.
2. Effects
Outer Glow: ⚠️ Supported (Workaround). You can simulate this using box_shadow with a 0 offset, a large blur_radius, and a bright color.
Inner Glow: ❌ Not Supported.
Feasibility: High. You would need to add an inset flag to the BoxShadow struct in style.rs and update the shadow shader in shaders.wgsl to render inside the bounds instead of outside.
Time Estimate: 1 week.
Blur Effect (Backdrop Blur / Element Blur): ❌ Not Supported.
Feasibility: Low/Very Hard. GPUI currently only supports blur for shadows. Adding backdrop-filter: blur requires a multi-pass rendering pipeline (rendering the scene behind the element to a texture, blurring it, and then rendering the element on top). This is a significant architectural change to gpui_wgpu.
Time Estimate: 1-2 months.
3. SVG Animations
❌ Not Supported.
Feasibility: Very Low. GPUI uses the resvg and usvg crates for SVG rendering (see svg_renderer.rs). These libraries explicitly do not support SVG animations (SMIL or CSS inside SVG). To support this, you would either need to write a custom SVG animation engine or find a Rust library that supports it (which currently doesn't exist at production quality).
Time Estimate: Months (practically a massive undertaking).
4. UI Animations (Fade in, Fade out, Slide, etc.)
Fade in / Fade out: ✅ Already Supported! GPUI has an AnimationExt trait (with_animation in animation.rs) that lets you animate any property over time using an easing function, including opacity or background color.
Slide in / Scale: ⚠️ Supported (with caveats). You can animate padding, margins, or bounds. However, GPUI does not have a native transform matrix for divs (only for SVGs and paths), so you can't just apply a CSS-like transform: scale(0.9). You have to animate the actual layout properties.
Blur in: ❌ Not Supported. Because element blur is not supported, you cannot animate it.
The Verdict
If you want to build a Shadcn-like UI library on top of GPUI, you will have a great time with the layout, text, and basic animations (fade/slide). However, for advanced visual effects (mesh gradients, backdrop blurs, SVG animations), you will hit a wall because GPUI is not a web browser. You will have to get your hands dirty with Rust and WGSL shaders to build these features yourself.

If you are willing to dive into WGPU and WGSL, adding radial gradients and inner glows is a great first step and very achievable within a few weeks!
```
