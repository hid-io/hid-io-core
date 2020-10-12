/* Copyright (C) 2019-2020 by Jacob Alexander
 * Copyright (C) 2019 by Rowan Decker
 *
 * This file is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This file is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this file.  If not, see <http://www.gnu.org/licenses/>.
 */

#[macro_use]
extern crate log;

#[cfg(windows)]
#[macro_use]
extern crate windows_service;

use clap::App;
use hid_io_core::logging;
use hid_io_core::mailbox;
use hid_io_core::RUNNING;
use hid_io_core::{api, built_info, device, module};
use std::env;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};

#[cfg(windows)]
fn main() -> Result<(), std::io::Error> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() > 1 && args[1] == "-d" {
        info!("-------------------------- HID-IO Core starting! --------------------------");
        match service::run() {
            Ok(_) => (),
            Err(_e) => panic!("Service failed"),
        }
    } else {
        logging::setup_logging();
        start();
    }
    Ok(())
}

#[cfg(not(windows))]
fn main() -> Result<(), std::io::Error> {
    logging::setup_logging();
    start();
    Ok(())
}

/// Main entry point
#[tokio::main]
async fn start() {
    // Setup signal handler
    let r = RUNNING.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
    println!("Press Ctrl-C to exit...");

    let version_info = format!(
        "{}{} - {}",
        built_info::PKG_VERSION,
        built_info::GIT_VERSION.map_or_else(|| "".to_owned(), |v| format!(" (git {})", v)),
        built_info::PROFILE,
    );
    info!("Version: {}", version_info);
    let after_info = format!(
        "{} ({}) -> {} ({})",
        built_info::RUSTC_VERSION,
        built_info::HOST,
        built_info::TARGET,
        built_info::BUILT_TIME_UTC,
    );
    info!("Build: {}", after_info);

    // Process command-line arguments
    // Most of the information is generated from Cargo.toml using built crate (build.rs)
    App::new(built_info::PKG_NAME.to_string())
        .version(version_info.as_str())
        .author(built_info::PKG_AUTHORS)
        .about(format!("\n{}", built_info::PKG_DESCRIPTION).as_str())
        .after_help(after_info.as_str())
        .get_matches();

    // Start initialization
    info!("Initializing HID-IO daemon...");

    // Setup mailbox
    let mailbox = mailbox::Mailbox::new();

    // Wait until completion
    let (_, _, _) = tokio::join!(
        // Initialize Modules
        module::initialize(mailbox.clone()),
        // Initialize Device monitoring
        device::initialize(mailbox.clone()),
        // Initialize Cap'n'Proto API Server
        api::initialize(mailbox),
    );

    info!("-------------------------- HID-IO Core exiting! --------------------------");
}

#[cfg(windows)]
fn stop() {
    info!("Stopping!");
    let r = RUNNING.clone();
    r.store(false, Ordering::SeqCst);
}

#[cfg(windows)]
mod service {
    use flexi_logger::{opt_format, Logger};
    use hid_io_core::built_info;
    use windows_service::service_dispatcher;

    const SERVICE_NAME: &str = built_info::PKG_NAME;

    // Generate the windows service boilerplate.
    // The boilerplate contains the low-level service entry function (ffi_service_main) that parses
    // incoming service arguments into Vec<OsString> and passes them to user defined service
    // entry (my_service_main).
    define_windows_service!(ffi_service_main, my_service_main);

    use std::ffi::OsString;
    use std::time::Duration;
    use windows_service::service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};

    pub fn run() -> windows_service::Result<()> {
        // Register generated `ffi_service_main` with the system and start the service, blocking
        // this thread until the service is stopped.
        service_dispatcher::start(SERVICE_NAME, ffi_service_main)
    }

    fn my_service_main(arguments: Vec<OsString>) {
        Logger::with_env()
            .log_to_file()
            .directory("log_files")
            .format(opt_format)
            .start()
            .unwrap_or_else(|e| panic!("Logger initialization failed {}", e));
        info!("Running as service!");

        if let Err(_e) = run_service(arguments) {
            // Handle error in some way.
        }
    }

    fn run_service(_arguments: Vec<OsString>) -> windows_service::Result<()> {
        let event_handler = move |control_event| -> ServiceControlHandlerResult {
            info!("EVENT: {:?}", control_event);
            match control_event {
                ServiceControl::Stop => {
                    crate::stop();
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        };

        // Register system service event handler
        let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

        let next_status = ServiceStatus {
            // Should match the one from system service registry
            service_type: ServiceType::OwnProcess,
            // The new state
            current_state: ServiceState::Running,
            // Accept stop events when running
            controls_accepted: ServiceControlAccept::STOP,
            // Used to report an error when starting or stopping only, otherwise must be zero
            exit_code: ServiceExitCode::Win32(0),
            // Only used for pending states, otherwise must be zero
            checkpoint: 0,
            // Only used for pending states, otherwise must be zero
            wait_hint: Duration::default(),
        };

        // Tell the system that the service is running now
        status_handle.set_service_status(next_status)?;

        crate::start();

        // Tell the system that service has stopped.
        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OwnProcess,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
        })?;

        Ok(())
    }
}
