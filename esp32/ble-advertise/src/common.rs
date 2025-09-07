pub use defmt_rtt as _;
pub use esp_alloc as _;
pub use esp_backtrace as _;

use embassy_time::Instant;

defmt::timestamp!("{=u32:us}", Instant::now().as_micros() as u32);
