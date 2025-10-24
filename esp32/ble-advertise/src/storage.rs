use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    mutex::{Mutex, MutexGuard},
};
use embassy_time::{Duration, Instant};
use heapless::Vec;

pub struct Bucket {
    pub value: u16,
    pub cutoff: usize,
    pub oldest_updated_at: Instant,
}

pub struct Events {
    #[allow(dead_code)]
    pub buckets: Mutex<NoopRawMutex, [Bucket; 8]>,

    pub last_updated_at: Instant,
}

pub enum RecordType {
    High,
    Low,
}

fn clean_buckets(
    buckets: &mut MutexGuard<NoopRawMutex, [Bucket; 8]>,
    last_updated_at: Instant,
    now: Instant,
) {
    buckets.iter_mut().for_each(|bucket| {
        let cutoff = now.saturating_sub(Duration::from_millis(bucket.cutoff as u64));

        if bucket.oldest_updated_at < cutoff {
            if bucket.value > 0 {
                bucket.value -= 1;
            }

            bucket.oldest_updated_at = bucket
                .oldest_updated_at
                .saturating_add(now - last_updated_at);
        }
    });
}

impl Events {
    pub fn new() -> Self {
        Self::new_with_time(Instant::now())
    }

    pub fn new_with_time(start_time: Instant) -> Self {
        let buckets = [
            Bucket {
                value: 0,
                cutoff: 500,
                oldest_updated_at: start_time,
            },
            Bucket {
                value: 0,
                cutoff: 1000,
                oldest_updated_at: start_time,
            },
            Bucket {
                value: 0,
                cutoff: 5 * 1000,
                oldest_updated_at: start_time,
            },
            Bucket {
                value: 0,
                cutoff: 30 * 1000,
                oldest_updated_at: start_time,
            },
            Bucket {
                value: 0,
                cutoff: 60 * 1000,
                oldest_updated_at: start_time,
            },
            Bucket {
                value: 0,
                cutoff: 120 * 1000,
                oldest_updated_at: start_time,
            },
            Bucket {
                value: 0,
                cutoff: 180 * 1000,
                oldest_updated_at: start_time,
            },
            Bucket {
                value: 0,
                cutoff: 300 * 1000,
                oldest_updated_at: start_time,
            },
        ];

        Self {
            buckets: Mutex::new(buckets),
            last_updated_at: start_time,
        }
    }

    pub async fn record(&mut self, record_type: RecordType) {
        self.record_at_time(record_type, Instant::now()).await;
    }

    pub async fn record_at_time(&mut self, record_type: RecordType, now: Instant) {
        let mut buckets = self.buckets.lock().await;

        clean_buckets(&mut buckets, self.last_updated_at, now);

        match record_type {
            RecordType::High => {
                buckets.iter_mut().for_each(|bucket| {
                    bucket.value += 1;
                });
            }
            RecordType::Low => {}
        }

        drop(buckets);

        self.last_updated_at = now;
    }

    pub async fn report(&self) -> Vec<u16, 8> {
        let buckets = self.buckets.lock().await;
        buckets
            .iter()
            .map(|bucket| bucket.value)
            .collect::<Vec<u16, 8>>()
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
            let start_time = Instant::now();
            let events = Events::new_with_time(start_time);

            let report = events.report().await;
            assert_eq!(report.len(), 8);
            for value in report.iter() {
                assert_eq!(*value, 0);
            }
        });
    }

    #[test]
    fn test_record_high_increments_all_buckets() {
        block_on(async {
            let start_time = Instant::now();
            let mut events = Events::new_with_time(start_time);

            events.record_at_time(RecordType::High, start_time).await;

            let report = events.report().await;
            for value in report.iter() {
                assert_eq!(*value, 1);
            }
        });
    }

    #[test]
    fn test_record_low_does_not_increment_buckets() {
        block_on(async {
            let start_time = Instant::now();
            let mut events = Events::new_with_time(start_time);

            events.record_at_time(RecordType::Low, start_time).await;

            let report = events.report().await;
            for value in report.iter() {
                assert_eq!(*value, 0);
            }
        });
    }

    #[test]
    fn test_multiple_high_records_increment_correctly() {
        block_on(async {
            let start_time = Instant::now();
            let mut events = Events::new_with_time(start_time);

            events.record_at_time(RecordType::High, start_time).await;
            events
                .record_at_time(RecordType::High, start_time + Duration::from_millis(100))
                .await;
            events
                .record_at_time(RecordType::High, start_time + Duration::from_millis(200))
                .await;

            let report = events.report().await;
            for value in report.iter() {
                assert_eq!(*value, 3);
            }
        });
    }

    #[test]
    fn test_bucket_cleaning_after_cutoff_time() {
        block_on(async {
            let start_time = Instant::now();
            let mut events = Events::new_with_time(start_time);

            // Record a high event
            events.record_at_time(RecordType::High, start_time).await;

            // First bucket has 500ms cutoff, advance time beyond that
            let future_time = start_time + Duration::from_millis(600);
            events
                .record_at_time(RecordType::Low, future_time)
                .await;

            let report = events.report().await;
            // First bucket (500ms cutoff) should be decremented to 0
            assert_eq!(report[0], 0);
            // Other buckets should still be 1
            for i in 1..8 {
                assert_eq!(report[i], 1);
            }
        });
    }

    #[test]
    fn test_bucket_cleaning_multiple_buckets() {
        block_on(async {
            let start_time = Instant::now();
            let mut events = Events::new_with_time(start_time);

            // Record a high event
            events.record_at_time(RecordType::High, start_time).await;

            // Advance time beyond second bucket cutoff (1000ms)
            let future_time = start_time + Duration::from_millis(1100);
            events
                .record_at_time(RecordType::Low, future_time)
                .await;

            let report = events.report().await;
            // First two buckets (500ms, 1000ms cutoffs) should be decremented to 0
            assert_eq!(report[0], 0);
            assert_eq!(report[1], 0);
            // Other buckets should still be 1
            for i in 2..8 {
                assert_eq!(report[i], 1);
            }
        });
    }

    #[test]
    fn test_bucket_values_do_not_go_negative() {
        block_on(async {
            let start_time = Instant::now();
            let mut events = Events::new_with_time(start_time);

            // Don't record any events, just advance time
            let future_time = start_time + Duration::from_millis(600);
            events
                .record_at_time(RecordType::Low, future_time)
                .await;

            let report = events.report().await;
            // All buckets should remain at 0
            for value in report.iter() {
                assert_eq!(*value, 0);
            }
        });
    }

    #[test]
    fn test_incremental_bucket_decay() {
        block_on(async {
            let start_time = Instant::now();
            let mut events = Events::new_with_time(start_time);

            // Record multiple high events
            events.record_at_time(RecordType::High, start_time).await;
            events
                .record_at_time(RecordType::High, start_time + Duration::from_millis(100))
                .await;
            events
                .record_at_time(RecordType::High, start_time + Duration::from_millis(200))
                .await;

            // All buckets should have value 3
            let report = events.report().await;
            for value in report.iter() {
                assert_eq!(*value, 3);
            }

            // Advance time beyond first bucket cutoff
            let future_time = start_time + Duration::from_millis(600);
            events
                .record_at_time(RecordType::Low, future_time)
                .await;

            let report = events.report().await;
            // First bucket should decrement by 1 to value 2
            assert_eq!(report[0], 2);
            // Other buckets should still be 3
            for i in 1..8 {
                assert_eq!(report[i], 3);
            }
        });
    }

    #[test]
    fn test_all_buckets_decay_over_long_time() {
        block_on(async {
            let start_time = Instant::now();
            let mut events = Events::new_with_time(start_time);

            // Record a high event
            events.record_at_time(RecordType::High, start_time).await;

            // Advance time beyond all bucket cutoffs (past 300s)
            let future_time = start_time + Duration::from_millis(301_000);
            events
                .record_at_time(RecordType::Low, future_time)
                .await;

            let report = events.report().await;
            // All buckets should be decremented to 0
            for value in report.iter() {
                assert_eq!(*value, 0);
            }
        });
    }

    #[test]
    fn test_mixed_record_types() {
        block_on(async {
            let start_time = Instant::now();
            let mut events = Events::new_with_time(start_time);

            events.record_at_time(RecordType::High, start_time).await;
            events
                .record_at_time(RecordType::Low, start_time + Duration::from_millis(100))
                .await;
            events
                .record_at_time(RecordType::High, start_time + Duration::from_millis(200))
                .await;
            events
                .record_at_time(RecordType::Low, start_time + Duration::from_millis(300))
                .await;

            let report = events.report().await;
            // Should have 2 high events recorded
            for value in report.iter() {
                assert_eq!(*value, 2);
            }
        });
    }
}
