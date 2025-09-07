use core::{
    cell::RefCell,
    sync::atomic::{AtomicBool, Ordering},
};
use defmt::debug;
use embassy_sync::blocking_mutex::CriticalSectionMutex;
use heapless::HistoryBuf;

pub type SharedMeasurements = CriticalSectionMutex<Measurements>;

pub struct Measurements {
    inner: RefCell<HistoryBuf<u8, 25>>,
    locked: AtomicBool,
}

impl Measurements {
    pub const fn new() -> Self {
        Self {
            inner: RefCell::new(HistoryBuf::new()),
            locked: AtomicBool::new(false),
        }
    }
    pub fn lock(&self) {
        self.locked.store(true, Ordering::Relaxed);
    }

    pub fn unlock(&self) {
        self.locked.store(false, Ordering::Relaxed);
    }

    pub fn add(&self, measurement: u8) {
        debug!("Adding measurement: {}", measurement);
        self.inner.borrow_mut().write(measurement);
    }

    pub fn add_if_unlocked(&self, measurement: u8) {
        if self.locked.load(Ordering::Relaxed) {
            return;
        }

        self.add(measurement);
    }

    pub fn sum(&self) -> u8 {
        let cell = self.inner.borrow_mut();

        cell.iter().copied().sum()
    }
}
