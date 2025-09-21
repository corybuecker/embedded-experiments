#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = embassy_nrf::interrupt::Priority::P5;
    // config.time_interrupt_priority = embassy_nrf::interrupt::Priority::P5;
    let _peripherals = embassy_nrf::init(config);

    info!("booted!");

    loop {}
}
