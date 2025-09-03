use rusb::{Context, UsbContext};

fn main() {
    let version = rusb::version();

    println!(
        "libusb v{}.{}.{}.{}{}",
        version.major(),
        version.minor(),
        version.micro(),
        version.nano(),
        version.rc().unwrap_or("")
    );

    let mut context = match rusb::Context::new() {
        Ok(c) => c,
        Err(e) => panic!("libusb::Context::new(): {}", e),
    };

    context.set_log_level(rusb::LogLevel::Debug);
    context.set_log_level(rusb::LogLevel::Info);
    context.set_log_level(rusb::LogLevel::Warning);
    context.set_log_level(rusb::LogLevel::Error);
    context.set_log_level(rusb::LogLevel::None);

    println!("has capability? {}", rusb::has_capability());
    println!("has hotplug? {}", rusb::has_hotplug());
    println!("has HID access? {}", rusb::has_hid_access());
    println!(
        "supports detach kernel driver? {}",
        rusb::supports_detach_kernel_driver()
    );
    let context = Context::new().expect("Failed to create USB context");

    for device in context.devices().expect("Failed to list devices").iter() {
        let device_desc = device
            .device_descriptor()
            .expect("Failed to get device descriptor");


        // Replace these IDs with your CDC device's vendor/product IDs
        if device_desc.vendor_id() == 0xc0de && device_desc.product_id() == 0xcafe {
            let mut handle = device.open().expect("Failed to open device");

            for i in device.config_descriptor(0).unwrap().interfaces() {
                for j in i.descriptors() {
                    for k in j.endpoint_descriptors() {
                        println!("{:?}", k);
                    }
                }
            }
            

            // Claim interface 0 (CDC usually uses 0 or 1)
            handle
                .claim_interface(1)
                .expect("Failed to claim interface");

            // Replace 0x81 with your CDC IN endpoint address
            let mut buf = [0u8; 64];
            match handle.read_bulk(0x82, &mut buf, std::time::Duration::from_secs(1)) {
                Ok(len) => println!("Read {} bytes: {:?}", len, &buf[..len]),
                Err(e) => println!("Read error: {:?}", e),
            }
        }
    }
}
