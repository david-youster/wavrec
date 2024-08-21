use winapi::WasapiLoopbackRecorder;

mod winapi;

#[cfg(target_os = "windows")]
pub type LoopbackRecorder = WasapiLoopbackRecorder;
