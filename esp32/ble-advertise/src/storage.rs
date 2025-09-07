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
