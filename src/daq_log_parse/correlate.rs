use chrono::TimeZone as _;
use std::ops::Sub as _;

use crate::daq_log_parse::parse::ParsedMessage;

pub struct CorrelationFunction {
    ref_real_ts: chrono::DateTime<chrono::Local>,
    ref_log_ts: u32,
    avg_offset: chrono::Duration,
}
impl CorrelationFunction {
    pub fn correlate(&self, log_ts: u64) -> chrono::DateTime<chrono::Local> {
        self.ref_real_ts
            + chrono::Duration::milliseconds(log_ts as i64 - self.ref_log_ts as i64)
            + self.avg_offset
    }
}

pub struct CorrelationChunkResult {
    chunk: Vec<ParsedMessage>,
    correlation_fn: Option<CorrelationFunction>,
}

pub fn time_correlate_chunks(chunks: Vec<Vec<ParsedMessage>>) -> Vec<CorrelationChunkResult> {
    chunks
        .into_iter()
        .map(|chunk| time_correlate_chunk(chunk))
        .collect()
}

impl CorrelationChunkResult {
    pub fn uncorrelated_new(chunk: Vec<ParsedMessage>) -> Self {
        Self {
            chunk,
            correlation_fn: None,
        }
    }

    pub fn correlated_new(chunk: Vec<ParsedMessage>, correlation_fn: CorrelationFunction) -> Self {
        Self {
            chunk,
            correlation_fn: Some(correlation_fn),
        }
    }
}

pub fn sig_to_value(dsv: &can_decode::DecodedSignalValue) -> u64 {
    match &dsv {
        can_decode::DecodedSignalValue::Numeric(v) => v.round() as u64,
        can_decode::DecodedSignalValue::Enum(v, _) => *v as u64,
    }
}

pub fn time_correlate_chunk(chunk: Vec<ParsedMessage>) -> CorrelationChunkResult {
    // Idea: in the chunk, look for GPS messages which have both a timestamp and a corresponding real time
    // Use those to create a mapping from the log's timestamps to real time, and use that mapping to convert
    // all messages in the chunk to have real timestamps.
    // If the correlation is successful, we return the time-adjusted messages.
    // If we can't find any GPS messages, or if the correlation fails for some reason, we return the
    // original messages without time adjustment.

    // First, find all GPS messages and extract their timestamps and real times
    let mut gps_points = Vec::new();
    for msg in &chunk {
        if msg.decoded.name == "gps_time" {
            let millisecond = msg
                .decoded
                .signals
                .get("millisecond")
                .map(|sig| sig_to_value(&sig.value));
            let second = msg
                .decoded
                .signals
                .get("second")
                .map(|sig| sig_to_value(&sig.value));
            let minute = msg
                .decoded
                .signals
                .get("minute")
                .map(|sig| sig_to_value(&sig.value));
            let hour = msg
                .decoded
                .signals
                .get("hour")
                .map(|sig| sig_to_value(&sig.value));
            let day = msg
                .decoded
                .signals
                .get("day")
                .map(|sig| sig_to_value(&sig.value));
            let month = msg
                .decoded
                .signals
                .get("month")
                .map(|sig| sig_to_value(&sig.value));
            let year = msg
                .decoded
                .signals
                .get("year")
                .map(|sig| sig_to_value(&sig.value));

            if let (Some(ms), Some(s), Some(min), Some(h), Some(d), Some(mon), Some(y)) =
                (millisecond, second, minute, hour, day, month, year)
            {
                // Construct a chrono::DateTime from the extracted values
                if let Some(dt) = chrono::NaiveDate::from_ymd_opt(y as i32, mon as u32, d as u32)
                    .and_then(|date| {
                        date.and_hms_milli_opt(h as u32, min as u32, s as u32, ms as u32)
                    })
                {
                    let dt_local = chrono::Local.from_local_datetime(&dt).unwrap();
                    gps_points.push((msg.timestamp, dt_local));
                } else {
                    log::error!(
                        "GPS message at {} ms has invalid date/time values, skipping",
                        msg.timestamp
                    );
                    continue;
                }
            } else {
                log::error!(
                    "GPS message at {} ms is missing some time signals, skipping",
                    msg.timestamp
                );
                continue;
            }
        }
    }

    if gps_points.is_empty() {
        // No GPS points found, can't correlate
        return CorrelationChunkResult::uncorrelated_new(chunk);
    }

    // Attempt to correlate. Use the first GPS point as a reference, and calculate the offset for
    // each subsequent GPS point. If the offsets are consistent-ish, we can assume a linear
    // correlation and adjust all timestamps accordingly. If the offsets are wildly inconsistent,
    // give up on correlation.
    let (ref_log_ts, ref_real_ts) = gps_points[0];
    let mut offsets = Vec::new();
    for (log_ts, real_ts) in &gps_points[1..] {
        let offset = *real_ts
            - chrono::Duration::milliseconds(*log_ts as i64)
            - (ref_real_ts - chrono::Duration::milliseconds(ref_log_ts as i64));
        offsets.push(offset);
    }

    // Check if offsets are consistent (within 20 ms of each other)
    let zero = chrono::Duration::zero();
    let max_offset = offsets.iter().max().unwrap_or(&zero);
    let min_offset = offsets.iter().min().unwrap_or(&zero);
    if max_offset.sub(*min_offset) > chrono::Duration::milliseconds(20) {
        log::error!(
            "Offsets between GPS points are inconsistent (max: {:?}, min: {:?}), giving up on correlation",
            max_offset,
            min_offset
        );
        return CorrelationChunkResult::uncorrelated_new(chunk);
    }

    // Use the average offset for correlation
    let avg_offset = offsets
        .iter()
        .fold(chrono::Duration::zero(), |acc, x| acc + *x)
        / (offsets.len() as i32);

    CorrelationChunkResult::correlated_new(
        chunk,
        CorrelationFunction {
            ref_real_ts,
            ref_log_ts,
            avg_offset,
        },
    )
}
