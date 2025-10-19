#![no_std]
#![no_main]

mod common;

use core::future::pending;

#[allow(unused)]
use defmt::{debug, info, unwrap, warn};
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_time::Timer;

#[embassy_executor::task]
pub async fn blink_task(mut led: Output<'static>) -> ! {
    loop {
        led.set_low();
        Timer::after_millis(5000).await;
        led.set_high();
        Timer::after_millis(1000).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> () {
    let perihperals = embassy_rp::init(Default::default());

    let led = perihperals.PIN_25;
    let led = Output::new(led, Level::Low);

    debug!("Blinking...");
    unwrap!(spawner.spawn(blink_task(led)));

    pending().await
}
