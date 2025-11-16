use embassy_time::Timer;
use esp_hal::{
    Async,
    gpio::Level,
    peripherals::{GPIO8, RMT},
    rmt::{Channel, PulseCode, Rmt, Tx, TxChannelConfig, TxChannelCreator},
    time::Rate,
};
use heapless::Vec;

const LOW: PulseCode = PulseCode::new(Level::High, 7, Level::Low, 16);
const HIGH: PulseCode = PulseCode::new(Level::High, 14, Level::Low, 12);

pub async fn create_channel<'channel>(
    rmt: RMT<'channel>,
    pin: GPIO8<'channel>,
) -> Channel<'channel, Async, Tx> {
    let rmt = Rmt::new(rmt, Rate::from_mhz(80)).unwrap().into_async();

    let tx_config = TxChannelConfig::default()
        .with_clk_divider(4)
        .with_idle_output(true)
        .with_idle_output_level(Level::Low)
        .with_memsize(2);

    let mut channel = rmt.channel0.configure_tx(pin, tx_config).unwrap();

    // More investigation is needed here. Without this pause, the LED will
    // always drive high on all bits for a short period before turning off. I
    // wonder if the RMT subsystem is not properly initialized.
    Timer::after_micros(50).await;

    if let Some(pulses) = rgb_to_pulses((0, 0, 0)).ok() {
        #[allow(unused)]
        channel.transmit(&pulses).await;
    }

    channel
}

pub async fn off(channel: &mut Channel<'static, Async, Tx>) -> Result<(), esp_hal::rmt::Error> {
    let pulses = rgb_to_pulses((0, 0, 0)).map_err(|_e| esp_hal::rmt::Error::InvalidDataLength)?;
    channel.transmit(&pulses).await
}

// pub async fn blue_led(
//     channel: &mut Channel<'static, Async, Tx>,
// ) -> Result<(), esp_hal::rmt::Error> {
//     let pulses = rgb_to_pulses((0, 0, 255)).map_err(|_e| esp_hal::rmt::Error::InvalidDataLength)?;
//     channel.transmit(&pulses).await
// }

pub async fn red_led(channel: &mut Channel<'static, Async, Tx>) -> Result<(), esp_hal::rmt::Error> {
    let pulses = rgb_to_pulses((10, 0, 0)).map_err(|_e| esp_hal::rmt::Error::InvalidDataLength)?;
    channel.transmit(&pulses).await
}

fn rgb_to_pulses((r, g, b): (u8, u8, u8)) -> Result<Vec<PulseCode, 26>, PulseCode> {
    let mut bits: Vec<PulseCode, 26> = [g, r, b]
        .into_iter()
        .flat_map(|byte| {
            (0..8).rev().map(move |bit: u8| {
                if byte >> bit & 0b00000001 == 1 {
                    HIGH
                } else {
                    LOW
                }
            })
        })
        .collect();

    bits.push(PulseCode::new(Level::Low, 20, Level::Low, 20))?;
    bits.push(PulseCode::end_marker())?;

    Ok(bits)
}
