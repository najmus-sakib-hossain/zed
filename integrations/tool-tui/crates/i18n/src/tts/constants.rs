//! TTS constants

pub const GOOGLE_TTS_URL: &str =
    "https://translate.google.com/_/TranslateWebserverUi/data/batchexecute";
pub const GOOGLE_TTS_MAX_CHARS: usize = 100;
pub const GOOGLE_TTS_RPC: &str = "jQ1olc";

pub const EDGE_TTS_BASE_URL: &str = "speech.platform.bing.com";
pub const EDGE_TTS_WSS_URL: &str =
    "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1";
pub const EDGE_TTS_VOICE_LIST_URL: &str =
    "https://speech.platform.bing.com/consumer/speech/synthesize/readaloud/voices/list";
pub const EDGE_TTS_TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
pub const EDGE_TTS_CHROMIUM_VERSION: &str = "140.0.3485.14";

pub const DEFAULT_VOICE: &str = "en-US-AriaNeural";
