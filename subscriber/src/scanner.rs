use anyhow::{Result, anyhow};
use btleplug::{
    api::{Central, Peripheral as _, ScanFilter},
    platform::{Adapter, Peripheral},
};
use std::{str::FromStr, time::Duration};
use tokio::{
    spawn,
    time::{sleep, timeout},
};
use tokio_stream::StreamExt;
use tracing::{debug, error};
use uuid::Uuid;

static SERVICE_UUID: &str = "0000183b-0000-1000-8000-00805f9b34fb";
static BLUETOOTH_ADVERTISING_INTERVAL: u64 = 1280; // in milliseconds

pub async fn connect(adapter: &Adapter) -> Result<Peripheral> {
    let join_handle = spawn(connect_to_peripheral(adapter.clone()));

    match timeout(Duration::from_millis(5000), join_handle).await {
        Ok(Ok(peripheral)) => Ok(peripheral?),
        Ok(Err(e)) => Err(anyhow!("Error connecting to device: {e}")),
        Err(_) => Err(anyhow!("Timeout connecting to device")),
    }
}

async fn connect_to_peripheral(adapter: Adapter) -> Result<Peripheral> {
    adapter.start_scan(filter()?).await?;
    sleep(Duration::from_millis(BLUETOOTH_ADVERTISING_INTERVAL)).await;
    adapter.stop_scan().await?;

    let peripherals = adapter.peripherals().await.unwrap();

    let peripheral = peripherals.first();

    match peripheral {
        Some(peripheral) => {
            let peripheral = peripheral.clone();
            peripheral.connect().await?;
            peripheral.discover_services().await?;

            let services = peripheral.services();

            let service_uuid = Uuid::from_str(SERVICE_UUID)?;
            let service = services
                .into_iter()
                .find(|s| s.uuid == service_uuid)
                .ok_or(anyhow!("could not find the service"))?;

            debug!("Discovered service: {:?}", service);

            let characteristics = service.characteristics;
            let characteristic = characteristics
                .into_iter()
                .next()
                .ok_or(anyhow!("could not find a characteristic"))?;

            peripheral.subscribe(&characteristic).await?;
            Ok(peripheral)
        }
        None => Err(anyhow!("No peripheral found")),
    }
}

pub async fn wait_for_notify(peripheral: &Peripheral) -> Result<usize> {
    let join_handle = spawn(collect_samples(peripheral.clone()));

    match timeout(Duration::from_millis(10000), join_handle).await {
        Ok(Ok(samples)) => Ok(samples?),
        Ok(Err(e)) => Err(anyhow!("Error collecting samples: {e}")),
        Err(_) => Err(anyhow!("Timeout collecting samples")),
    }
}

async fn collect_samples(peripheral: Peripheral) -> Result<usize> {
    let notifications = peripheral
        .notifications()
        .await?
        .take(3)
        .map(|s| s.value)
        .map(|samples| -> Result<[u8; 1], anyhow::Error> {
            if samples.len() != 1 {
                error!("Insufficient data: expected 1 byte, got {:?}", samples);
                return Err(anyhow!("Insufficient data"));
            }
            let sample: [u8; 1] = samples[..1].try_into()?;
            Ok(sample)
        })
        .collect::<Vec<Result<[u8; 1], _>>>()
        .await
        .into_iter()
        .collect::<Result<Vec<[u8; 1]>, _>>()?;

    debug!("Notifications: {:?}", notifications);

    let notifications: Vec<u8> = notifications
        .iter()
        .map(|s| u8::from_le_bytes(*s))
        .collect();

    debug!("Notifications: {:?}", notifications);

    Ok(notifications.len())
}

fn filter() -> Result<ScanFilter> {
    // Ok(ScanFilter::default())
    Ok(ScanFilter {
        services: vec![Uuid::from_str(SERVICE_UUID)?],
    })
}
