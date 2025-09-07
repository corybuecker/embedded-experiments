use crate::storage::{Measurements, SharedMeasurements};
use core::cell::Cell;
use defmt::{error, info};
use embassy_nrf::gpio::Input;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::blocking_mutex::{CriticalSectionMutex, Mutex};
use embassy_time::Timer;
use nrf_softdevice::ble::gatt_server::notify_value;
use nrf_softdevice::{Softdevice, ble::Connection};

static STORAGE: SharedMeasurements = CriticalSectionMutex::new(Measurements::new());

#[embassy_executor::task]
pub async fn sensor_task(mut sensor: Input<'static>) {
    loop {
        STORAGE.lock(|m| m.unlock());
        sensor.wait_for_high().await;
        STORAGE.lock(|m| m.lock());
        while sensor.is_high() {
            STORAGE.lock(|m| m.add(0x1));
            Timer::after_millis(100).await;
        }
    }
}

#[embassy_executor::task]
pub async fn add_empty_measurements() {
    loop {
        STORAGE.lock(|m| m.add_if_unlocked(0x0));
        Timer::after_millis(100).await;
    }
}

#[embassy_executor::task]
pub async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}

pub async fn notify_task<'a>(
    conn: &'a Connection,
    value_handle: u16,
    notify_enabled: &'a Mutex<NoopRawMutex, Cell<bool>>,
) -> ! {
    loop {
        if notify_enabled.lock(|flag| flag.get()) {
            let sum = STORAGE.lock(|m| m.sum());

            match notify_value(conn, value_handle, &[sum]) {
                Ok(()) => {}
                Err(e) => {
                    error!("Failed to notify value: {}", e);
                }
            }
        }
        Timer::after_millis(250).await;
    }
}
