#![no_std]
#![no_main]

mod beacon_server;
mod common;
mod storage;
mod tasks;

use crate::storage::Events;
use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_futures::select::select;
use embassy_time::{Duration, Timer};
use nrf_softdevice::{
    Softdevice,
    ble::{
        advertisement_builder::{ExtendedAdvertisementBuilder, ServiceList},
        peripheral::{self, NonconnectableAdvertisement, advertise},
    },
};
use tasks::softdevice_task;
use uuid::Uuid;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let mut config = embassy_nrf::config::Config::default();
    config.debug = embassy_nrf::config::Debug::Allowed;
    config.gpiote_interrupt_priority = embassy_nrf::interrupt::Priority::P5;
    config.time_interrupt_priority = embassy_nrf::interrupt::Priority::P5;

    let _peripherals = embassy_nrf::init(config);

    let softdevice = Softdevice::enable(&nrf_softdevice::Config::default());

    let advertisement_data = ExtendedAdvertisementBuilder::new()
        .full_name("Beacon")
        .build();

    let _beacon_server = beacon_server::BeaconServer {};
    unwrap!(spawner.spawn(softdevice_task(softdevice)));

    let mut events = Events::new();

    loop {
        events.record(storage::RecordType::High).await;

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

        let scan_data = ExtendedAdvertisementBuilder::new()
            .services_128(ServiceList::Complete, &[uuid.as_bytes().clone()])
            .build();

        let advertisement = NonconnectableAdvertisement::ScannableUndirected {
            adv_data: &advertisement_data,
            scan_data: &scan_data,
        };

        select(
            advertise(softdevice, advertisement, &peripheral::Config::default()),
            Timer::after(Duration::from_millis(100)),
        )
        .await;
    }
}

#[cfg(test)]
mod tests {}
