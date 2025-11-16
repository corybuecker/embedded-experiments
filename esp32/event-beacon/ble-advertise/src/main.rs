#![no_std]
#![no_main]

mod common;
mod gatt;
mod led;

use crate::gatt::advertise_and_handle_connection;
use crate::led::{create_channel, off, red_led};
use core::future::pending;
use defmt::Debug2Format;
use embassy_executor::Spawner;
use embassy_futures::select::select4;
use embassy_time::Timer;
use esp_hal::Async;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Level};
use esp_hal::rmt::{Channel, Tx};
use esp_hal::rtc_cntl::Rtc;
use esp_hal::time::Duration;
use esp_hal::timer::timg::TimerGroup;
use esp_radio::ble::controller::BleConnector;
use event_storage::storage::Events;
use static_cell::StaticCell;
use trouble_host::HostResources;
use trouble_host::prelude::*;

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);

    let mut watchdog = timg0.wdt;
    watchdog.enable();
    watchdog.set_timeout(
        esp_hal::timer::timg::MwdtStage::Stage0,
        Duration::from_secs(5),
    );

    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    let _rtc = Rtc::new(peripherals.LPWR);

    let mut led_channel = create_channel(peripherals.RMT, peripherals.GPIO8).await;

    static RADIO: StaticCell<esp_radio::Controller<'static>> = StaticCell::new();
    let radio = RADIO.init(esp_radio::init().unwrap());

    let bluetooth = peripherals.BT;
    let connector = BleConnector::new(radio, bluetooth, Default::default());

    let connector = match connector {
        Ok(connector) => connector,
        Err(err) => panic!("Failed to connect: {:?}", err),
    };

    let controller: ExternalController<_, 1> =
        trouble_host::prelude::ExternalController::new(connector);

    let mut resources: HostResources<DefaultPacketPool, 1, 2> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources);
    let Host {
        mut peripheral,
        mut runner,
        ..
    } = stack.build();

    let events = Events::default();
    let input = Input::new(
        peripherals.GPIO3,
        InputConfig::default().with_pull(esp_hal::gpio::Pull::None),
    );

    let _ = select4(
        runner.run(),
        collect_events(&input, &events, &mut led_channel),
        advertise_and_handle_connection(&events, &mut peripheral),
        async {
            loop {
                Timer::after_secs(3).await;
                watchdog.feed();
            }
            // let mut sleep_config = RtcSleepConfig::default();
            // Timer::after_secs(5).await;
            // esp_println::dbg!("going to sleep for five seconds");
            // let wakeup_source = TimerWakeupSource::new(Duration::from_millis(5000));
            // rtc.sleep(&sleep_config, &[&wakeup_source]);
        },
    )
    .await;

    pending().await
}

async fn collect_events(
    input: &Input<'static>,
    events: &Events,
    led_channel: &mut Channel<'static, Async, Tx>,
) {
    loop {
        match input.level() {
            Level::High => {
                events
                    .record(event_storage::storage::RecordType::High)
                    .await;

                let _result = red_led(led_channel).await;
                // defmt::error!("{}", Debug2Format(&result));
            }
            Level::Low => {
                events.record(event_storage::storage::RecordType::Low).await;
                let _ = off(led_channel).await;
            }
        }

        Timer::after_millis(100).await;
    }
}
