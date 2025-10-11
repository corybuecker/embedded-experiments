#![no_std]
#![no_main]

mod common;
mod usb;

use defmt::{info, unwrap, warn};
use embassy_executor::Spawner;
use embassy_time::Timer;
use embassy_usb::driver::EndpointError;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let perihperals = embassy_rp::init(Default::default());

    let mut usb = usb::create_usb_device(perihperals.USB);
    unwrap!(spawner.spawn(usb::usb_task(usb.usb_device)));

    loop {
        info!("Waiting for USB Connection");
        usb.class.wait_connection().await;
        info!("USB Connected");

        let message = b"Hello world";
        loop {
            match usb.class.write_packet(message).await {
                Ok(_) => {}
                Err(EndpointError::Disabled) => {
                    info!("USB Disconnected");
                    break;
                }
                Err(e) => {
                    warn!("Write error: {:?}", e);
                }
            }

            Timer::after_millis(1000).await;
        }
    }
}
