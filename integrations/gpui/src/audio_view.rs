//! 🔊 Audio Player Component
//!
//! Strategy: `rodio` + `symphonia` for decode & playback. UI controls via GPUI.
//! Talks directly to OS-native audio APIs — CoreAudio / WASAPI / ALSA.

use gpui::*;
use gpui_component::StyledExt;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::io::{BufReader, Cursor};
use std::sync::{Arc, Mutex};

/// Remote-fetch lifecycle states
#[derive(Clone, PartialEq)]
enum FetchStatus {
    Loading,
    Ready,
    Error(String),
}

pub struct AudioPlayerView {
    sink: Option<Arc<Sink>>,
    _stream: Option<OutputStream>,
    _stream_handle: Option<OutputStreamHandle>,
    file_path: String,
    is_playing: bool,
    volume: f32,
    track_name: String,
    /// Downloaded audio bytes (None = not yet fetched)
    audio_data: Arc<Mutex<Option<Vec<u8>>>>,
    fetch_status: FetchStatus,
}

impl AudioPlayerView {
    pub fn new(file_path: &str, _window: &Window, cx: &mut Context<Self>) -> Self {
        let track_name = std::path::Path::new(file_path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".into());

        let audio_data: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
        let data_clone = audio_data.clone();
        let src = file_path.to_string();

        // Fetch audio bytes in background (URL download or file read)
        cx.spawn(async move |this: WeakEntity<AudioPlayerView>, cx| {
            let result = smol::unblock(move || crate::remote::resolve(&src)).await;
            match result {
                Ok(bytes) if !bytes.is_empty() => {
                    *data_clone.lock().unwrap() = Some(bytes);
                    cx.update(|cx| {
                        this.update(cx, |view, cx: &mut Context<AudioPlayerView>| {
                            view.fetch_status = FetchStatus::Ready;
                            cx.notify();
                        })
                    }).ok();
                }
                Ok(_) => {
                    cx.update(|cx| {
                        this.update(cx, |view, cx: &mut Context<AudioPlayerView>| {
                            view.fetch_status = FetchStatus::Error(
                                "Empty response — check the URL or file path".into(),
                            );
                            cx.notify();
                        })
                    }).ok();
                }
                Err(e) => {
                    cx.update(|cx| {
                        this.update(cx, |view, cx: &mut Context<AudioPlayerView>| {
                            view.fetch_status = FetchStatus::Error(e.to_string());
                            cx.notify();
                        })
                    }).ok();
                }
            }
        })
        .detach();

        Self {
            sink: None,
            _stream: None,
            _stream_handle: None,
            file_path: file_path.to_string(),
            is_playing: false,
            volume: 0.75,
            track_name,
            audio_data,
            fetch_status: FetchStatus::Loading,
        }
    }

    fn play(&mut self, cx: &mut Context<Self>) {
        if self.sink.is_some() {
            // Resume paused sink
            if let Some(ref sink) = self.sink {
                sink.play();
            }
            self.is_playing = true;
            cx.notify();
            return;
        }

        // Acquire audio bytes — prefer downloaded data, fall back to local file
        let maybe_bytes: Option<Vec<u8>> = {
            let guard = self.audio_data.lock().unwrap();
            guard.clone()
        };
        let audio_bytes = maybe_bytes.or_else(|| {
            if self.file_path.is_empty() { None }
            else { std::fs::read(&self.file_path).ok() }
        });

        if let Some(bytes) = audio_bytes
            && let Ok((stream, stream_handle)) = OutputStream::try_default()
            && let Ok(sink) = Sink::try_new(&stream_handle)
            && let Ok(source) = Decoder::new(BufReader::new(Cursor::new(bytes)))
        {
            sink.set_volume(self.volume);
            sink.append(source);
            self.sink       = Some(Arc::new(sink));
            self._stream    = Some(stream);
            self._stream_handle = Some(stream_handle);
            self.is_playing = true;
        }
        cx.notify();
    }

    fn pause(&mut self, cx: &mut Context<Self>) {
        if let Some(ref sink) = self.sink {
            sink.pause();
        }
        self.is_playing = false;
        cx.notify();
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        if let Some(ref sink) = self.sink {
            sink.stop();
        }
        self.sink = None;
        self._stream = None;
        self._stream_handle = None;
        self.is_playing = false;
        cx.notify();
    }

    fn set_volume(&mut self, vol: f32, cx: &mut Context<Self>) {
        self.volume = vol.clamp(0.0, 1.0);
        if let Some(ref sink) = self.sink {
            sink.set_volume(self.volume);
        }
        cx.notify();
    }
}

impl Render for AudioPlayerView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let vol_pct = (self.volume * 100.0) as u32;
        let is_playing = self.is_playing;
        let ready = self.fetch_status == FetchStatus::Ready;
        let play_pause = cx.listener(|this, _: &MouseDownEvent, _, cx| {
            if this.fetch_status != FetchStatus::Ready { return; }
            if this.is_playing { this.pause(cx); } else { this.play(cx); }
        });
        let stop = cx.listener(|this, _: &MouseDownEvent, _, cx| this.stop(cx));
        let vol_up = cx.listener(|this, _: &MouseDownEvent, _, cx| this.set_volume(this.volume + 0.1, cx));
        let vol_dn = cx.listener(|this, _: &MouseDownEvent, _, cx| this.set_volume(this.volume - 0.1, cx));

        // Status banner
        let (status_text, status_color): (String, Hsla) = match &self.fetch_status {
            FetchStatus::Loading => ("⏳ Fetching audio…".into(), rgb(0x89b4fa).into()),
            FetchStatus::Ready   => ("✅ Ready".into(),            rgb(0xa6e3a1).into()),
            FetchStatus::Error(e) => (format!("❌ {e}"),           rgb(0xf38ba8).into()),
        };

        div()
            .v_flex()
            .gap_4()
            .p_6()
            .w(px(440.))
            .bg(rgb(0x1e1e2e))
            .rounded(px(12.))
            .shadow_lg()
            .child(div().text_color(rgb(0xcdd6f4)).text_xl().child("🔊 Audio Player"))
            // Status row
            .child(div().text_color(status_color).text_sm().child(status_text))
            // Track info
            .child(
                div()
                    .v_flex()
                    .gap_1()
                    .child(div().text_color(rgb(0xcdd6f4)).child(format!("🎵 {}", self.track_name)))
                    .child(div().text_color(rgb(0x6c7086)).text_sm().child(self.file_path.clone())),
            )
            // Playback controls
            .child(
                div()
                    .h_flex()
                    .gap_2()
                    .justify_center()
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .bg(if !ready {
                                rgb(0x313244)           // greyed out while loading
                            } else if is_playing {
                                rgb(0xf9e2af)
                            } else {
                                rgb(0xa6e3a1)
                            })
                            .text_color(rgb(0x1e1e2e))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .child(if !ready {
                                "⏳ Loading…"
                            } else if is_playing {
                                "⏸ Pause"
                            } else {
                                "▶ Play"
                            })
                            .on_mouse_down(MouseButton::Left, play_pause),
                    )
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .bg(rgb(0xf38ba8))
                            .text_color(rgb(0x1e1e2e))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .child("⏹ Stop")
                            .on_mouse_down(MouseButton::Left, stop),
                    ),
            )
            // Volume
            .child(
                div()
                    .h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div().text_color(rgb(0xa6adc8)).child("🔉")
                            .on_mouse_down(MouseButton::Left, vol_dn),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h(px(8.))
                            .bg(rgb(0x313244))
                            .rounded(px(4.))
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(self.volume))
                                    .bg(rgb(0x89b4fa))
                                    .rounded(px(4.)),
                            ),
                    )
                    .child(
                        div()
                            .text_color(rgb(0xa6adc8))
                            .text_sm()
                            .child(format!("{}%", vol_pct))
                            .on_mouse_down(MouseButton::Left, vol_up),
                    ),
            )
    }
}
