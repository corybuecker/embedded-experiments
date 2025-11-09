#![no_std]
#![no_main]

mod common;

use core::future::pending;
use embassy_executor::Spawner;
use embassy_futures::select::select4;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::Timer;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Level};
use esp_hal::rtc_cntl::Rtc;
use esp_hal::timer::timg::TimerGroup;
use esp_radio::ble::controller::BleConnector;
use event_storage::storage::Events;
use static_cell::StaticCell;
use trouble_host::HostResources;
use trouble_host::prelude::Uuid;
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

    let mut rtc = Rtc::new(peripherals.LPWR);

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
        peripherals.GPIO10,
        InputConfig::default().with_pull(esp_hal::gpio::Pull::None),
    );

    let _ = select4(
        runner.run(),
        async {
            // let mut sleep_config = RtcSleepConfig::default();
            // Timer::after_secs(5).await;
            // esp_println::dbg!("going to sleep for five seconds");
            // let wakeup_source = TimerWakeupSource::new(Duration::from_millis(5000));
            // rtc.sleep(&sleep_config, &[&wakeup_source]);
            pending::<bool>().await
        },
        collect_events(&input, &events),
        advertising_loop(&events, &mut peripheral),
    )
    .await;

    pending().await
}

async fn advertising_loop(
    events: &Events,
    peripheral: &mut Peripheral<'_, ExternalController<BleConnector<'_>, 1>, DefaultPacketPool>,
) {
    let mut advertising_data: [u8; 32] = [0; 32];
    AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE),
            AdStructure::ServiceUuids16(&[[0, 0]]),
            AdStructure::CompleteLocalName("ESPBeacon".as_bytes()),
        ],
        &mut advertising_data,
    )
    .expect("could not encode scan data");

    loop {
        defmt::debug!("starting advertisement");
        let advertiser = peripheral
            .advertise(
                &AdvertisementParameters::default(),
                Advertisement::ConnectableScannableUndirected {
                    adv_data: &advertising_data,
                    scan_data: &[],
                },
            )
            .await
            .expect("could not create advertiser");

        match advertiser.accept().await {
            Ok(conn) => {
                let mut characteristic_storage: [u8; 2] = [0; 2];
                let mut table: AttributeTable<'_, NoopRawMutex, 4> = AttributeTable::new();
                let service = Service::new(Uuid::new_short(0));
                let mut service_builder = table.add_service(service);

                let characteristic_builder = service_builder.add_characteristic(
                    Uuid::new_short(1),
                    &[CharacteristicProp::Notify, CharacteristicProp::Read],
                    [1, 1],
                    &mut characteristic_storage,
                );

                let characteristic_handle = characteristic_builder.build();
                let _service_handle = service_builder.build();

                let server: AttributeServer<'_, NoopRawMutex, DefaultPacketPool, 4, 4, 1> =
                    AttributeServer::new(table);

                let conn_result = conn.with_attribute_server(&server).unwrap();

                gatt_events_task(&conn_result, characteristic_handle)
                    .await
                    .unwrap();
            }
            Err(err) => {}
        }
    }
}

async fn gatt_events_task<P: PacketPool>(
    conn: &GattConnection<'_, '_, P>,
    characteristic_handle: Characteristic<[u8; 2]>,
) -> Result<(), Error> {
    let _reason = loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => break reason,
            GattConnectionEvent::Gatt { event } => {
                match &event {
                    GattEvent::Other(e) => {
                        defmt::debug!("other event received");
                    }
                    GattEvent::Read(e) => {
                        defmt::debug!("read event");
                    }
                    GattEvent::Write(e) => {
                        let data = e.data();
                        if data.len() == 2 {
                            let value = u16::from_le_bytes([data[0], data[1]]);
                            match value {
                                0x0001 => {
                                    defmt::debug!("Central subscribed to notifications");
                                    match event.accept() {
                                        Ok(reply) => reply.send().await,
                                        Err(e) => {
                                            defmt::warn!("[gatt] error sending response: {:?}", e)
                                        }
                                    };
                                    // Send a test notification
                                    let notification_data: [u8; 2] = [1, 0];
                                    let result = characteristic_handle
                                        .notify(conn, &notification_data)
                                        .await;

                                    defmt::debug!("notification complete result={:?}", result);
                                }
                                0x0000 => {
                                    defmt::debug!("Central unsubscribed from notifications");
                                }
                                _ => {
                                    defmt::debug!("CCCD write with value: {}", value);
                                }
                            }
                        }
                    }
                };
            }
            _ => {}
        }
    };
    defmt::debug!("leaving event loop");
    Ok(())
}

async fn collect_events(input: &Input<'static>, events: &Events) {
    loop {
        match input.level() {
            Level::High => {
                events
                    .record(event_storage::storage::RecordType::High)
                    .await;
            }
            Level::Low => {
                events.record(event_storage::storage::RecordType::Low).await;
            }
        }

        Timer::after_millis(100).await;
    }
}
