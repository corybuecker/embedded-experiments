use anyhow::anyhow;
use btleplug::api::Manager as _;
use btleplug::platform::Manager;
use std::error::Error;
use tracing::{debug, info};
use tracing_subscriber::layer::SubscriberExt;

use crate::scanner::{connect, wait_for_notify};

mod scanner;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let stdout_log = tracing_subscriber::fmt::layer().pretty();
    let subscriber = tracing_subscriber::registry().with(stdout_log);
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let manager = Manager::new().await.unwrap();
    let adapters = manager.adapters().await?;
    let adapter = adapters
        .first()
        .ok_or(anyhow!("could not find any adapters"))?;

    debug!("Central adapter: {:?}", adapter);

    // adapter.start_scan(ScanFilter::default()).await?;

    // let mut events = adapter.events().await?;

    // while let Some(event) = events.next().await {
    //     match event {
    //         // CentralEvent::DeviceUpdated(id) => {
    //         //     // let peripheral = adapter.peripheral(&id).await?;
    //         //     // let properties = peripheral.properties().await?.unwrap();

    //         //     // debug!("Device updated: {:#?}", properties);
    //         // }
    //         CentralEvent::ServiceDataAdvertisement { id, service_data } => {
    //             debug!(
    //                 "Received service data advertisement: id = {:?}, data = {:?}",
    //                 id, service_data
    //             );
    //         }
    //         // CentralEvent::ManufacturerDataAdvertisement {
    //         //     id,
    //         //     manufacturer_data,
    //         // } => {
    //         //     // debug!(
    //         //     //     "Received manufacturer data advertisement: id = {:?}, data = {:?}",
    //         //     //     id, manufacturer_data
    //         //     // );
    //         // }
    //         _ => {
    //             // debug!("Received event: {:?}", event);
    //         }
    //     }
    // }

    let peripheral = connect(adapter).await?;
    info!("Connected to peripheral: {:?}", peripheral);
    wait_for_notify(&peripheral).await?;

    Ok(())
}
