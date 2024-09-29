# WAV Recorder

Console application that allows capturing the audio output from a device and
writing to WAV file.

Windows only for now. Uses [wasapi-rs](https://github.com/HEnquist/wasapi-rs)
for audio capture.

## Basic Usage

### From Source
1. Install Rust and Cargo: https://www.rust-lang.org/tools/install
(the application was created with stable Rust `1.81.0`)
2. Checkout the repository: `git clone git@github.com:david-youster/wavrec.git`
3. Run via `cargo`: `cargo run -- somefilename.wav`

See additional options using `cargo run -- -h`.