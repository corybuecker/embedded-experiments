#![no_std]
#![no_main]

mod common;
mod led;

use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_hal::{
    clock::CpuClock,
    gpio::{Input, InputConfig},
    timer::timg::TimerGroup,
};

use crate::led::{blue_led, create_channel, off, red_led};

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);

    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    let mut channel = create_channel(peripherals.RMT, peripherals.GPIO8).await;

    defmt::unwrap!(off(&mut channel).await);

    let sensor = Input::new(
        peripherals.GPIO3,
        InputConfig::default().with_pull(esp_hal::gpio::Pull::None),
    );

    loop {
        if sensor.is_high() {
            for _ in 0..50 {
                defmt::unwrap!(red_led(&mut channel).await);
                Timer::after_millis(100).await;
                defmt::unwrap!(blue_led(&mut channel).await);
                Timer::after_millis(100).await;
            }
        }

        defmt::unwrap!(off(&mut channel).await);
        Timer::after_millis(10).await;
    }
}
