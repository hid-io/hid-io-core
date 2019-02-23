#[macro_use]
extern crate log;

use clap::App;
use hid_io::{api, built_info, device, module};

use std::sync::mpsc::channel;
use crate::device::hidusb::HIDIOMailbox;
use crate::device::hidusb::HIDIOMailer;
use crate::device::hidusb::HIDIOMessage;

use hid_io::RUNNING;
use std::sync::atomic::Ordering;

/// Main entry point
fn main() {
    // Setup logging mechanism
    pretty_env_logger::init();

    // Setup signal handler
    let r = RUNNING.clone();
    ctrlc::set_handler(move || {
	r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");
    println!("Press Ctrl-C to exit...");

    // Process command-line arguments
    // Most of the information is generated from Cargo.toml using built crate (build.rs)
    App::new(built_info::PKG_NAME.to_string())
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
        .about(format!("\n{}", built_info::PKG_DESCRIPTION).as_str())
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

    let (mailer_writer, mailer_reader) = channel::<HIDIOMessage>();
    let mut mailer = HIDIOMailer::new(mailer_reader);

    let (sink1, mailbox1) = HIDIOMailbox::from_sender(mailer_writer.clone());
    mailer.register_listener(sink1);

    let (sink2, mailbox2) = HIDIOMailbox::from_sender(mailer_writer.clone());
    mailer.register_listener(sink2);

    // Initialize Modules
    let a = module::initialize(mailbox2);

    // Initialize Devices
    device::initialize(mailer);

    // Initialize Cap'n'Proto API Server
    api::initialize(mailbox1);

    // Cleanup
    while RUNNING.load(Ordering::SeqCst) {}
    println!("Waiting for threads to finish...");
    a.join().unwrap();
    println!("Exiting.");

    /*
    debug!("Debug message");
    error!("Error message");
    warn!("Warn message");
    trace!("Trace message");
    */
}
