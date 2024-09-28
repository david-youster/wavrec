use winapi::WasapiLoopbackRecorder;

mod winapi;

/// Captures the device audio output.
#[cfg(target_os = "windows")]
pub type LoopbackRecorder = WasapiLoopbackRecorder;
