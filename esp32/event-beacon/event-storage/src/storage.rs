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

#[cfg(test)]
mod tests {
    use super::*;
    use embassy_time::Duration;
    use futures::executor::block_on;

    #[test]
    fn test_new_events_initializes_with_zero_values() {
        block_on(async {
            let events = Events::default();

            let report = events.as_bytes().await;
            assert_eq!(report.len(), 8);
            for value in report.iter() {
                assert_eq!(*value, 0);
            }
        });
    }

    #[test]
    fn test_record_high_increments_all_buckets() {
        block_on(async {
            let start_time = Instant::from_ticks(0);
            let events = Events::default();

            // Need at least 3 records to establish timestamp history
            events.record_at_time(RecordType::Low, start_time).await;
            events
                .record_at_time(RecordType::Low, start_time + Duration::from_millis(100))
                .await;
            events
                .record_at_time(RecordType::High, start_time + Duration::from_millis(200))
                .await;

            let report = events.as_bytes().await;
            for value in report.iter() {
                assert_eq!(*value, 1);
            }
        });
    }

    #[test]
    fn test_record_low_does_not_increment_buckets() {
        block_on(async {
            let start_time = Instant::from_ticks(0);
            let events = Events::default();

            // Need at least 3 records to establish timestamp history
            events.record_at_time(RecordType::Low, start_time).await;
            events
                .record_at_time(RecordType::Low, start_time + Duration::from_millis(100))
                .await;
            events
                .record_at_time(RecordType::Low, start_time + Duration::from_millis(200))
                .await;

            let report = events.as_bytes().await;
            for value in report.iter() {
                assert_eq!(*value, 0);
            }
        });
    }

    #[test]
    fn test_multiple_high_records_increment_correctly() {
        block_on(async {
            let start_time = Instant::from_ticks(0);
            let events = Events::default();

            // Need at least 3 records to establish timestamp history
            events.record_at_time(RecordType::Low, start_time).await;
            events
                .record_at_time(RecordType::Low, start_time + Duration::from_millis(100))
                .await;
            events
                .record_at_time(RecordType::High, start_time + Duration::from_millis(200))
                .await;
            events
                .record_at_time(RecordType::High, start_time + Duration::from_millis(300))
                .await;
            events
                .record_at_time(RecordType::High, start_time + Duration::from_millis(400))
                .await;

            let report = events.as_bytes().await;
            for value in report.iter() {
                assert_eq!(*value, 3);
            }
        });
    }

    #[test]
    fn test_bucket_cleaning_after_cutoff_time() {
        block_on(async {
            let start_time = Instant::from_ticks(0);
            let events = Events::default();

            // Establish timestamp history with regular 100ms intervals
            events.record_at_time(RecordType::Low, start_time).await;
            events
                .record_at_time(RecordType::Low, start_time + Duration::from_millis(100))
                .await;

            // Record a high event
            events
                .record_at_time(RecordType::High, start_time + Duration::from_millis(200))
                .await;

            // The high event is in the buffer, report should show 1 for all buckets
            let report = events.as_bytes().await;
            for value in report.iter() {
                assert_eq!(*value, 1);
            }

            // Add more low events to push the high event out of the first bucket's window
            // First bucket is 1000ms, with ~100ms avg duration we need ~10 more entries
            for i in 0..12 {
                events
                    .record_at_time(
                        RecordType::Low,
                        start_time + Duration::from_millis(300 + i * 100),
                    )
                    .await;
            }

            let report = events.as_bytes().await;
            // First bucket should now be 0 (high event pushed out of window)
            assert_eq!(report[0], 0);
            // Larger buckets should still contain the high event
            for i in 1..8 {
                assert_eq!(report[i], 1);
            }
        });
    }

    #[test]
    fn test_bucket_cleaning_multiple_buckets() {
        block_on(async {
            let start_time = Instant::from_ticks(0);
            let events = Events::default();

            // Establish timestamp history with regular 100ms intervals
            events.record_at_time(RecordType::Low, start_time).await;
            events
                .record_at_time(RecordType::Low, start_time + Duration::from_millis(100))
                .await;

            // Record a high event
            events
                .record_at_time(RecordType::High, start_time + Duration::from_millis(200))
                .await;

            // Add enough low events to push high event out of first two buckets
            // Second bucket is 5000ms, with ~100ms avg we need ~50 more entries
            for i in 0..60 {
                events
                    .record_at_time(
                        RecordType::Low,
                        start_time + Duration::from_millis(300 + i * 100),
                    )
                    .await;
            }

            let report = events.as_bytes().await;
            // First two buckets should now be 0 (high event pushed out of their windows)
            assert_eq!(report[0], 0);
            assert_eq!(report[1], 0);
            // Larger buckets should still contain the high event
            for i in 2..8 {
                assert_eq!(report[i], 1);
            }
        });
    }

    #[test]
    fn test_bucket_values_do_not_go_negative() {
        block_on(async {
            let start_time = Instant::from_ticks(0);
            let events = Events::default();

            // Don't record any events, just advance time
            let future_time = start_time + Duration::from_millis(600);
            events.record_at_time(RecordType::Low, future_time).await;

            let report = events.as_bytes().await;
            // All buckets should remain at 0
            for value in report.iter() {
                assert_eq!(*value, 0);
            }
        });
    }

    #[test]
    fn test_incremental_bucket_decay() {
        block_on(async {
            let start_time = Instant::from_ticks(0);
            let events = Events::default();

            // Establish timestamp history with regular 100ms intervals
            events.record_at_time(RecordType::Low, start_time).await;
            events
                .record_at_time(RecordType::Low, start_time + Duration::from_millis(100))
                .await;

            // Record multiple high events
            events
                .record_at_time(RecordType::High, start_time + Duration::from_millis(200))
                .await;
            events
                .record_at_time(RecordType::High, start_time + Duration::from_millis(300))
                .await;
            events
                .record_at_time(RecordType::High, start_time + Duration::from_millis(400))
                .await;

            // All buckets should have value 3
            let report = events.as_bytes().await;
            for value in report.iter() {
                assert_eq!(*value, 3);
            }

            // Add more entries to push one high event out of the first bucket
            // First bucket is 1000ms, we need about 10 entries total in that window
            for i in 0..8 {
                events
                    .record_at_time(
                        RecordType::Low,
                        start_time + Duration::from_millis(500 + i * 100),
                    )
                    .await;
            }

            let report = events.as_bytes().await;
            // First bucket should now have 2 (one high event pushed out)
            assert_eq!(report[0], 2);
            // Other buckets should still have 3
            for i in 1..8 {
                assert_eq!(report[i], 3);
            }
        });
    }

    #[test]
    fn test_all_buckets_decay_over_long_time() {
        block_on(async {
            let start_time = Instant::from_ticks(0);
            let events = Events::default();

            // Establish timestamp history with regular 100ms intervals
            events.record_at_time(RecordType::Low, start_time).await;
            events
                .record_at_time(RecordType::Low, start_time + Duration::from_millis(100))
                .await;

            // Record a high event
            events
                .record_at_time(RecordType::High, start_time + Duration::from_millis(200))
                .await;

            // The largest bucket is 600000ms (600s). Add enough entries to push it out.
            // With 100ms intervals, we need 6000+ entries, but buffer is only 3000.
            // So let's fill the buffer with Low events which will push out the High event.
            for i in 0..3000 {
                events
                    .record_at_time(
                        RecordType::Low,
                        start_time + Duration::from_millis(300 + i * 100),
                    )
                    .await;
            }

            let report = events.as_bytes().await;
            // All buckets should be 0 (high event pushed completely out of buffer)
            for value in report.iter() {
                assert_eq!(*value, 0);
            }
        });
    }

    #[test]
    fn test_different_record_types() {
        block_on(async {
            let start_time = Instant::from_ticks(0);
            let events = Events::default();

            // Establish timestamp history first
            events.record_at_time(RecordType::Low, start_time).await;
            events
                .record_at_time(RecordType::Low, start_time + Duration::from_millis(100))
                .await;
            events
                .record_at_time(RecordType::High, start_time + Duration::from_millis(200))
                .await;
            events
                .record_at_time(RecordType::Low, start_time + Duration::from_millis(300))
                .await;
            events
                .record_at_time(RecordType::High, start_time + Duration::from_millis(400))
                .await;
            events
                .record_at_time(RecordType::Low, start_time + Duration::from_millis(500))
                .await;

            let report = events.as_bytes().await;
            // Should have 2 high events recorded
            for value in report.iter() {
                assert_eq!(*value, 2);
            }
        });
    }
}
