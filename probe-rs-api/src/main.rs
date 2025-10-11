use anyhow::Result;
use probe_rs::Permissions;
use probe_rs::flashing::{FlashProgress, Format, download_file, erase_all};
use probe_rs::probe::list::Lister;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::new("info")
                .add_directive("probe_rs::flashing=debug".parse().unwrap()),
        )
        .init();

    let lister = Lister::new();

    let probes = lister.list_all();

    // Use the first probe found.
    let probe = probes[0].open()?;

    let mut progress = FlashProgress::new(|_progress| {
        println!("Progress: ");
    });

    // Attach to a chip.
    let mut session = probe.attach("nRF52840_xxAA", Permissions::default().allow_erase_all())?;

    erase_all(&mut session, &mut progress)?;

    download_file(
        &mut session,
        std::path::Path::new("s140_nrf52_7.3.0_softdevice.hex"),
        Format::Hex,
    )?;

    Ok(())
}
