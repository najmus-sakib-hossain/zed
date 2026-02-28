# DX Platform Apps: Universal Presence

> **"One Binary. Every Platform. Zero Compromise."**

DX extends beyond the terminal with native apps for every major operating system, bringing voice wake, visual canvas, and system-level integration to all platforms.

---

## Platform Overview

| Platform | Features | Build Status | Notes |
|----------|----------|--------------|-------|
| **macOS** | Menu bar app + Voice Wake + Canvas | 🟡 Planned | Requires Apple Developer cert |
| **iOS** | Canvas + Camera + Voice Wake | 🟡 Planned | Requires Xcode + iOS device |
| **Android** | Canvas + Camera + Screen | 🟢 Buildable | Windows can build via Android Studio |
| **Windows** | WSL2 + Native CLI | 🟢 Buildable | Primary development platform |
| **Linux** | Native support + systemd | 🟢 Buildable | Full feature support |

---

## 1. macOS App

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    DX macOS Menu Bar App                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │   Menu Bar      │  │   Voice Wake    │  │   Talk Mode     │ │
│  │   Control       │  │   (Always-On)   │  │   Overlay       │ │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘ │
│           │                    │                    │           │
│           └────────────────────┼────────────────────┘           │
│                                │                                │
│                    ┌───────────┴───────────┐                   │
│                    │   DX Gateway (WS)     │                   │
│                    │   ws://127.0.0.1:8789 │                   │
│                    └───────────────────────┘                   │
│                                                                 │
│  Features:                                                      │
│  • Menu bar icon with status indicator                         │
│  • Voice Wake activation ("Hey DX")                            │
│  • Talk Mode overlay for continuous conversation               │
│  • WebChat embedded view                                       │
│  • System notifications integration                            │
│  • Canvas host for visual workspace                            │
│  • Remote gateway control                                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Features

#### Menu Bar Control
- **Status Indicator**: Visual feedback of connection state
- **Quick Actions**: Start/stop gateway, open canvas, toggle voice
- **Settings Access**: Direct link to config files
- **Debug Tools**: Logs viewer, connection inspector

#### Voice Wake System
- **Wake Word**: Configurable trigger phrase ("Hey DX")
- **Local Processing**: Uses Whisper.cpp for privacy
- **Low Latency**: < 200ms wake detection
- **Power Efficient**: Optimized for background operation

#### Talk Mode Overlay
- **Floating Panel**: Non-intrusive conversation UI
- **Voice Feedback**: ElevenLabs or local TTS
- **Waveform Display**: Real-time audio visualization
- **Quick Dismiss**: Escape or click outside

### Configuration

```sr
# ~/.dx/config/macos.sr

[macos]
menu_bar_icon = "dx"  # or custom icon path
start_at_login = true
show_dock_icon = false

[macos.voice_wake]
enabled = true
wake_word = "hey dx"
sensitivity = 0.7
model = "whisper-tiny"  # tiny, base, small
device = "default"  # or specific audio device

[macos.talk_mode]
enabled = true
tts_provider = "elevenlabs"  # elevenlabs, system, local
voice_id = "pMsXgVXv3BLzUgSXRplE"
overlay_position = "top-right"
auto_dismiss_seconds = 30

[macos.canvas]
enabled = true
default_size = [800, 600]
transparent_background = false
always_on_top = false

[macos.notifications]
enabled = true
sound = true
badge_count = true
```

### Build Requirements

```bash
# Requirements
- macOS 13+ (Ventura or later)
- Xcode 15+
- Apple Developer Certificate (for signing)
- Rust toolchain with aarch64-apple-darwin target

# Build commands
cd apps/macos
swift build -c release
./scripts/package-mac-app.sh

# Code signing
SIGN_IDENTITY="Developer ID Application" ./scripts/codesign-mac-app.sh
```

---

## 2. iOS App

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                       DX iOS Node App                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌───────────┐ │
│  │   Canvas    │ │   Camera    │ │ Voice Wake  │ │ Talk Mode │ │
│  │  (SwiftUI)  │ │  (AVFound.) │ │  (Speech)   │ │ (Overlay) │ │
│  └──────┬──────┘ └──────┬──────┘ └──────┬──────┘ └─────┬─────┘ │
│         │               │               │               │       │
│         └───────────────┼───────────────┼───────────────┘       │
│                         │               │                       │
│                    ┌────┴───────────────┴────┐                  │
│                    │   Gateway Connection    │                  │
│                    │   (WebSocket + Bonjour) │                  │
│                    └─────────────────────────┘                  │
│                                                                 │
│  Node Capabilities:                                             │
│  • camera.snap - Capture photos                                │
│  • camera.clip - Record video clips                            │
│  • canvas.render - Display visual content                      │
│  • canvas.snapshot - Capture canvas state                      │
│  • voice.listen - Activate voice input                         │
│  • location.get - GPS coordinates                              │
│  • notify.send - Push notifications                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Features

#### Canvas (Visual Workspace)
- **A2UI Support**: Agent-to-UI rendering protocol
- **Interactive Elements**: Touch-responsive components
- **Real-time Updates**: WebSocket-driven state sync
- **Snapshot Export**: Capture canvas as image

#### Camera Integration
- **Photo Capture**: Quick snap with quality settings
- **Video Recording**: Clip recording with duration limits
- **Audio Option**: Include/exclude audio in clips
- **Preview Window**: Live camera feed display

#### Voice Wake
- **iOS Speech Framework**: Native speech recognition
- **Background Mode**: Works when app is backgrounded
- **Siri Integration**: Optional Siri shortcut support
- **Privacy First**: All processing on-device

### Configuration

```sr
# iOS app settings (synced from gateway)

[ios]
auto_connect = true
bonjour_discovery = true
manual_gateway = ""  # IP:port if not using discovery

[ios.canvas]
enabled = true
orientation = "auto"  # portrait, landscape, auto
safe_area_insets = true

[ios.camera]
enabled = true
default_quality = "high"  # low, medium, high
include_audio = true
max_clip_duration = 60

[ios.voice]
enabled = true
wake_word = "hey dx"
continuous_listening = false
language = "en-US"

[ios.notifications]
enabled = true
critical_alerts = false
```

### Build Requirements

```bash
# Requirements
- macOS with Xcode 15+
- iOS 16+ deployment target
- Apple Developer Account
- Physical iOS device (or simulator for testing)

# Generate Xcode project
cd apps/ios
xcodegen generate
open DX.xcodeproj

# Build via fastlane
fastlane build
fastlane deploy  # To TestFlight
```

---

## 3. Android App

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     DX Android Node App                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌───────────┐ │
│  │   Canvas    │ │   Camera    │ │   Screen    │ │ Talk Mode │ │
│  │  (Compose)  │ │  (CameraX)  │ │  (MediaProj)│ │ (Overlay) │ │
│  └──────┬──────┘ └──────┬──────┘ └──────┬──────┘ └─────┬─────┘ │
│         │               │               │               │       │
│         └───────────────┼───────────────┼───────────────┘       │
│                         │               │                       │
│                    ┌────┴───────────────┴────┐                  │
│                    │   Foreground Service    │                  │
│                    │   (Persistent Connection)│                  │
│                    └─────────────────────────┘                  │
│                         │                                       │
│                    ┌────┴────┐                                  │
│                    │ Gateway │                                  │
│                    │  (NSD)  │                                  │
│                    └─────────┘                                  │
│                                                                 │
│  Node Capabilities:                                             │
│  • camera.snap - CameraX photo capture                         │
│  • camera.clip - Video recording with audio                    │
│  • screen.record - MediaProjection screen capture              │
│  • canvas.render - Jetpack Compose UI                          │
│  • chat.send - Direct messaging                                │
│  • sms.send - Optional SMS gateway (requires permission)       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Features

#### Canvas (Jetpack Compose)
- **Modern UI**: Fully Compose-based rendering
- **Material 3**: Dynamic color theming
- **Responsive Layout**: Adapts to all screen sizes
- **Gesture Support**: Pinch, swipe, tap interactions

#### Camera (CameraX)
- **Photo Mode**: High-quality still capture
- **Video Mode**: Recording with configurable quality
- **Audio Toggle**: Include/exclude audio
- **Flash Control**: Auto, on, off modes

#### Screen Recording (MediaProjection)
- **System-level Capture**: Records entire screen
- **Audio Options**: System audio + microphone
- **Quality Settings**: Resolution and bitrate control
- **Privacy Indicator**: System-required notification

### Configuration

```sr
# Android app settings

[android]
min_sdk = 31  # Android 12+
foreground_service = true
auto_start = false

[android.gateway]
discovery_enabled = true
discovery_method = "nsd"  # Network Service Discovery
manual_host = ""
manual_port = 8789

[android.canvas]
enabled = true
theme = "system"  # light, dark, system

[android.camera]
enabled = true
default_lens = "back"  # back, front
video_quality = "1080p"
max_duration_seconds = 300

[android.screen]
enabled = true
capture_audio = true
quality = "high"
show_touches = false

[android.permissions]
camera = "required"
microphone = "required"
location = "optional"
sms = "disabled"
```

### Build Requirements

```bash
# Requirements
- Android Studio Hedgehog+
- Android SDK 34+
- Kotlin 1.9+
- JDK 17+

# Build commands
cd apps/android
./gradlew :app:assembleRelease
./gradlew :app:installRelease

# Run tests
./gradlew :app:testDebugUnitTest

# Windows can build Android apps directly!
# No macOS required for Android development
```

---

## 4. Windows (WSL2)

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    DX on Windows (WSL2)                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Windows Host                                                   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Windows Terminal / PowerShell                          │   │
│  │  • dx.exe (native Windows binary)                       │   │
│  │  • VS Code with DX Extension                            │   │
│  └─────────────────────────────────────────────────────────┘   │
│                         │                                       │
│                    ┌────┴────┐                                  │
│                    │  WSL2   │                                  │
│                    └────┬────┘                                  │
│                         │                                       │
│  WSL2 (Ubuntu/Debian)                                          │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  • dx (Linux binary) - Full feature support             │   │
│  │  • Gateway daemon (systemd user service)                │   │
│  │  • Browser control (Chromium via xdg)                   │   │
│  │  • All CLI features                                     │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Recommended Setup:                                             │
│  1. Install WSL2 with Ubuntu 24.04                             │
│  2. Install DX in WSL2                                         │
│  3. Run gateway in WSL2                                        │
│  4. Access from Windows via localhost                          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Setup Guide

```bash
# 1. Enable WSL2 (PowerShell as Admin)
wsl --install -d Ubuntu-24.04

# 2. Install DX in WSL2
wsl -d Ubuntu-24.04
curl -fsSL https://dx.dev/install.sh | sh

# 3. Start gateway
dx gateway --port 8789 --daemon

# 4. Access from Windows
# Gateway available at localhost:8789
```

### Native Windows Binary

```sr
# config/windows.sr

[windows]
use_wsl = true
wsl_distro = "Ubuntu-24.04"

[windows.native]
# Features available without WSL
shell = "powershell"
browser_control = true
file_system = true

[windows.wsl_features]
# Features requiring WSL2
gateway = true
voice = true
cron = true
```

---

## 5. Linux

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    DX on Linux (Native)                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    User Space                            │   │
│  │  ┌───────────────┐  ┌───────────────┐  ┌─────────────┐  │   │
│  │  │   DX CLI      │  │   Gateway     │  │  Browser    │  │   │
│  │  │   Binary      │  │   Daemon      │  │  (Chromium) │  │   │
│  │  └───────┬───────┘  └───────┬───────┘  └──────┬──────┘  │   │
│  │          │                  │                  │         │   │
│  │          └──────────────────┼──────────────────┘         │   │
│  │                             │                            │   │
│  │  ┌──────────────────────────┴─────────────────────────┐  │   │
│  │  │                   systemd                          │  │   │
│  │  │  • dx-gateway.service (user service)               │  │   │
│  │  │  • dx-cron.timer (scheduled tasks)                 │  │   │
│  │  │  • Automatic restart on failure                    │  │   │
│  │  └────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Supported Distributions:                                       │
│  • Ubuntu 22.04+ (Primary)                                     │
│  • Debian 12+                                                  │
│  • Fedora 38+                                                  │
│  • Arch Linux (rolling)                                        │
│  • NixOS (declarative config)                                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Installation

```bash
# Ubuntu/Debian
curl -fsSL https://dx.dev/install.sh | sh

# Arch Linux
paru -S dx-bin

# NixOS (flake)
nix profile install github:paiml/dx

# From source
git clone https://github.com/paiml/dx.git
cd dx && cargo install --path crates/cli
```

### Systemd Integration

```ini
# ~/.config/systemd/user/dx-gateway.service
[Unit]
Description=DX Gateway
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/dx gateway --port 8789
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

```bash
# Enable and start
systemctl --user enable dx-gateway
systemctl --user start dx-gateway

# Check status
systemctl --user status dx-gateway
```

### Configuration

```sr
# ~/.dx/config/linux.sr

[linux]
display_server = "auto"  # x11, wayland, auto
audio_system = "auto"    # pulseaudio, pipewire, auto
browser = "chromium"     # chromium, google-chrome, firefox

[linux.io_uring]
enabled = true  # High-performance async I/O
ring_size = 1024

[linux.systemd]
user_service = true
socket_activation = false
journal_logging = true

[linux.desktop]
notifications = true
tray_icon = false  # Requires desktop environment
```

---

## Cross-Platform Features Matrix

| Feature | macOS | iOS | Android | Windows | Linux |
|---------|-------|-----|---------|---------|-------|
| Gateway | ✅ | ❌ | ❌ | ✅ (WSL) | ✅ |
| Voice Wake | ✅ | ✅ | 🟡 | ❌ | ❌ |
| Talk Mode | ✅ | ✅ | ✅ | ❌ | ❌ |
| Canvas | ✅ | ✅ | ✅ | ❌ | ❌ |
| Camera | ❌ | ✅ | ✅ | ❌ | ❌ |
| Screen Record | ❌ | ❌ | ✅ | ❌ | ❌ |
| Browser Control | ✅ | ❌ | ❌ | ✅ | ✅ |
| Notifications | ✅ | ✅ | ✅ | ✅ | ✅ |
| Cron Jobs | ✅ | ❌ | ❌ | ✅ (WSL) | ✅ |
| Webhooks | ✅ | ❌ | ❌ | ✅ | ✅ |

---

## Gateway Connection Protocol

All platform apps connect to the DX Gateway using a unified WebSocket protocol:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Gateway Connection Flow                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. Discovery                                                    │
│  ┌─────────────────┐      ┌─────────────────┐                   │
│  │   Mobile App    │──────│   mDNS/Bonjour  │                   │
│  └─────────────────┘      └────────┬────────┘                   │
│                                    │                             │
│  2. Connection Request             │                             │
│  ┌─────────────────┐      ┌────────▼────────┐                   │
│  │   App           │──────│   Gateway       │                   │
│  │   (WS Client)   │      │   (WS Server)   │                   │
│  └─────────────────┘      └────────┬────────┘                   │
│                                    │                             │
│  3. Pairing                        │                             │
│  ┌─────────────────┐      ┌────────▼────────┐                   │
│  │   Pairing Code  │◄─────│   dx nodes      │                   │
│  │   (One-time)    │      │   approve <id>  │                   │
│  └─────────────────┘      └────────┬────────┘                   │
│                                    │                             │
│  4. Authenticated Connection       │                             │
│  ┌─────────────────┐      ┌────────▼────────┐                   │
│  │   Node Session  │◄────▶│   Gateway       │                   │
│  │   (Persistent)  │      │   (Authorized)  │                   │
│  └─────────────────┘      └─────────────────┘                   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Node Commands

```bash
# List connected nodes
dx nodes list

# List pending pairing requests
dx nodes pending

# Approve a node
dx nodes approve <request-id>

# Revoke node access
dx nodes revoke <node-id>

# Send command to node
dx nodes exec <node-id> camera.snap
```

---

## Build Automation

### CI/CD Matrix

```yaml
# .github/workflows/platforms.yml
name: Platform Builds

on:
  push:
    tags: ['v*']

jobs:
  macos:
    runs-on: macos-14
    steps:
      - uses: actions/checkout@v4
      - name: Build macOS App
        run: |
          cd apps/macos
          swift build -c release
          ./scripts/package-mac-app.sh

  ios:
    runs-on: macos-14
    steps:
      - uses: actions/checkout@v4
      - name: Build iOS App
        run: |
          cd apps/ios
          xcodegen generate
          fastlane build

  android:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build Android App
        run: |
          cd apps/android
          ./gradlew :app:assembleRelease

  linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build Linux Binary
        run: cargo build --release

  windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build Windows Binary
        run: cargo build --release
```

---

## Security Considerations

### Platform-Specific Security

| Platform | Security Model | Key Considerations |
|----------|----------------|-------------------|
| macOS | App Sandbox + Hardened Runtime | Code signing required for TCC permissions |
| iOS | App Sandbox | All network must be HTTPS, keychain for secrets |
| Android | Permission Model | Runtime permissions, foreground service notification |
| Windows | User Account Control | Admin elevation for some features |
| Linux | User Permissions | Capabilities for privileged operations |

### Best Practices

1. **Credentials**: Store in platform keychain, never in plaintext
2. **Network**: Use TLS for all gateway connections
3. **Pairing**: One-time codes with expiration
4. **Audit**: Log all node commands for review
5. **Updates**: Auto-update with signature verification
