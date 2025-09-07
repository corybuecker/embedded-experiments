#![no_std]
#![no_main]

mod common;
mod storage;

use crate::storage::Events;
use core::future::pending;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_time::Duration;
use embassy_time::Timer;
use esp_hal::clock::CpuClock;
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_radio::Controller;
use esp_radio::ble::controller::BleConnector;
use static_cell::StaticCell;
use trouble_host::HostResources;
use trouble_host::prelude::*;
use uuid::Uuid;

esp_bootloader_esp_idf::esp_app_desc!();

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

    // let address: Address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);
    // info!("Our address = {:?}", address);

    let mut resources: HostResources<DefaultPacketPool, 1, 2> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources);
    let Host {
        mut peripheral,
        mut runner,
        ..
    } = stack.build();

    let mut advertising_data: [u8; 64] = [0; 64];
    let mut events = Events::new();

    AdStructure::encode_slice(
        &[AdStructure::CompleteLocalName("ESPBeacon".as_bytes())],
        &mut advertising_data,
    )
    .expect("could not encode advertising data");

    defmt::info!("starting advertisment");

    let _ = join(runner.run(), async {
        loop {
            let a = Rng::new().random();
            if a.is_multiple_of(2) {
                events.record(storage::RecordType::High).await;
            } else {
                events.record(storage::RecordType::Low).await;
            }

            let values = events.report().await;
            let values = values.as_slice();
            let uuid = {
                let mut bytes = [0u8; 16];
                for (i, &value) in values.iter().enumerate() {
                    let value_bytes = value.to_le_bytes(); // or to_le_bytes() depending on endianness
                    bytes[i * 2] = value_bytes[0];
                    bytes[i * 2 + 1] = value_bytes[1];
                }

                Uuid::from_bytes(bytes)
            };

            let mut scan_data: [u8; 128] = [0; 128];
            AdStructure::encode_slice(
                &[AdStructure::ServiceUuids128(&[uuid.to_bytes_le()])],
                &mut scan_data,
            )
            .expect("could not encode scan data");

            let _advertising = peripheral
                .advertise(
                    &AdvertisementParameters::default(),
                    Advertisement::NonconnectableScannableUndirected {
                        adv_data: &advertising_data,
                        scan_data: &scan_data,
                    },
                )
                .await
                .expect("could not advertise");

            Timer::after(Duration::from_millis(100)).await;
        }
    })
    .await;

    pending().await
}
