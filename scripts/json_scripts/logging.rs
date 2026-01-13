use color_eyre::eyre::Result;

/// Initialize terminal logger with debug or info level filtering for
/// releasaurus output.
pub fn initialize_logger(debug: bool) -> Result<()> {
    let filter = if debug {
        simplelog::LevelFilter::Debug
    } else {
        simplelog::LevelFilter::Info
    };

    let config = simplelog::ConfigBuilder::new()
        .add_filter_allow_str("releasaurus-slack")
        .build();

    simplelog::TermLogger::init(
        filter,
        config,
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )?;

    Ok(())
}
