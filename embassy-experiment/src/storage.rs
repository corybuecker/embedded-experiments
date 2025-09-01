use heapless::HistoryBuf;

pub struct Measurements {
    inner: HistoryBuf<u8, 25>,
}

impl Measurements {
    pub const fn new() -> Self {
        Self {
            inner: HistoryBuf::new(),
        }
    }

    pub fn add(&mut self, measurement: u8) {
        self.inner.write(measurement);
    }

    pub fn sum(&self) -> u8 {
        self.inner.iter().copied().sum()
    }
}
