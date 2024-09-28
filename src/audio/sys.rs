use winapi::WasapiLoopbackRecorder;

mod winapi;

/// Captures the device audio output.
pub type LoopbackRecorder = WasapiLoopbackRecorder;
