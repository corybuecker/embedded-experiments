use anyhow::{Result, anyhow};
use embedded_hal::i2c::I2c;
use linux_embedded_hal::I2cdev;
use std::error::Error;
use tracing::{debug, info};
use tracing_subscriber::{field::debug, layer::SubscriberExt};

fn main() -> Result<(), Box<dyn Error>> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(tracing_subscriber::fmt::layer().pretty()),
    )
    .expect("setting default subscriber failed");

    let mut i2c = I2cdev::new("/dev/i2c-1")?;
    let device_address: u8 = 0x40; // Adafruit INA228, A0 and A1 tied to GND, 1000000, R/W low

    reset_device(&mut i2c, device_address)?;
    read_config_register(&mut i2c, device_address)?;
    read_adc_config(&mut i2c, device_address)?;

    read_shunt_calibration(&mut i2c, device_address)?;
    calibrate_shunt_resistor(&mut i2c, device_address)?;
    read_shunt_calibration(&mut i2c, device_address)?;

    // println!("{}", read_current(&mut i2c, device_address)?);

    loop {
        std::thread::sleep(std::time::Duration::from_millis(50));
        println!("{}", read_current(&mut i2c, device_address)?);
        // println!("{}", read_voltage(&mut i2c, device_address)?);
    }

    Ok(())
}

fn read_voltage(i2c: &mut I2cdev, address: u8) -> Result<f32> {
    let register: u8 = 0x04; // Shunt voltage register
    let mut buffer = [0u8; 3];

    i2c.write_read(address, &[register], &mut buffer)?;

    let raw_24bit = ((buffer[0] as u32) << 16) | ((buffer[1] as u32) << 8) | (buffer[2] as u32);
    debug!("Raw 24-bit value: 0x{:06X}", raw_24bit);

    // Extract 20-bit signed value from bits 23-4 (shift right by 4)
    let raw_20bit = ((raw_24bit >> 4) & 0x000F_FFFF) as i32;

    let nanovolts = raw_20bit as f32 * 312.5;

    Ok(nanovolts)
}

fn read_current(i2c: &mut I2cdev, address: u8) -> Result<f32> {
    let register: u8 = 0x07; // Current register
    let mut buffer = [0u8; 3];

    i2c.write_read(address, &[register], &mut buffer)?;

    let raw_24bit = u32::from_be_bytes([0, buffer[0], buffer[1], buffer[2]]);

    let raw_20bit = (raw_24bit >> 4) as i32 & 0xFFFFF;

    let current_lsb = current_lsb();
    let current = raw_20bit as f32;

    let amperes = current * current_lsb;
    let milliamperes = amperes * 1000.0;

    Ok(milliamperes)
}

fn reset_device(i2c: &mut I2cdev, address: u8) -> Result<()> {
    let reset_register: u8 = 0x00; // 0000 0000
    let reset_device_command: [u8; 3] = [reset_register, 0x80, 0x00];

    let mut register_buffer = [0u8; 2];

    i2c.write(address, &reset_device_command)?;
    i2c.write_read(address, &[reset_register], &mut register_buffer)?;

    if !register_buffer.eq(&[0x00, 0x00]) {
        return Err(anyhow!("Failed to reset device"));
    }

    std::thread::sleep(std::time::Duration::from_millis(10));

    info!("Device reset successfully");

    Ok(())
}

fn read_config_register(i2c: &mut I2cdev, address: u8) -> Result<()> {
    let register: u8 = 0x00; // Configuration register
    let mut buffer = [0u8; 2];

    i2c.write_read(address, &[register], &mut buffer)?;

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

fn read_adc_config(i2c: &mut I2cdev, address: u8) -> Result<()> {
    let register: u8 = 0x01; // Configuration register
    let mut buffer = [0u8; 2];

    i2c.write_read(address, &[register], &mut buffer)?;

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

fn read_shunt_calibration(i2c: &mut I2cdev, address: u8) -> Result<()> {
    let register: u8 = 0x02; // Shunt calibration register
    let mut buffer = [0u8; 2];

    i2c.write_read(address, &[register], &mut buffer)?;

    let shunt_cal: u16 = u16::from_be_bytes([buffer[0], buffer[1]]);

    // Extract SHUNT_CAL field (bits 14-0), bit 15 is reserved and should be ignored
    let shunt_cal = shunt_cal & 0x7FFF; // 0x7FFF = 0111111111111111 (clears bit 15)

    debug!(
        "Shunt calibration: 0x{:X} (decimal {})",
        shunt_cal, shunt_cal
    );

    Ok(())
}

fn calibrate_shunt_resistor(i2c: &mut I2cdev, address: u8) -> Result<()> {
    let register: u8 = 0x02; // Shunt calibration register
    let current_lsb = current_lsb();
    let r_shunt = (15.0 / 1000.0) as f32;

    let value = 13107.2e6 * current_lsb * r_shunt;
    let value = value as u16;
    let value: [u8; 2] = value.to_be_bytes();

    let register_and_value = [register, value[0], value[1]];

    i2c.write(address, &register_and_value)?;

    Ok(())
}

fn current_lsb() -> f32 {
    let divisor = 2 as u32;
    let divisor = divisor.pow(19);
    let divisor = divisor as f32;

    10.0 / divisor
}
