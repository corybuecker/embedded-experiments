#![no_std]
#![no_main]

mod common;

use core::future::pending;
use embassy_executor::Spawner;
use embassy_futures::select::select3;
use embassy_time::Duration;
use embassy_time::Timer;
use esp_hal::clock::CpuClock;
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_radio::Controller;
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

    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    static RADIO: StaticCell<Controller<'static>> = StaticCell::new();
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

    defmt::info!("starting advertisment");

    let _ = select3(
        runner.run(),
        async {
            loop {
                let a = Rng::new().random();
                if a.is_multiple_of(2) {
                    events
                        .record(event_storage::storage::RecordType::High)
                        .await;
                } else {
                    events.record(event_storage::storage::RecordType::Low).await;
                }

                Timer::after(Duration::from_millis(100)).await;
            }
        },
        async {
            loop {
                let mut advertising_data: [u8; 128] = [0; 128];

                match events.as_uuid().await {
                    Ok(uuid) => {
                        AdStructure::encode_slice(
                            &[AdStructure::ServiceUuids128(&[uuid.to_bytes_le()])],
                            &mut advertising_data,
                        )
                        .expect("could not encode scan data");
                    }
                    Err(err) => {
                        defmt::error!("Error getting UUID: {}", err);
                    }
                }

                let _advertising = peripheral
                    .advertise(
                        &AdvertisementParameters::default(),
                        Advertisement::NonconnectableNonscannableUndirected {
                            adv_data: &advertising_data,
                        },
                    )
                    .await
                    .expect("could not advertise");

                Timer::after(Duration::from_millis(100)).await;
            }
        },
    )
    .await;

    pending().await
}
