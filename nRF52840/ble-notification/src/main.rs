#![no_std]
#![no_main]

mod common;
mod notify_server;
mod storage;
mod tasks;

use crate::notify_server::NotifyServer;
use crate::tasks::{add_empty_measurements, notify_task, sensor_task, softdevice_task};
use core::cell::Cell;
use defmt::{info, unwrap};
use embassy_executor::Spawner;
use embassy_nrf::gpio::Input;
use embassy_nrf::{config, interrupt::Priority};
use embassy_sync::blocking_mutex::Mutex;
use futures::{future::select, pin_mut};
use nrf_softdevice::ble::Uuid;
use nrf_softdevice::ble::gatt_server::builder::ServiceBuilder;
use nrf_softdevice::{
    Softdevice,
    ble::{
        advertisement_builder::{
            AdvertisementPayload, ExtendedAdvertisementBuilder, Flag, ServiceList, ServiceUuid16,
        },
        gatt_server::{
            self,
            characteristic::{self, Properties},
        },
        peripheral,
    },
};

static SCAN_DATA: [u8; 0] = [];

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let mut config = config::Config::default();
    config.gpiote_interrupt_priority = Priority::P5;
    config.time_interrupt_priority = Priority::P5;
    let peripherals = embassy_nrf::init(config);

    let sensor = Input::new(peripherals.P1_08, embassy_nrf::gpio::Pull::Down);

    let service_id = ServiceUuid16::from_u16(0x183B);
    let sd = Softdevice::enable(&nrf_softdevice::Config::default());
    let mut service = unwrap!(ServiceBuilder::new(sd, Uuid::new_16(0x183B)));

    let attribute = characteristic::Attribute::new(&[0x1]);
    let attribute = attribute.read_security(nrf_softdevice::ble::SecurityMode::Open);
    let metadata = characteristic::Metadata::new(Properties::default().notify());
    let characteristic =
        unwrap!(service.add_characteristic(Uuid::new_16(0x183C), attribute, metadata,));

    let characteristic = characteristic.build();
    let notify_server: NotifyServer =
        NotifyServer::new(characteristic.cccd_handle, characteristic.value_handle);
    let notify_enabled = Mutex::new(Cell::new(false));

    // Start the BLE stack
    unwrap!(spawner.spawn(softdevice_task(sd)));

    // unwrap!(spawner.spawn(sensor_task(sensor)));
    // unwrap!(spawner.spawn(add_empty_measurements()));

    let adv_data: AdvertisementPayload<_> = ExtendedAdvertisementBuilder::new()
        .flags(&[Flag::GeneralDiscovery, Flag::LE_Only])
        .services_16(ServiceList::Complete, &[service_id])
        .full_name("Notify1")
        .build();

    let advertising = peripheral::ConnectableAdvertisement::ScannableUndirected {
        adv_data: &adv_data,
        scan_data: &SCAN_DATA,
    };

    let ble_peripheral_config = nrf_softdevice::ble::peripheral::Config::default();
    info!("advertising done");

    #[allow(clippy::empty_loop)]
    loop {
        notify_enabled.lock(|flag| flag.set(false));

        let conn = unwrap!(
            peripheral::advertise_connectable(sd, advertising, &ble_peripheral_config).await
        );
        info!("advertising done");

        let notify_enabled_ref = &notify_enabled;
        let gatt_server_future = gatt_server::run(&conn, &notify_server, move |e| {
            info!("event {:?} received", e);
            match e {
                notify_server::NotifyEvent::NotifyEnabled => {
                    info!("Notifications enabled");
                    notify_enabled_ref.lock(|flag| flag.set(true));
                }
            }
        });
        let notify_task_future = notify_task(&conn, characteristic.value_handle, &notify_enabled);

        pin_mut!(gatt_server_future);
        pin_mut!(notify_task_future);

        select(gatt_server_future, notify_task_future).await;
    }
}
