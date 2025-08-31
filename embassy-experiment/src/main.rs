#![no_std]
#![no_main]

pub mod notify_server;

use crate::notify_server::NotifyServer;
use core::cell::Cell;
use defmt::{error, info, *};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::{self as _, config, gpio::Input, gpiote::InputChannel, interrupt::Priority};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::Timer;
use futures::{future::select, pin_mut};
use nrf_softdevice::{
    self as _, Softdevice,
    ble::{
        Connection, Uuid,
        advertisement_builder::{
            AdvertisementPayload, ExtendedAdvertisementBuilder, Flag, ServiceList, ServiceUuid16,
        },
        gatt_server::{
            self,
            builder::ServiceBuilder,
            characteristic::{self, Properties},
            notify_value,
        },
        peripheral,
    },
};
use panic_probe as _;

async fn notify_task<'a>(
    conn: &'a Connection,
    value_handle: u16,
    notify_enabled: &'a Mutex<NoopRawMutex, Cell<bool>>,
) -> ! {
    loop {
        if notify_enabled.lock(|flag| flag.get()) {
            match notify_value(conn, value_handle, &[0x1]) {
                Ok(()) => {}
                Err(e) => {
                    error!("Failed to notify value: {}", e);
                }
            }
        }
        Timer::after_millis(250).await;
    }
}

#[embassy_executor::task]
async fn button_task(mut button: InputChannel<'static>) {
    loop {
        button.wait().await;
        defmt::info!("Button pressed!");

        // Debounce delay
        Timer::after_millis(50).await;
    }
}

#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}

static SCAN_DATA: [u8; 0] = [];

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let mut config = config::Config::default();
    config.gpiote_interrupt_priority = Priority::P2;
    config.time_interrupt_priority = Priority::P2;
    let peripherals = embassy_nrf::init(config);
    let button = Input::new(peripherals.P1_06, embassy_nrf::gpio::Pull::Up);
    let button_channel = InputChannel::new(
        peripherals.GPIOTE_CH0,
        button,
        embassy_nrf::gpiote::InputChannelPolarity::HiToLo,
    );

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

    unwrap!(spawner.spawn(softdevice_task(sd)));
    unwrap!(spawner.spawn(button_task(button_channel)));

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
        notify_enabled.lock(|flag| flag.set(false));

        let conn = unwrap!(peripheral::advertise_connectable(sd, advertising, &config).await);
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
