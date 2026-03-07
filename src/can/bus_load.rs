const CAN_BUS_SPEED: f32 = 500_000.0; // 500 kbps

// SOF	1
// ID	11
// RTR	1
// IDE	1
// r0	1
// DLC	4
// Data	64
// CRC	15
// CRC delim	1
// ACK	2
// EOF	7
// IFS	3
// Subtotal: 111
// But with bit stuffing: +~20%
const CAN_FRAME_BITS: usize = 130;

const CLEAN_UP_INTERVAL_SECS: i64 = 30;

pub struct BusLoadTracker {
    timestamps: std::collections::VecDeque<chrono::DateTime<chrono::Local>>,
}

impl BusLoadTracker {
    pub fn new() -> Self {
        Self {
            timestamps: std::collections::VecDeque::new(),
        }
    }

    pub fn record_frame(&mut self) {
        self.timestamps.push_back(chrono::Local::now());
    }

    // Returns bus load percentage for the given window in seconds
    pub fn get_load(&self, window_secs: u64) -> f32 {
        let now = chrono::Local::now();
        let window_duration = chrono::Duration::seconds(window_secs as i64);
        let cutoff_time = now - window_duration;

        let total_bits = self
            .timestamps
            .iter()
            .filter(|&&ts| ts > cutoff_time)
            .count()
            * CAN_FRAME_BITS;

        let max_bits_in_window = CAN_BUS_SPEED * window_secs as f32;
        (total_bits as f32 / max_bits_in_window) * 100.0
    }

    // Clean up old entries
    pub fn cleanup(&mut self) {
        let cutoff_time = chrono::Local::now() - chrono::Duration::seconds(CLEAN_UP_INTERVAL_SECS);
        while let Some(&ts) = self.timestamps.front() {
            if ts <= cutoff_time {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }
    }
}
