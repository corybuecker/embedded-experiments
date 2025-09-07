use embassy_rp::{
    Peri, bind_interrupts,
    peripherals::USB,
    usb::{Driver, InterruptHandler},
};
use embassy_usb::{
    Config, UsbDevice,
    class::cdc_acm::{CdcAcmClass, State},
};
use static_cell::StaticCell;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

pub struct LocalUsb<'a> {
    pub usb_device: UsbDevice<'a, Driver<'a, USB>>,
    pub class: CdcAcmClass<'a, Driver<'a, USB>>,
}

#[embassy_executor::task]
pub async fn usb_task(mut usb: UsbDevice<'static, Driver<'static, USB>>) -> ! {
    usb.run().await
}

pub fn create_usb_device(usb: Peri<'static, USB>) -> LocalUsb<'static> {
    let driver = Driver::new(usb, Irqs);
    let config = Config::new(0xc0de, 0xcafe);

    let mut builder = {
        static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

        embassy_usb::Builder::new(
            driver,
            config,
            CONFIG_DESCRIPTOR.init([0; 256]),
            BOS_DESCRIPTOR.init([0; 256]),
            &mut [], // no msos descriptors
            CONTROL_BUF.init([0; 64]),
        )
    };
    static STATE: StaticCell<State> = StaticCell::new();
    let state = State::new();
    let state = STATE.init(state);

    let class = CdcAcmClass::new(&mut builder, state, 64);

    LocalUsb {
        usb_device: builder.build(),
        class,
    }
}
