#![no_std]
#![no_main]

use defmt::{self as _, info};
use defmt_rtt as _;
use panic_probe as _;
use rp235x_hal as hal;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

#[hal::entry]
fn main() -> ! {
    info!("Hello, world!");

    #[warn(clippy::empty_loop)]
    loop {}
}
