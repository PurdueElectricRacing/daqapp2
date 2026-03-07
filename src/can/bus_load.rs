const BUS_LOAD_BUCKET_MS: i64 = 25; // Group frames into 25ms buckets for load calculation
const CAN_BUS_SPEED: f32 = 500_000.0; // 500 kbps
const CAN_FRAME_BITS: usize = 130;

pub struct BusLoadTracker {
    pub timestamps: std::collections::VecDeque<chrono::DateTime<chrono::Local>>,
    pub bits: std::collections::VecDeque<usize>,
}

impl BusLoadTracker {
    pub fn new() -> Self {
        Self {
            timestamps: std::collections::VecDeque::new(),
            bits: std::collections::VecDeque::new(),
        }
    }

    pub fn record_frame(&mut self) {
        let now = chrono::Local::now();
        if let Some(last_timestamp) = self.timestamps.back() {
            if (now - *last_timestamp).num_milliseconds() < BUS_LOAD_BUCKET_MS {
                // Same timing bucket, add to existing
                if let Some(last_bits) = self.bits.back_mut() {
                    *last_bits += CAN_FRAME_BITS;
                }
                return;
            }
        }
        self.timestamps.push_back(now);
        self.bits.push_back(CAN_FRAME_BITS);
    }

    // Returns bus load percentage for the given window in seconds
    pub fn get_load(&self, window_secs: u64) -> f32 {
        let now = chrono::Local::now();
        let window_duration = chrono::Duration::seconds(window_secs as i64);
        let cutoff_time = now - window_duration;

        let mut total_bits = 0;
        for (i, &ts) in self.timestamps.iter().enumerate() {
            if ts > cutoff_time {
                total_bits += self.bits[i];
            }
        }

        let max_bits_in_window = CAN_BUS_SPEED * window_secs as f32;
        (total_bits as f32 / max_bits_in_window) * 100.0
    }

    // Clean up old entries
    pub fn cleanup(&mut self) {
        let cutoff_time = chrono::Local::now() - chrono::Duration::seconds(30);
        while let Some(&ts) = self.timestamps.front() {
            if ts <= cutoff_time {
                self.timestamps.pop_front();
                self.bits.pop_front();
            } else {
                break;
            }
        }
    }
}
