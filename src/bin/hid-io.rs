#[macro_use]
extern crate log;

use clap::App;
use hid_io::{api, built_info, device, module};

/// Main entry point
fn main() {
    // Setup logging mechanism
    env_logger::init();

    // Process command-line arguments
    // Most of the information is generated from Cargo.toml using built crate (build.rs)
    App::new(format!("{}", built_info::PKG_NAME))
        .version(
            format!(
                "{}{} - {}",
                built_info::PKG_VERSION,
                built_info::GIT_VERSION.map_or_else(|| "".to_owned(), |v| format!(" (git {})", v)),
                built_info::PROFILE,
            )
            .as_str(),
        )
        .author(built_info::PKG_AUTHORS)
        .about(format!("\n{}", built_info::PKG_DESCRIPTION,).as_str())
        .after_help(
            format!(
                "{} ({}) -> {} ({})",
                built_info::RUSTC_VERSION,
                built_info::HOST,
                built_info::TARGET,
                built_info::BUILT_TIME_UTC,
            )
            .as_str(),
        )
        .get_matches();

    // Start initialization
    info!("Initializing HID-IO daemon...");

    // Initialize Modules
    module::initialize();

    // Initialize Devices
    device::initialize();

    // Initialize Cap'n'Proto API Server
    api::initialize();

    // XXX (jacob) Is an infinite loop needed here?
    //loop {
    //    thread::sleep(time::Duration::from_millis(2000));
    //}

    /*
    debug!("Debug message");
    error!("Error message");
    warn!("Warn message");
    trace!("Trace message");
    */
}
