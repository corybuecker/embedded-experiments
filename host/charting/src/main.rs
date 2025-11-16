use anyhow::Result;

mod logging;

#[tokio::main]
async fn main() -> Result<()> {
    logging::initialize_debug_logging();

    let term = console::Term::stdout();
    tracing::debug!("{:#?}", term);

    term.hide_cursor()?;
    term.clear_screen()?;

    Ok(())
}
