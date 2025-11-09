use anyhow::{Result, anyhow};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use esp_radio::ble::controller::BleConnector;
use event_storage::storage::Events;
use trouble_host::prelude::Uuid;
use trouble_host::prelude::*;

pub async fn advertise_and_handle_connection(
    events: &Events,
    peripheral: &mut Peripheral<'_, ExternalController<BleConnector<'_>, 1>, DefaultPacketPool>,
) -> Result<()> {
    let mut advertising_data: [u8; 32] = [0; 32];

    let encoded_advertising_data_length = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE),
            AdStructure::ServiceUuids16(&[[0, 0]]),
            AdStructure::CompleteLocalName("ESPBeacon".as_bytes()),
        ],
        &mut advertising_data,
    )
    .map_err(|e| anyhow!("could not encode advertising data error={:?}", e))?;

    defmt::debug!(
        "encoded_advertising_data_length={}",
        encoded_advertising_data_length
    );

    loop {
        let advertiser = peripheral
            .advertise(
                &AdvertisementParameters::default(),
                Advertisement::ConnectableScannableUndirected {
                    adv_data: &advertising_data,
                    scan_data: &[],
                },
            )
            .await
            .map_err(|e| anyhow!("could not start advertising error={:?}", e))?;

        let connection = advertiser
            .accept()
            .await
            .map_err(|e| anyhow!("could not connect to central error={:?}", e))?;

        upgrade_connection_and_handle_events(connection, events).await?
    }
}

async fn upgrade_connection_and_handle_events<P: PacketPool>(
    connection: Connection<'_, P>,
    events: &Events,
) -> Result<()> {
    let mut characteristic_storage: [u8; 16] = [0; 16];
    let mut table: AttributeTable<'_, NoopRawMutex, 4> = AttributeTable::new();

    let service = Service::new(Uuid::new_short(0));
    let mut service_builder = table.add_service(service);

    let characteristic_builder = service_builder.add_characteristic(
        Uuid::new_short(1),
        &[CharacteristicProp::Notify],
        characteristic_storage,
        &mut characteristic_storage,
    );
    let characteristic_handle = characteristic_builder.build();
    let _service_handle = service_builder.build();
    let server: AttributeServer<'_, NoopRawMutex, P, 4, 1, 1> = AttributeServer::new(table);
    let upgraded_connection = connection
        .with_attribute_server(&server)
        .map_err(|e| anyhow!("could not upgrade connection error={:?}", e))?;

    gatt_events_task(&upgraded_connection, characteristic_handle, events).await?;

    Ok(())
}

async fn gatt_events_task<P: PacketPool>(
    connection: &GattConnection<'_, '_, P>,
    characteristic_handle: Characteristic<[u8; 16]>,
    events: &Events,
) -> Result<()> {
    let _reason = loop {
        match connection.next().await {
            GattConnectionEvent::Disconnected { reason } => break reason,
            GattConnectionEvent::Gatt { event } => match event {
                GattEvent::Write(write_event) => {
                    let should_notify = write_event.data().len() == 2
                        && u16::from_le_bytes([write_event.data()[0], write_event.data()[1]])
                            == 0x0001;

                    let reply = write_event
                        .accept()
                        .map_err(|e| anyhow!("could not accept write event error={:?}", e))?;
                    reply.send().await;

                    if should_notify {
                        let bytes = events.as_bytes().await;
                        let _result = characteristic_handle.notify(connection, &bytes).await;
                    }
                }
                _ => {}
            },
            _ => {}
        }
    };

    Ok(())
}
