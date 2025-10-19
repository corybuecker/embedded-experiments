#![no_std]
#![no_main]

mod common;

use core::future::pending;

#[allow(unused)]
use defmt::{debug, info, unwrap, warn};
use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Pull};
use embassy_time::Timer;

#[embassy_executor::task]
pub async fn read_sensor_level(sensor: Input<'static>) -> ! {
    loop {
        debug!("Sensor level: {:?}", sensor.get_level());

        Timer::after_millis(100).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> () {
    let peripherals = embassy_rp::init(Default::default());

    let sensor = peripherals.PIN_2;
    let sensor = Input::new(sensor, Pull::None);

    unwrap!(spawner.spawn(read_sensor_level(sensor)));

    pending().await
}
