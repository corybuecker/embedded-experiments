#![no_std]
#![no_main]

mod common;

use defmt::info;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use esp_radio::Controller;
use esp_radio::ble::controller::BleConnector;
use static_cell::StaticCell;
use trouble_host::prelude::*;
use trouble_host::{Address, HostResources};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    rtt_target::rtt_init_defmt!();
    esp_alloc::heap_allocator!(size: 72 * 1024);
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
    AdStructure::encode_slice(
        &[AdStructure::CompleteLocalName("Beacon1".as_bytes())],
        &mut advertising_data,
    );

    join(
        runner.run(),
        peripheral.advertise(
            &Default::default(),
            Advertisement::NonconnectableScannableUndirected {
                adv_data: &advertising_data,
                scan_data: &[],
            },
        ),
    )
    .await;

    loop {}
}
