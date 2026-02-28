# Zed Media Rendering with GPUI

This document explains how Zed handles image, video, and audio rendering using GPUI.

## 📁 Folder Structure

```
integrations/zed/crates/
├── image_viewer/          # Image viewing and manipulation
│   ├── image_viewer.rs    # Main image viewer component
│   ├── image_info.rs      # Image metadata display
│   └── image_viewer_settings.rs
│
├── audio/                 # Audio playback and recording
│   ├── audio.rs           # Main audio system
│   ├── audio_settings.rs  # Audio configuration
│   ├── replays.rs         # Audio replay/recording
│   └── rodio_ext.rs       # Rodio extensions
│
├── media/                 # Low-level media bindings (macOS)
│   ├── media.rs           # CoreMedia/CoreVideo FFI
│   └── bindings.rs        # C bindings
│
├── livekit_client/        # Video streaming (LiveKit)
│   └── remote_video_track_view.rs  # Video rendering
│
└── repl/outputs/          # REPL image output
    └── image.rs           # Jupyter-style image display
```

## 🖼️ Image Rendering

### Location
`integrations/zed/crates/image_viewer/src/image_viewer.rs`

### How It Works

Zed uses GPUI's built-in `img()` element for image rendering:

```rust
// Key rendering code from image_viewer.rs
img(image)
    .id(("image-viewer-image", self.image_view.entity_id()))
    .size_full()
```

### Architecture

1. **Image Loading**
   - Images are loaded via `ImageItem` from the project
   - Stored in `image_store` with metadata (width, height, format)
   - Converted to `RenderImage` for GPUI

2. **Rendering Pipeline**
   ```rust
   ImageView
     ├── ImageContentElement (custom Element)
     │   ├── Canvas (checkerboard background)
     │   └── img() element (actual image)
     └── Zoom/Pan controls
   ```

3. **Features**
   - Zoom in/out (Ctrl+scroll or buttons)
   - Pan (drag with mouse)
   - Fit to view
   - Actual size (100%)
   - Checkerboard background for transparency

4. **GPUI Integration**
   ```rust
   // Images are rendered using GPUI's img() element
   use gpui::img;
   
   img(image_data)  // Takes Arc<RenderImage> or ImageSource
       .size_full()
       .into_any_element()
   ```

### Image Data Flow

```
File System → ImageItem → Image (asset) → RenderImage → img() → GPU
```

## 🎥 Video Rendering

### Location
`integrations/zed/crates/livekit_client/src/remote_video_track_view.rs`

### How It Works

Video rendering differs by platform:

#### macOS (Metal-based)
```rust
// Uses GPUI's surface() for hardware-accelerated rendering
gpui::surface(latest_frame.clone())
    .size_full()
    .into_any_element()
```

#### Other Platforms
```rust
// Uses img() element with frame updates
gpui::img(latest_frame.clone())
    .size_full()
```

### Architecture

1. **Video Track Management**
   - `RemoteVideoTrack` - Represents a video stream
   - `RemoteVideoTrackView` - GPUI component for rendering
   - Frames arrive via async stream

2. **Frame Pipeline**
   ```rust
   LiveKit Stream → RemoteVideoFrame → surface()/img() → Display
   ```

3. **Frame Management**
   - Frames are updated asynchronously
   - Old frames are dropped to prevent memory leaks
   - Uses `cx.notify()` to trigger re-renders

4. **Platform-Specific Rendering**
   - **macOS**: Uses Metal surfaces via `gpui::surface()`
   - **Other**: Uses `gpui::img()` with frame updates

### Video Integration

```rust
pub struct RemoteVideoTrackView {
    track: RemoteVideoTrack,
    latest_frame: Option<RemoteVideoFrame>,
    _maintain_frame: Task<()>,  // Async frame updates
}

impl Render for RemoteVideoTrackView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(target_os = "macos")]
        if let Some(frame) = &self.latest_frame {
            return gpui::surface(frame.clone()).size_full();
        }
        
        #[cfg(not(target_os = "macos"))]
        if let Some(frame) = &self.latest_frame {
            return gpui::img(frame.clone()).size_full();
        }
        
        Empty
    }
}
```

## 🔊 Audio Playback

### Location
`integrations/zed/crates/audio/src/audio.rs`

### How It Works

Audio uses the `rodio` crate (not GPUI-based):

```rust
// Audio is NOT rendered visually - it's played through speakers
pub struct Audio {
    output_handle: Option<MixerDeviceSink>,  // Rodio mixer
    source_cache: HashMap<Sound, Buffered<Decoder<Cursor<Vec<u8>>>>>,
}
```

### Architecture

1. **Audio System**
   - Uses `rodio` for audio playback
   - `cpal` for device management
   - WebRTC APM for echo cancellation (VoIP)

2. **Sound Effects**
   ```rust
   pub enum Sound {
       Joined,
       GuestJoined,
       Leave,
       Mute,
       Unmute,
       StartScreenshare,
       StopScreenshare,
       AgentDone,
   }
   ```

3. **VoIP Audio**
   - Microphone input processing
   - Echo cancellation
   - Automatic gain control
   - Denoising
   - Speaker output

4. **Audio Pipeline**
   ```
   Microphone → WebRTC APM → Denoise → AGC → Network
   Network → Decode → AGC → Mixer → Speakers
   ```

### Audio Features

- **Sound Effects**: WAV files loaded from assets
- **VoIP**: Real-time audio streaming with processing
- **Recording**: 30-second replay buffer for debugging
- **Device Selection**: Input/output device management

## 🎨 GPUI Media Primitives

### Core Elements

1. **`img()`** - Static/animated images
   ```rust
   img(image_source)
       .size(px(100.0), px(100.0))
       .object_fit(ObjectFit::Cover)
   ```

2. **`surface()`** - Hardware-accelerated video (macOS)
   ```rust
   surface(video_frame)
       .size_full()
   ```

3. **`canvas()`** - Custom drawing
   ```rust
   canvas(
       |bounds, _, _| {},  // Layout
       |bounds, _, window, _| {  // Paint
           window.paint_quad(fill(bounds, color));
       }
   )
   ```

### Image Sources

```rust
pub enum ImageSource {
    Render(Arc<RenderImage>),  // Pre-loaded image
    Uri(SharedUri),            // URL to load
    File(Arc<Path>),           // File path
}
```

### RenderImage

```rust
pub struct RenderImage {
    id: ImageId,
    data: SmallVec<[Frame; 1]>,  // Supports animated images
}

pub struct Frame {
    data: Arc<[u8]>,
    format: ImageFormat,
}
```

## 📊 Media Crate (macOS Only)

### Location
`integrations/zed/crates/media/src/media.rs`

### Purpose

Provides FFI bindings to macOS frameworks:

1. **CoreMedia** - Video frame handling
   - `CMSampleBuffer` - Video samples
   - `CMFormatDescription` - Video format info
   - `CMBlockBuffer` - Raw data access

2. **CoreVideo** - GPU texture integration
   - `CVMetalTextureCache` - Metal texture caching
   - `CVMetalTexture` - Metal textures from video
   - `CVImageBuffer` - Video frame buffers

### Usage

Used by LiveKit for hardware-accelerated video on macOS:

```rust
// Convert video frame to Metal texture
let texture_cache = CVMetalTextureCache::new(metal_device)?;
let texture = texture_cache.create_texture_from_image(
    video_frame,
    pixel_format,
    width,
    height,
    plane_index
)?;
```

## 🎯 Key Takeaways

### For Image Rendering
- Use `gpui::img()` element
- Load images as `RenderImage`
- GPUI handles GPU upload and caching
- Supports static and animated images

### For Video Rendering
- **macOS**: Use `gpui::surface()` for Metal acceleration
- **Other**: Use `gpui::img()` with frame updates
- Update frames asynchronously via `cx.notify()`
- Drop old frames to prevent memory leaks

### For Audio
- Audio is NOT rendered in GPUI
- Use `rodio` for playback
- Use `cpal` for device management
- WebRTC APM for VoIP processing

## 📝 Example: Simple Image Viewer

```rust
use gpui::*;

struct SimpleImageViewer {
    image: Arc<RenderImage>,
}

impl Render for SimpleImageViewer {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .child(
                img(self.image.clone())
                    .size_full()
                    .object_fit(ObjectFit::Contain)
            )
    }
}
```

## 📝 Example: Simple Video Player

```rust
use gpui::*;

struct SimpleVideoPlayer {
    current_frame: Option<RemoteVideoFrame>,
}

impl Render for SimpleVideoPlayer {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        if let Some(frame) = &self.current_frame {
            #[cfg(target_os = "macos")]
            return surface(frame.clone()).size_full();
            
            #[cfg(not(target_os = "macos"))]
            return img(frame.clone()).size_full();
        }
        
        div().size_full().bg(rgb(0x000000))
    }
}
```

## 🔗 Related Files

- `gpui/src/elements/img.rs` - Image element implementation
- `gpui/src/assets.rs` - RenderImage and asset loading
- `gpui/src/platform.rs` - Platform-specific rendering
- `project/src/image_store.rs` - Image caching and management
