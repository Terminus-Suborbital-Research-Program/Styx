
pub const TARGET_PACKET_SIZE: usize = 65_536;
pub const READ_CHUNK_SIZE: usize = 4096;
pub const BUFF_SIZE: usize = TARGET_PACKET_SIZE + READ_CHUNK_SIZE;

#[derive(Clone, Copy)]
pub struct RadioConfig {
    pub frequency: f64,
    pub sample_rate: f64,
    pub gain: Option<f64>,
    // pub target_packet_size: usize,
    // pub read_chunk_size: usize,
}

impl RadioConfig {
    /// Specify frequency and sample rate, gain and sizes controlled with builder functions
    pub fn new(frequency: f64, sample_rate: f64) -> Self {
        Self {
            frequency,
            sample_rate,
            ..Default::default()
        }
    }

    /// Set custom gain
    pub fn with_gain(mut self, gain: f64) -> Self {
        self.gain = Some(gain);
        self
    }

    // Configure how large a sequence of I/Q data will be, and the size of chunks read from the SDR
    // pub fn with_sizes(mut self, packet_size: usize, chunk_size: usize) -> Self {
    //     self.target_packet_size = packet_size;
    //     self.read_chunk_size = chunk_size;
    //     self
    // }
}

impl Default for RadioConfig {
    fn default() -> Self {
        Self {
            frequency: 101.1e6,
            sample_rate: 2.048e6,
            gain: None,
        }
    }
}
