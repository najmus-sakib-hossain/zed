//! 🎬 VIDEO Player Component
//!
//! Strategy: `mp4` crate demuxes the container (with proper AVC config / SPS+PPS extraction)
//! → `openh264` decodes H.264 frames → RGBA → GPUI img.
//!
//! Root-cause of previous failure: symphonia's MP4 demuxer does NOT populate
//! `CodecParameters::extra_data` for video tracks, so SPS+PPS were never sent to
//! the decoder, causing OpenH264 error `dsNoParamSets` (native=16) on every packet.
//! The `mp4` crate exposes `track.sequence_parameter_set()` / `picture_parameter_set()`
//! directly, giving us the SPS+PPS we need to prime the decoder.

use gpui::*;
use gpui_component::StyledExt;
use openh264::formats::YUVSource;
use image::RgbaImage;
use openh264::decoder::Decoder;
use std::io::{BufReader, Cursor};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::bitmap_utils;

// Max frames kept in memory at once (~10 s at 30 fps).  Prevents OOM for long
// remote videos while still giving a usable scrubbing window.
const MAX_FRAMES: usize = 300;

/// Holds decoded video frames and playback state
pub struct VideoPlayerView {
    frames: Arc<Mutex<Vec<RgbaImage>>>,
    current_frame: usize,
    fps: f64,
    is_playing: bool,
    file_path: String,
    /// None = still loading, Some(Ok(())) = loaded OK, Some(Err(msg)) = failed
    status: Arc<Mutex<Option<Result<(), String>>>>,
}

impl VideoPlayerView {
    pub fn new(file_path: &str, _window: &Window, cx: &mut Context<Self>) -> Self {
        let frames = Arc::new(Mutex::new(Vec::<RgbaImage>::new()));
        let frames_clone = frames.clone();
        let path = file_path.to_string();
        let status: Arc<Mutex<Option<Result<(), String>>>> = Arc::new(Mutex::new(None));
        let status_clone = status.clone();

        cx.spawn(async move |this: WeakEntity<VideoPlayerView>, cx| {
            let result = smol::unblock(move || {
                let data = crate::remote::resolve(&path)?;
                if data.is_empty() {
                    return Err(anyhow::anyhow!("Empty response — check the URL or file path"));
                }
                decode_h264_video(data)
            }).await;

            match result {
                Ok(decoded_frames) => {
                    let count = decoded_frames.len();
                    *frames_clone.lock().unwrap() = decoded_frames;
                    *status_clone.lock().unwrap() = Some(Ok(()));
                    cx.update(|cx| {
                        this.update(cx, |view, cx: &mut Context<VideoPlayerView>| {
                            // Auto-start playback if frames were decoded
                            if count > 0 {
                                view.is_playing = true;
                                view.schedule_next_frame(cx);
                            }
                            cx.notify();
                        })
                    }).ok();
                }
                Err(e) => {
                    *status_clone.lock().unwrap() = Some(Err(e.to_string()));
                    cx.update(|cx| {
                        this.update(cx, |_, cx: &mut Context<VideoPlayerView>| {
                            cx.notify();
                        })
                    }).ok();
                }
            }
        })
        .detach();

        Self {
            frames,
            current_frame: 0,
            fps: 30.0,
            is_playing: false,
            file_path: file_path.to_string(),
            status,
        }
    }

    fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        self.is_playing = !self.is_playing;
        if self.is_playing {
            self.schedule_next_frame(cx);
        }
    }

    fn schedule_next_frame(&mut self, cx: &mut Context<Self>) {
        let frame_duration = Duration::from_secs_f64(1.0 / self.fps);
        cx.spawn(async move |this: WeakEntity<VideoPlayerView>, cx| {
            smol::Timer::after(frame_duration).await;
            cx.update(|cx| {
                this.update(cx, |view, cx: &mut Context<VideoPlayerView>| {
                    if view.is_playing {
                        let frame_count = view.frames.lock().unwrap().len();
                        if frame_count > 0 {
                            view.current_frame = (view.current_frame + 1) % frame_count;
                        }
                        view.schedule_next_frame(cx);
                        cx.notify();
                    }
                })
            }).ok();
        })
        .detach();
    }
}

impl Render for VideoPlayerView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let frames = self.frames.lock().unwrap();
        let status = self.status.lock().unwrap().clone();
        let toggle = cx.listener(|this, _: &MouseDownEvent, _, cx| this.toggle_playback(cx));

        // Determine status label
        let status_label: String = match &status {
            None => "⏳ Fetching & decoding…".into(),
            Some(Ok(())) if frames.is_empty() => "⚠ Decoded 0 frames — codec may be unsupported".into(),
            Some(Ok(())) => format!("✅ {} frames  |  {:.0} fps", frames.len(), self.fps),
            Some(Err(e)) => format!("❌ {}", e),
        };
        let status_color = match &status {
            Some(Err(_)) => rgb(0xf38ba8),
            Some(Ok(())) if frames.is_empty() => rgb(0xf9e2af),
            _ => rgb(0x6c7086),
        };

        div()
            .v_flex()
            .gap_3()
            .items_center()
            .p_4()
            .child(div().text_color(rgb(0xcdd6f4)).text_xl().child("🎬 Video Player"))
            .child(div().text_color(rgb(0x6c7086)).text_sm().child(self.file_path.clone()))
            .child(div().text_color(status_color).text_sm().child(status_label))
            .child(
                div()
                    .w(px(640.))
                    .h(px(360.))
                    .bg(rgb(0x000000))
                    .rounded(px(8.))
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(if let Some(frame) = frames.get(self.current_frame) {
                        let source = bitmap_utils::rgba_to_gpui_image(frame);
                        div().child(
                            img(source)
                                .w(px(640.))
                                .h(px(360.))
                                .object_fit(ObjectFit::Contain),
                        )
                    } else if status.is_none() {
                        div().text_color(rgb(0x89b4fa)).child("⏳ Loading…")
                    } else {
                        div().text_color(rgb(0xf38ba8)).child("No frame to display")
                    }),
            )
            .child(
                div()
                    .h_flex()
                    .gap_3()
                    .items_center()
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .bg(rgb(0x89b4fa))
                            .text_color(rgb(0x1e1e2e))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .child(if self.is_playing { "⏸ Pause" } else { "▶ Play" })
                            .on_mouse_down(MouseButton::Left, toggle),
                    )
                    .child(
                        div()
                            .text_color(rgb(0xa6adc8))
                            .child(format!("Frame {}/{}", self.current_frame + 1, frames.len())),
                    ),
            )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// H.264 helpers — AVCC ↔ Annex B conversion
// ─────────────────────────────────────────────────────────────────────────────

/// Re-frame AVCC packet data (length-prefixed NALUs) as Annex B (start-code
/// prefixed NALUs).  `length_size` is typically 4 (from the AVCC config record).
fn avcc_packet_to_annex_b(data: &[u8], length_size: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 8);
    let mut pos = 0;
    while pos + length_size <= data.len() {
        let nalu_len = match length_size {
            4 => u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap()) as usize,
            3 => (data[pos] as usize) << 16 | (data[pos + 1] as usize) << 8 | data[pos + 2] as usize,
            2 => u16::from_be_bytes(data[pos..pos + 2].try_into().unwrap()) as usize,
            1 => data[pos] as usize,
            _ => break,
        };
        pos += length_size;
        if nalu_len == 0 || pos + nalu_len > data.len() { break; }
        out.extend_from_slice(&[0, 0, 0, 1]);
        out.extend_from_slice(&data[pos..pos + nalu_len]);
        pos += nalu_len;
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Decode H.264 video using the `mp4` crate (demux) + openh264 (decode)
// ─────────────────────────────────────────────────────────────────────────────

fn decode_h264_video(data: Vec<u8>) -> anyhow::Result<Vec<RgbaImage>> {
    use mp4::{Mp4Reader, TrackType};

    let size = data.len() as u64;
    // BufReader<Cursor<Vec<u8>>> satisfies both Read + Seek required by Mp4Reader.
    let mut mp4 = Mp4Reader::read_header(BufReader::new(Cursor::new(data)), size)
        .map_err(|e| anyhow::anyhow!("Failed to parse MP4 container: {e}"))?;

    // ── Find the H.264 video track ──────────────────────────────────────────
    // Extract everything we need while `mp4.tracks()` is immutably borrowed,
    // then release that borrow so we can call `mp4.read_sample()` mutably.
    let (track_id, sample_count, sps_pps_annex_b) = {
        let tracks = mp4.tracks();
        let video_track = tracks
            .values()
            .find(|t| t.track_type().ok() == Some(TrackType::Video))
            .ok_or_else(|| anyhow::anyhow!("No video track found in MP4"))?;

        let id = video_track.track_id();
        let count = video_track.sample_count();

        // Build Annex B SPS+PPS by prepending a 4-byte start code to each NALU.
        // The `mp4` crate parses the avcC box and gives us the raw NALU bytes
        // without length prefix — exactly what we need.
        let mut sps_pps = Vec::<u8>::new();
        if let Ok(sps) = video_track.sequence_parameter_set() {
            sps_pps.extend_from_slice(&[0, 0, 0, 1]);
            sps_pps.extend_from_slice(sps);
        }
        if let Ok(pps) = video_track.picture_parameter_set() {
            sps_pps.extend_from_slice(&[0, 0, 0, 1]);
            sps_pps.extend_from_slice(pps);
        }

        (id, count, sps_pps)
    };

    // Standard MP4/AVC uses 4-byte NALU length prefixes (NALULengthSizeMinusOne == 3).
    let length_size: usize = 4;

    let mut h264_decoder = Decoder::new()?;
    let mut frames = Vec::new();
    let mut ok_none: usize = 0;
    let mut first_errs: Vec<String> = Vec::new();

    // Helper: convert a decoded YUV frame → RgbaImage and push into `frames`.
    macro_rules! push_yuv {
        ($yuv:expr) => {{
            let (w, h) = $yuv.dimensions();
            let mut rgb = vec![0u8; w * h * 3];
            $yuv.write_rgb8(&mut rgb);
            let mut rgba = Vec::with_capacity(w * h * 4);
            for c in rgb.chunks_exact(3) {
                rgba.push(c[0]);
                rgba.push(c[1]);
                rgba.push(c[2]);
                rgba.push(255);
            }
            if let Some(img) = RgbaImage::from_raw(w as u32, h as u32, rgba) {
                frames.push(img);
            }
        }};
    }

    // ── Prime the decoder with SPS + PPS ────────────────────────────────────
    // OpenH264 must receive the parameter sets before any IDR/P-frames.
    // Without this, every `decode()` call returns dsNoParamSets (native=16).
    if !sps_pps_annex_b.is_empty() {
        match h264_decoder.decode(&sps_pps_annex_b) {
            Ok(Some(yuv)) => push_yuv!(yuv),
            Ok(None) => {}
            Err(e) => first_errs.push(format!("SPS/PPS prime: {e:?}")),
        }
    }

    // ── Decode samples ──────────────────────────────────────────────────────
    // sample_id is 1-based per the MP4 spec.
    let cap = sample_count.min(MAX_FRAMES as u32);
    for sample_id in 1..=cap {
        if let Ok(Some(sample)) = mp4.read_sample(track_id, sample_id) {
            // Each sample is in AVCC format (4-byte length-prefixed NALUs).
            // Convert to Annex B (start-code prefixed) for OpenH264.
            let annex_b = avcc_packet_to_annex_b(&sample.bytes, length_size);
            let to_decode: &[u8] = if annex_b.is_empty() { &sample.bytes } else { &annex_b };

            match h264_decoder.decode(to_decode) {
                Ok(Some(yuv)) => push_yuv!(yuv),
                // Ok(None) is normal: H.264 decoders pipeline frames internally
                // and emit output several packets after the corresponding input.
                Ok(None) => ok_none += 1,
                Err(e) => {
                    if first_errs.len() < 5 {
                        first_errs.push(format!("{e:?}"));
                    }
                }
            }
        }
    }

    // ── Flush the decoder pipeline ──────────────────────────────────────────
    // H.264 reorders B-frames and has pipeline latency. decode(&[]) drains
    // any buffered output frames that haven't been emitted yet.
    loop {
        match h264_decoder.decode(&[]) {
            Ok(Some(yuv)) => push_yuv!(yuv),
            _ => break,
        }
    }

    if frames.is_empty() {
        return Err(anyhow::anyhow!(
            "Decoded 0 frames.\n\
             Stats: samples={sample_count}  ok_none={ok_none}  errors={}\n\
             has_sps_pps={}  length_size={length_size}\n\
             First decode errors: {:?}",
            first_errs.len(),
            !sps_pps_annex_b.is_empty(),
            first_errs,
        ));
    }

    Ok(frames)
}
