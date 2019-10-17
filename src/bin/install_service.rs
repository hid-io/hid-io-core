#[cfg(windows)]
extern crate windows_service;

#[cfg(windows)]
use hid_io_core::built_info;

#[cfg(windows)]
const SERVICE_NAME: &str = built_info::PKG_NAME;

#[cfg(windows)]
fn main() -> windows_service::Result<()> {
    use std::ffi::OsString;
    use windows_service::service::{
        ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType,
    };
    use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_binary_path = ::std::env::current_exe()
        .unwrap()
        .with_file_name("hid-io.exe");

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(format!("{} service", built_info::PKG_NAME)),
        service_type: ServiceType::OwnProcess,
        start_type: ServiceStartType::AutoStart, //ServiceStartType::OnDemand,
        error_control: ServiceErrorControl::Normal,
        executable_path: service_binary_path,
        launch_arguments: vec!["-d".into()],
        account_name: None, // run as System
        account_password: None,
    };
    let _service = service_manager.create_service(service_info, ServiceAccess::empty())?;
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    panic!("This program is only intended to run on Windows.");
}
