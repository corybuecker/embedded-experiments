#![no_std]
#![no_main]

use core::future;
use defmt::{info, *};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::{self as _, config, gpio::Input, gpiote::InputChannel, interrupt::Priority};
use embassy_time::Timer;
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
use panic_probe as _;

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
        let _conn = unwrap!(peripheral::advertise_connectable(sd, advertising, &config).await);
        info!("advertising done");
        future::pending::<()>().await
    }
}
