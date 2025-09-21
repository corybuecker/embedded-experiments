#![no_std]
#![no_main]

mod beacon_server;
mod common;
mod tasks;

use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Timer};
use nrf_softdevice::{
    Softdevice,
    ble::{
        advertisement_builder::{ExtendedAdvertisementBuilder, ServiceList, ServiceUuid16},
        peripheral::{self, NonconnectableAdvertisement, advertise},
    },
};
use tasks::softdevice_task;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let mut config = embassy_nrf::config::Config::default();
    config.debug = embassy_nrf::config::Debug::Allowed;
    config.gpiote_interrupt_priority = embassy_nrf::interrupt::Priority::P5;
    config.time_interrupt_priority = embassy_nrf::interrupt::Priority::P5;

    let _peripherals = embassy_nrf::init(config);

    let softdevice = Softdevice::enable(&nrf_softdevice::Config::default());

    let advertisement_data = ExtendedAdvertisementBuilder::new()
        .full_name("Beacon1")
        .build();

    let _beacon_server = beacon_server::BeaconServer {};
    unwrap!(spawner.spawn(softdevice_task(softdevice)));
    let mut data: u16 = 0;
    loop {
        data += 1;

        let scan_data = ExtendedAdvertisementBuilder::new()
            .services_16(ServiceList::Complete, &[ServiceUuid16::from_u16(data)])
            .build();

        let advertisement = NonconnectableAdvertisement::ScannableUndirected {
            adv_data: &advertisement_data,
            scan_data: &scan_data,
        };

        defmt::info!("Starting advertisement cycle {}", data);

        match select(
            advertise(softdevice, advertisement, &peripheral::Config::default()),
            Timer::after(Duration::from_millis(100)),
        )
        .await
        {
            Either::First(_) => {
                defmt::info!("Advertisement completed first");
            }
            Either::Second(_) => {
                defmt::info!("1000ms timer elapsed first - updating scan data");
            }
        }
    }
}
