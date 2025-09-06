#![no_std]
#![no_main]

mod common;

use defmt::debug;
use defmt::info;
use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_rp::i2c::Blocking;
use embassy_rp::i2c::I2c;
use embassy_rp::i2c::{self, Config};
use embassy_rp::peripherals::I2C0;
use embassy_time::Timer;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let p = embassy_rp::init(Default::default());

    let mut i2c = I2c::new_blocking(p.I2C0, p.PIN_1, p.PIN_0, Config::default());
    let device_address: u8 = 0x40; // Adafruit INA228, A0 and A1 tied to GND, 1000000, R/W low

    unwrap!(reset_device(&mut i2c, device_address).await);
    unwrap!(read_config_register(&mut i2c, device_address).await);
    unwrap!(read_adc_config(&mut i2c, device_address).await);
    unwrap!(calibrate_shunt_resistor(&mut i2c, device_address).await);
    unwrap!(read_shunt_calibration(&mut i2c, device_address).await);

    loop {
        Timer::after_millis(500).await;
        let reading = unwrap!(read_current(&mut i2c, device_address).await);
        info!("{}mA", reading);
    }
}

async fn reset_device(
    i2c: &mut i2c::I2c<'_, I2C0, Blocking>,
    device_address: u8,
) -> Result<(), i2c::Error> {
    let reset_register: u8 = 0x00; // 0000 0000
    let reset_device_command: [u8; 3] = [reset_register, 0x80, 0x00];

    let mut register_buffer = [0u8; 2];

    i2c.blocking_write_read(device_address, &reset_device_command, &mut register_buffer)?;

    if !register_buffer.eq(&[0x00, 0x00]) {}

    Timer::after_millis(10).await;
    info!("Device reset successfully");

    Ok(())
}

async fn read_bus_voltage(
    i2c: &mut I2c<'_, I2C0, Blocking>,
    address: u8,
) -> Result<f32, i2c::Error> {
    let register: u8 = 0x05; // Bus voltage register
    let mut buffer = [0u8; 3];

    i2c.blocking_write_read(address, &[register], &mut buffer)?;

    let raw_24bit = u32::from_be_bytes([0, buffer[0], buffer[1], buffer[2]]);

    // Extract 20-bit signed value from bits 23-4 (shift right by 4)
    let raw_20bit = (raw_24bit >> 4) as i32 & 0xFFFFF;

    let microvolts = raw_20bit as f32 * 195.3125;
    let volts = microvolts / 1_000_000.0;

    Ok(volts)
}

async fn read_shunt_voltage(
    i2c: &mut I2c<'_, I2C0, Blocking>,
    address: u8,
) -> Result<f32, i2c::Error> {
    let register: u8 = 0x04; // Shunt voltage register
    let mut buffer = [0u8; 3];

    i2c.blocking_write_read(address, &[register], &mut buffer)?;

    let raw_24bit = u32::from_be_bytes([0, buffer[0], buffer[1], buffer[2]]);

    // Extract 20-bit signed value from bits 23-4 (shift right by 4)
    let raw_20bit = (raw_24bit >> 4) as i32 & 0xFFFFF;

    let nanovolts = raw_20bit as f32 * 312.5;
    let volts = nanovolts / 1_000_000_000.0;

    Ok(volts)
}
async fn read_current(i2c: &mut I2c<'_, I2C0, Blocking>, address: u8) -> Result<f32, i2c::Error> {
    let register: u8 = 0x07; // Current register
    let mut buffer = [0u8; 3];

    i2c.blocking_write_read(address, &[register], &mut buffer)?;

    let raw_24bit = u32::from_be_bytes([0, buffer[0], buffer[1], buffer[2]]);

    let raw_20bit = (raw_24bit >> 4) as i32 & 0xFFFFF;

    let current_lsb = current_lsb();
    let current = raw_20bit as f32;

    let amperes = current * current_lsb;
    let milliamperes = amperes * 1000.0;

    Ok(milliamperes)
}

async fn read_config_register(
    i2c: &mut I2c<'_, I2C0, Blocking>,
    address: u8,
) -> Result<(), i2c::Error> {
    let register: u8 = 0x00; // Configuration register
    let mut buffer = [0u8; 2];

    i2c.blocking_write_read(address, &[register], &mut buffer)?;

    let config: u16 = u16::from_be_bytes([buffer[0], buffer[1]]);

    let rst = (config >> 15) & 0b1;
    debug!("RST: {:X}h", rst);

    let rstacc = (config >> 14) & 0b1;
    debug!("RSTACC: {:X}h", rstacc);

    let convdly = (config >> 6) & 0b111111;
    debug!("CONVDLY: {:X}h", convdly);

    let tempcomp = (config >> 5) & 0b1;
    debug!("TEMPCOMP: {:X}h", tempcomp);

    let adcrange = (config >> 4) & 0b1;
    debug!("ADCRANGE: {:X}h", adcrange);

    Ok(())
}

async fn read_adc_config(i2c: &mut I2c<'_, I2C0, Blocking>, address: u8) -> Result<(), i2c::Error> {
    let register: u8 = 0x01; // Configuration register
    let mut buffer = [0u8; 2];

    i2c.blocking_write_read(address, &[register], &mut buffer)?;

    let adc_config: u16 = u16::from_be_bytes([buffer[0], buffer[1]]);

    let mode = (adc_config >> 12) & 0b1111;
    debug!("MODE: {:X}h", mode);

    let vbuscnt = (adc_config >> 9) & 0b111;
    debug!("VBUSCNT: {:X}h", vbuscnt);

    let vshcnt = (adc_config >> 6) & 0b111;
    debug!("VSHCNT: {:X}h", vshcnt);

    let vtcnt = (adc_config >> 3) & 0b111;
    debug!("VTCNT: {:X}h", vtcnt);

    let avg = adc_config & 0b111;
    debug!("AVG: {:X}h", avg);

    Ok(())
}

async fn read_shunt_calibration(
    i2c: &mut I2c<'_, I2C0, Blocking>,
    address: u8,
) -> Result<(), i2c::Error> {
    let register: u8 = 0x02; // Shunt calibration register
    let mut buffer = [0u8; 2];

    i2c.blocking_write_read(address, &[register], &mut buffer)?;

    let shunt_cal: u16 = u16::from_be_bytes([buffer[0], buffer[1]]);

    // Extract SHUNT_CAL field (bits 14-0), bit 15 is reserved and should be ignored
    let shunt_cal = shunt_cal & 0x7FFF; // 0x7FFF = 0111111111111111 (clears bit 15)

    debug!(
        "Shunt calibration: 0x{:X} (decimal {})",
        shunt_cal, shunt_cal
    );

    Ok(())
}

async fn calibrate_shunt_resistor(
    i2c: &mut I2c<'_, I2C0, Blocking>,
    address: u8,
) -> Result<(), i2c::Error> {
    let register: u8 = 0x02; // Shunt calibration register
    let current_lsb = current_lsb();
    let r_shunt = 0.025 as f32;

    let value = 13107.2e6 * current_lsb * r_shunt;
    let value = value as u16;
    let value: [u8; 2] = value.to_be_bytes();

    let register_and_value = [register, value[0], value[1]];

    i2c.blocking_write(address, &register_and_value)?;

    Ok(())
}

fn current_lsb() -> f32 {
    let divisor = 2 as u32;
    let divisor = divisor.pow(19);
    let divisor = divisor as f32;

    2.5 / divisor
}
