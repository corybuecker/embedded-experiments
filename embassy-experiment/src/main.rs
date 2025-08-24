#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::{self as _, config};
use panic_probe as _; // global logger + panicking behavior

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let _peripherals = embassy_nrf::init(config::Config::default());
    info!("Hello, world!");

    #[allow(clippy::empty_loop)]
    loop {}
}
