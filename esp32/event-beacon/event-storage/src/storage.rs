use core::array::TryFromSliceError;

use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_time::Instant;
use heapless::{HistoryBuf, Vec};
use uuid::Uuid;

pub enum RecordType {
    High,
    Low,
}

static BUFFER_SIZE: usize = 3000;
static UPDATE_BUFFER_SIZE: usize = 30;
static BREAKPOINTS: [u64; 8] = [1000, 5000, 30000, 60000, 120000, 240000, 360000, 600000];

pub struct Events {
    #[allow(dead_code)]
    buffer: Mutex<NoopRawMutex, HistoryBuf<u8, BUFFER_SIZE>>,
    update_timestamps: Mutex<NoopRawMutex, HistoryBuf<Instant, UPDATE_BUFFER_SIZE>>,
    report: Mutex<NoopRawMutex, Vec<u16, 8>>,
}

impl Default for Events {
    fn default() -> Self {
        Self {
            buffer: Mutex::new(HistoryBuf::new()),
            update_timestamps: Mutex::new(HistoryBuf::new()),
            report: Mutex::new(Vec::new()),
        }
    }
}

impl Events {
    pub async fn record(&self, record_type: RecordType) {
        self.record_at_time(record_type, Instant::now()).await;
    }

    async fn average_duration_between_updates(&self) -> Option<u64> {
        let update_timestamps = self.update_timestamps.lock().await;
        let total_duration_between_updates: u64 = update_timestamps
            .as_slice()
            .windows(2)
            .map(|pair| pair[1].saturating_duration_since(pair[0]))
            .map(|duration| duration.as_millis())
            .sum();

        total_duration_between_updates.checked_div(update_timestamps.len() as u64)
    }

    async fn record_at_time(&self, record_type: RecordType, timestamp: Instant) {
        let mut buffer = self.buffer.lock().await;
        match record_type {
            RecordType::High => buffer.write(1),
            RecordType::Low => buffer.write(0),
        }

        let average_duration_between_updates = self.average_duration_between_updates().await;

        match average_duration_between_updates {
            Some(duration) => {
                let breakpoints = BREAKPOINTS
                    .iter()
                    .map(|&breakpoint| {
                        buffer
                            .iter()
                            .rev()
                            .take(breakpoint.checked_div(duration).unwrap_or(0) as usize)
                            .map(|&x| x as u16)
                            .sum::<u16>()
                    })
                    .take(8)
                    .collect::<Vec<u16, 8>>();

                let mut report_for_update = self.report.lock().await;
                *report_for_update = breakpoints
            }
            None => {}
        }

        let mut update_timestamps_for_update = self.update_timestamps.lock().await;
        update_timestamps_for_update.write(timestamp);
    }

    pub async fn as_bytes(&self) -> [u16; 8] {
        let report = self.report.lock().await;
        let report_copy = report.clone();
        drop(report);

        let report_copy: [u16; 8] = report_copy
            .get(..)
            .and_then(|s| s.try_into().ok())
            .unwrap_or_default();

        report_copy
    }

    pub async fn as_uuid(&self) -> Result<Uuid, TryFromSliceError> {
        let report = self.report.lock().await;
        let report_copy = report.clone();
        drop(report);

        let bytes = report_copy
            .iter()
            .flat_map(|&v| v.to_be_bytes())
            .collect::<Vec<u8, 16>>();

        let bytes = bytes.as_slice().try_into()?;

        Ok(Uuid::from_bytes(bytes))
    }
}
