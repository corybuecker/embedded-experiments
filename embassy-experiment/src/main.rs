#![no_std]
#![no_main]

use core::future;

use defmt::{info, *};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::{self as _, config};
use nrf_softdevice::{
    self as _, Softdevice,
    ble::{
        Uuid,
        advertisement_builder::{
            AdvertisementPayload, ExtendedAdvertisementBuilder, Flag, ServiceList, ServiceUuid16,
        },
        gatt_server::{builder::ServiceBuilder, characteristic},
        peripheral,
    },
};
use panic_probe as _; // global logger + panicking behavior

#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}

static SCAN_DATA: [u8; 0] = [];

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let _peripherals = embassy_nrf::init(config::Config::default());

    let service_id = ServiceUuid16::from_u16(0x183B);

    let sd = Softdevice::enable(&nrf_softdevice::Config::default());
    let mut service = unwrap!(ServiceBuilder::new(sd, Uuid::new_16(0x183B)));
    let characteristic = unwrap!(
        service.add_characteristic(
            Uuid::new_16(0x2A3D),
            characteristic::Attribute::new("reading".as_bytes())
                .read_security(nrf_softdevice::ble::SecurityMode::Open),
            characteristic::Metadata::default(),
        )
    );
    characteristic.build();

    unwrap!(spawner.spawn(softdevice_task(sd)));

    let adv_data: AdvertisementPayload<_> = ExtendedAdvertisementBuilder::new()
        .flags(&[Flag::GeneralDiscovery, Flag::LE_Only])
        .services_16(ServiceList::Complete, &[service_id])
        .full_name("Notify1")
        .build();

    let advertising = peripheral::ConnectableAdvertisement::ScannableUndirected {
        adv_data: &adv_data,
        scan_data: &SCAN_DATA,
    };

    let config = peripheral::Config::default();

    #[allow(clippy::empty_loop)]
    loop {
        let conn = unwrap!(peripheral::advertise_connectable(sd, advertising, &config).await);
        info!("advertising done");
        future::pending::<()>().await
    }
}
