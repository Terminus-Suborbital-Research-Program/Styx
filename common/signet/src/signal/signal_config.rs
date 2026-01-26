use std::path::PathBuf;

pub struct SignalConfig {
    pub capture_output: PathBuf,

    // Amount to average to with psd bin averaging
    // or downsample by
    pub down_size: usize,
    // Amount of bins to shift the window when scanning for cross correlation
    pub search_size: usize,
}

impl SignalConfig {
    pub fn new(capture_output: PathBuf, down_size: usize, search_size: usize) -> Self {
        Self {
            capture_output,
            down_size,
            search_size,
        }
    }
}

impl Default for SignalConfig {
    fn default() -> Self {
        Self {
            capture_output: PathBuf::from("capture.iq"),
            down_size: 64,
            search_size: 100,
        }
    }
}
