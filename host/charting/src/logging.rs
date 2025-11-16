use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn initialize_debug_logging() {
    let layer = tracing_subscriber::fmt::layer().pretty();
    let level = tracing_subscriber::filter::LevelFilter::from_level(Level::DEBUG);
    let registry = tracing_subscriber::registry().with(level).with(layer);
    registry.init();
}
