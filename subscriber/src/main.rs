use anyhow::anyhow;
use btleplug::{
    api::Manager,
    platform::{self},
};
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

    let manager = platform::Manager::new().await.unwrap();
    let adapters = manager.adapters().await?;
    let adapter = adapters
        .first()
        .ok_or(anyhow!("could not find any adapters"))?;

    debug!("Central adapter: {:?}", adapter);

    let peripheral = connect(adapter).await?;
    info!("Connected to peripheral: {:?}", peripheral);
    wait_for_notify(&peripheral).await?;

    Ok(())
}
