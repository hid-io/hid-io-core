/* Copyright (C) 2020-2022 by Jacob Alexander
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

extern crate tokio;

use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use clap::{arg, Arg, Command};
use futures::{AsyncReadExt, FutureExt};
use hid_io_core::built_info;
use hid_io_core::common_capnp::NodeType;
use hid_io_core::hidio_capnp;
use hid_io_core::hidio_capnp::hid_io_server;
use hid_io_core::keyboard_capnp;
use hid_io_core::logging::setup_logging_lite;
use rand::Rng;
use std::fs;
use std::io::Write;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio_rustls::{rustls::ClientConfig, TlsConnector};

const LISTEN_ADDR: &str = "localhost:7185";

mod danger {
    use std::time::SystemTime;
    use tokio_rustls::rustls::{Certificate, ServerName};

    pub struct NoCertificateVerification {}

    impl rustls::client::ServerCertVerifier for NoCertificateVerification {
        fn verify_server_cert(
            &self,
            _end_entity: &Certificate,
            _intermediates: &[Certificate],
            _server_name: &ServerName,
            _scts: &mut dyn Iterator<Item = &[u8]>,
            _ocsp_response: &[u8],
            _now: SystemTime,
        ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
            Ok(rustls::client::ServerCertVerified::assertion())
        }
    }
}

fn format_node(node: hid_io_core::common_capnp::destination::Reader<'_>) -> String {
    format!(
        "{}: {} ({})",
        node.get_type().unwrap(),
        node.get_name().unwrap_or(""),
        node.get_serial().unwrap_or(""),
    )
}

struct KeyboardSubscriberImpl;

impl keyboard_capnp::keyboard::subscriber::Server for KeyboardSubscriberImpl {
    fn update(
        &mut self,
        params: keyboard_capnp::keyboard::subscriber::UpdateParams,
        _results: keyboard_capnp::keyboard::subscriber::UpdateResults,
    ) -> Promise<(), ::capnp::Error> {
        let signal = pry!(pry!(params.get()).get_signal());

        // Only read cli messages
        if let Ok(hid_io_core::keyboard_capnp::keyboard::signal::data::Which::Cli(cli)) =
            signal.get_data().which()
        {
            let cli = cli.unwrap();
            print!("{}", cli.get_output().unwrap());
            std::io::stdout().flush().unwrap();
        }

        Promise::ok(())
    }
}

#[tokio::main]
pub async fn main() -> Result<(), ::capnp::Error> {
    setup_logging_lite().ok();
    tokio::task::LocalSet::new().run_until(try_main()).await
}

async fn try_main() -> Result<(), ::capnp::Error> {
    let version_info = format!(
        "{}{} - {}",
        built_info::PKG_VERSION,
        built_info::GIT_VERSION.map_or_else(|| "".to_owned(), |v| format!(" (git {})", v)),
        built_info::PROFILE,
    );
    let after_info = format!(
        "{} ({}) -> {} ({})",
        built_info::RUSTC_VERSION,
        built_info::HOST,
        built_info::TARGET,
        built_info::BUILT_TIME_UTC,
    );

    // Parse arguments
    let matches = Command::new("hid-io-core tool")
        .version(version_info.as_str())
        .author(built_info::PKG_AUTHORS)
        .about(format!("\n{}", built_info::PKG_DESCRIPTION).as_str())
        .after_help(after_info.as_str())
        .arg(
            Arg::new("serial")
                .short('s')
                .long("serial")
                .value_name("SERIAL")
                .help("Serial number of device (may include spaces, remember to quote).")
                .takes_value(true),
        )
        .arg(
            Arg::new("list")
                .short('l')
                .long("list")
                .help("Lists currently connected hid-io enabled devices."),
        )
        .subcommand(Command::new("flash").about("Attempt to enable flash mode on device"))
        .subcommand(Command::new("ids").about("List supported ids by device"))
        .subcommand(Command::new("info").about("Query information on device"))
        .subcommand(
            Command::new("manufacturing")
                .about("Send manufacturing commands to the device")
                .arg(
                    Arg::new("cmd")
                        .short('c')
                        .long("cmd")
                        .takes_value(true)
                        .required(true)
                        .help("Manufacturing command id (16-int integer)"),
                )
                .arg(
                    Arg::new("arg")
                        .short('a')
                        .long("arg")
                        .takes_value(true)
                        .required(true)
                        .help("Manufacturing command arg (16-int integer)"),
                ),
        )
        .subcommand(
            Command::new("pixel")
                .about("Send pixel/led commands to the device")
                .subcommand(
                    Command::new("setting")
                    .about("Pixel/led process control settings")
                    .subcommand(
                        Command::new("control")
                            .about("Pixel/led process settings")
                            .subcommand(
                                Command::new("disable")
                                .about("Disable HID-IO control of LEDs")
                            )
                            .subcommand(
                                Command::new("enable-start")
                                .about("Enable HID-IO control of LEDs in free-running mode")
                            )
                            .subcommand(
                                Command::new("enable-pause")
                                .about("Enable HID-IO control of LEDs in pause mode (see frame next-frame).")
                            )
                            .arg_required_else_help(true)
                    )
                    .subcommand(
                        Command::new("reset")
                            .about("LED controller reset")
                            .subcommand(
                                Command::new("soft-reset")
                                .about("LED controller soft reset")
                            )
                            .subcommand(
                                Command::new("hard-reset")
                                .about("LED controller hard (chip) reset")
                            )
                            .arg_required_else_help(true)
                    )
                    .subcommand(
                        Command::new("clear")
                            .about("Clear current buffer if under HID-IO control")
                    )
                    .subcommand(
                        Command::new("frame")
                            .about("Frame control")
                            .subcommand(
                                Command::new("next-frame")
                                    .about("Iterate to next display frame")
                            )
                            .arg_required_else_help(true)
                    )
                    .arg_required_else_help(true)
                )
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("direct")
                    .about("Directly manipulate led buffer, device/configuration dependent.")
                    .arg_required_else_help(true)
                    .arg(arg!(<START_ADDRESS> "16-bit starting address for data").value_parser(clap::value_parser!(u64).range(0..0xFFFF)))
                    .arg(arg!(<DATA> ... "Channel data as 8 bit data (hex or int)").value_parser(clap::value_parser!(u64).range(0..0xFF)))
                )
        )
        .subcommand(Command::new("sleep").about("Attempt to enable sleep mode on device"))
        .subcommand(
            Command::new("test")
                .about("Send arbitrary data to the device to ack back")
                .arg(
                    Arg::new("data")
                        .short('d')
                        .long("data")
                        .takes_value(true)
                        .required(true)
                        .help("Taken as a string, used as a byte array"),
                ),
        )
        .get_matches();

    let addr = LISTEN_ADDR
        .to_socket_addrs()?
        .next()
        .expect("could not parse address");
    println!("Connecting to {}", addr);

    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(Arc::new(danger::NoCertificateVerification {}))
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));

    let domain = rustls::ServerName::try_from("localhost").unwrap();

    loop {
        let stream = match tokio::net::TcpStream::connect(&addr).await {
            Ok(stream) => stream,
            Err(e) => {
                println!("Failed to connect ({}): {}", addr, e);
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                continue;
            }
        };
        stream.set_nodelay(true)?;
        let stream = connector.connect(domain.clone(), stream).await?;

        let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();

        let network = Box::new(twoparty::VatNetwork::new(
            reader,
            writer,
            rpc_twoparty_capnp::Side::Client,
            Default::default(),
        ));
        let mut rpc_system = RpcSystem::new(network, None);
        let hidio_server: hid_io_server::Client =
            rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

        let _rpc_disconnector = rpc_system.get_disconnector();
        tokio::task::spawn_local(Box::pin(rpc_system.map(|_| ())));

        // Display server version information
        let request = hidio_server.version_request();
        let response = request.send().promise.await?;
        let value = response.get().unwrap().get_version().unwrap();
        println!("Version:    {}", value.get_version().unwrap());
        println!("Buildtime:  {}", value.get_buildtime().unwrap());
        println!("Serverarch: {}", value.get_serverarch().unwrap());
        println!("Compiler:   {}", value.get_compilerversion().unwrap());

        // Lookup key location
        let auth_key_file = {
            let request = hidio_server.key_request();
            let response = request.send().promise.await?;
            let value = response.get().unwrap().get_key().unwrap();
            value.get_auth_key_path().unwrap().to_string()
        };
        println!("Key Path:   {}", auth_key_file);

        // Lookup key
        let auth_key = fs::read_to_string(auth_key_file)?;
        println!("Key:        {}", auth_key);

        // Lookup uid
        let uid = {
            let request = hidio_server.id_request();
            let response = request.send().promise.await?;
            let value = response.get().unwrap().get_id();
            value
        };
        println!("Id:         {}", uid);

        // Make authentication request
        let hidio = {
            let mut request = hidio_server.auth_request();
            let mut info = request.get().get_info()?;
            let mut rng = rand::thread_rng();
            info.set_type(NodeType::HidioApi);
            info.set_name("Device Tool");
            info.set_serial(&format!(
                "{:x} - pid:{}",
                rng.gen::<u64>(),
                std::process::id()
            ));
            info.set_id(uid);
            request.get().set_key(&auth_key);
            request.send().pipeline.get_port()
        };

        let nodes_resp = {
            let request = hidio.nodes_request();
            request.send().promise.await.unwrap()
        };
        let nodes = nodes_resp.get()?.get_nodes()?;

        // List device nodes
        if matches.is_present("list") {
            let devices: Vec<_> = nodes
                .iter()
                .filter(|n| {
                    n.get_type().unwrap() == NodeType::UsbKeyboard
                        || n.get_type().unwrap() == NodeType::BleKeyboard
                })
                .collect();
            println!(" * <uid> - <NodeType>: [<VID>:<PID>-<Usage Page>:<Usage>] [<Vendor>] <Name> (<Serial>)");
            for n in devices {
                println!(" * {} - {}", n.get_id(), format_node(n));
            }
            return Ok(());
        }

        // Serial is used to specify the device (if necessary)
        let mut serial = "".to_string();

        let nid = match matches.value_of("serial") {
            Some(n) => {
                serial = n.to_string();

                let serial_matched: Vec<_> = nodes
                    .iter()
                    .filter(|n| n.get_serial().unwrap() == serial)
                    .collect();

                if serial_matched.len() == 1 {
                    let n = serial_matched[0];
                    println!("Registering to {}", format_node(n));
                    n.get_id()
                } else {
                    eprintln!("Could not find: {}", serial);
                    std::process::exit(1);
                }
            }
            None => {
                let id;

                let serial_matched: Vec<_> = nodes
                    .iter()
                    .filter(|n| n.get_serial().unwrap() == serial)
                    .collect();
                // First attempt to match serial number
                if !serial.is_empty() && serial_matched.len() == 1 {
                    let n = serial_matched[0];
                    println!("Registering to {}", format_node(n));
                    id = n.get_id();
                } else {
                    let keyboards: Vec<_> = nodes
                        .iter()
                        .filter(|n| {
                            n.get_type().unwrap() == NodeType::UsbKeyboard
                                || n.get_type().unwrap() == NodeType::BleKeyboard
                        })
                        .collect();

                    // Next, if serial number is unset and there is only one keyboard, automatically attach
                    if serial.is_empty() && keyboards.len() == 1 {
                        let n = keyboards[0];
                        println!("Registering to {}", format_node(n));
                        id = n.get_id();
                    // Otherwise display a list of keyboard nodes
                    } else {
                        println!();
                        for n in keyboards {
                            println!(" * {} - {}", n.get_id(), format_node(n));
                        }

                        print!("Please choose a device: ");
                        std::io::stdout().flush()?;

                        let mut n = String::new();
                        std::io::stdin().read_line(&mut n)?;
                        id = n.trim().parse().unwrap();
                    }
                }
                id
            }
        };

        let device = nodes.iter().find(|n| n.get_id() == nid);
        if device.is_none() {
            eprintln!("Could not find node: {}", nid);
            std::process::exit(1);
        }
        let device = device.unwrap();
        //serial = format!("{}", device.get_serial().unwrap());

        match matches.subcommand() {
            Some(("flash", _)) => {
                // Flash mode command
                if let Ok(hid_io_core::common_capnp::destination::node::Which::Keyboard(node)) =
                    device.get_node().which()
                {
                    let node = node?;

                    let flash_mode_resp = {
                        // Cast/transform keyboard node to a hidio node
                        let request = hidio_capnp::node::Client {
                            client: node.client,
                        }
                        .flash_mode_request();
                        match request.send().promise.await {
                            Ok(response) => response,
                            Err(e) => {
                                eprintln!("Flash Mode request failed: {}", e);
                                ::std::process::exit(1);
                            }
                        }
                    };
                    // TODO Fully implement flash mode sequence
                    if flash_mode_resp
                        .get()
                        .unwrap()
                        .get_status()
                        .unwrap()
                        .has_success()
                    {
                        println!("Flash mode set");
                    }
                    // TODO Implement errors
                }
            }
            Some(("ids", _)) => {
                if let Ok(hid_io_core::common_capnp::destination::node::Which::Keyboard(node)) =
                    device.get_node().which()
                {
                    let node = node?;

                    let id_resp = {
                        // Cast/transform keyboard node to a hidio node
                        let request = hidio_capnp::node::Client {
                            client: node.client,
                        }
                        .supported_ids_request();
                        match request.send().promise.await {
                            Ok(response) => response,
                            Err(e) => {
                                eprintln!("Info request failed: {}", e);
                                ::std::process::exit(1);
                            }
                        }
                    };

                    let ids = id_resp.get().unwrap().get_ids().unwrap();
                    for id in ids {
                        println!("{}: {}", id.get_uid(), id.get_name().unwrap());
                    }
                }
            }
            Some(("info", _)) => {
                // Flash mode command
                if let Ok(hid_io_core::common_capnp::destination::node::Which::Keyboard(node)) =
                    device.get_node().which()
                {
                    let node = node?;

                    let info_resp = {
                        // Cast/transform keyboard node to a hidio node
                        let request = hidio_capnp::node::Client {
                            client: node.client,
                        }
                        .info_request();
                        match request.send().promise.await {
                            Ok(response) => response,
                            Err(e) => {
                                eprintln!("Info request failed: {}", e);
                                ::std::process::exit(1);
                            }
                        }
                    };
                    // TODO Fully implement info response
                    let info = info_resp.get().unwrap().get_info().unwrap();
                    println!(
                        "Version:          {}.{}.{}",
                        info.get_hidio_major_version(),
                        info.get_hidio_minor_version(),
                        info.get_hidio_patch_version()
                    );
                    println!("Device Name:      {}", info.get_device_name().unwrap());
                    println!("Device Vendor:    {}", info.get_device_vendor().unwrap());
                    println!("Device Serial:    {}", info.get_device_serial().unwrap());
                    println!("Device Version:   {}", info.get_device_version().unwrap());
                    println!("Device MCU:       {}", info.get_device_mcu().unwrap());
                    println!("Firmware Name:    {}", info.get_firmware_name().unwrap());
                    println!("Firmware Version: {}", info.get_firmware_version().unwrap());
                }
            }
            Some(("manufacturing", submatches)) => {
                // Flash mode command
                if let Ok(hid_io_core::common_capnp::destination::node::Which::Keyboard(node)) =
                    device.get_node().which()
                {
                    let node = node?;

                    // Retrieve arguments
                    let cmd: u16 = submatches.value_of("cmd").unwrap().parse().unwrap();
                    let arg: u16 = submatches.value_of("arg").unwrap().parse().unwrap();

                    let manufacturing_test_resp = {
                        // Cast/transform keyboard node to a hidio node
                        let mut request = hidio_capnp::node::Client {
                            client: node.client,
                        }
                        .manufacturing_test_request();

                        let command = match cmd {
                            1 => {
                                request.get().get_command().unwrap().set_led_test_sequence(match arg {
                                    0 => hidio_capnp::node::manufacturing::LedTestSequenceArg::Disable,
                                    1 => hidio_capnp::node::manufacturing::LedTestSequenceArg::Enable,
                                    2 => hidio_capnp::node::manufacturing::LedTestSequenceArg::ActivateLedShortTest,
                                    3 => hidio_capnp::node::manufacturing::LedTestSequenceArg::ActivateLedOpenCircuitTest,
                                    _ => {
                                        eprintln!("Manufacturing Test unknown arg: {}", cmd);
                                        ::std::process::exit(1);
                                    }
                                });
                                hidio_capnp::node::manufacturing::Command::LedTestSequence
                            }
                            2 => {
                                request.get().get_command().unwrap().set_led_cycle_keypress_test(match arg {
                                    0 => hidio_capnp::node::manufacturing::LedCycleKeypressTestArg::Disable,
                                    1 => hidio_capnp::node::manufacturing::LedCycleKeypressTestArg::Enable,
                                    _ => {
                                        eprintln!("Manufacturing Test unknown arg: {}", cmd);
                                        ::std::process::exit(1);
                                    }
                                });
                                hidio_capnp::node::manufacturing::Command::LedCycleKeypressTest
                            }
                            3 => {
                                request.get().get_command().unwrap().set_hall_effect_sensor_test(match arg {
                                    0 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::DisableAll,
                                    1 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::PassFailTestToggle,
                                    2 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckToggle,
                                    _ => {
                                        eprintln!("Manufacturing Test unknown arg: {}", cmd);
                                        ::std::process::exit(1);
                                    }
                                });
                                hidio_capnp::node::manufacturing::Command::HallEffectSensorTest
                            }
                            _ => {
                                eprintln!("Manufacturing Test unknown cmd: {}", cmd);
                                ::std::process::exit(1);
                            }
                        };
                        println!("Command: {:?}", command);
                        request.get().get_command().unwrap().set_command(command);

                        // Send command
                        match request.send().promise.await {
                            Ok(response) => response,
                            Err(e) => {
                                eprintln!("Manufacturing Test request failed: {}", e);
                                ::std::process::exit(1);
                            }
                        }
                    };
                    if manufacturing_test_resp
                        .get()
                        .unwrap()
                        .get_status()
                        .unwrap()
                        .has_success()
                    {
                        println!("Manufacturing Test set: {}:{}", cmd, arg);
                    } else {
                        println!("NAK: Manufacturing Test set: {}:{} - FAILED", cmd, arg);
                    }
                }
            }
            Some(("pixel", submatches)) => {
                match submatches.subcommand() {
                    Some(("setting", submatches)) => {
                        if let Ok(hid_io_core::common_capnp::destination::node::Which::Keyboard(
                            node,
                        )) = device.get_node().which()
                        {
                            let node = node?;

                            let setting_resp = {
                                // Cast/transform keyboard node to a hidio node
                                let mut request = hidio_capnp::node::Client {
                                    client: node.client,
                                }
                                .pixel_setting_request();

                                let command = match submatches.subcommand() {
                                    Some(("clear", _)) => {
                                        request.get().get_command().unwrap().set_clear(
                                            hidio_capnp::node::pixel_setting::ClearArg::Clear,
                                        );
                                        hidio_capnp::node::pixel_setting::Command::Clear
                                    }
                                    Some(("control", submatches)) => {
                                        let arg = match submatches.subcommand() {
                                            Some(("disable", _)) => {
                                                hidio_capnp::node::pixel_setting::ControlArg::Disable
                                            }
                                            Some(("enable-pause", _)) => {
                                                hidio_capnp::node::pixel_setting::ControlArg::EnablePause
                                            }
                                            Some(("enable-start", _)) => {
                                                hidio_capnp::node::pixel_setting::ControlArg::EnableStart
                                            }
                                            _ => todo!(),
                                        };
                                        request.get().get_command().unwrap().set_control(arg);
                                        hidio_capnp::node::pixel_setting::Command::Control
                                    }
                                    Some(("frame", submatches)) => {
                                        let arg = match submatches.subcommand() {
                                            Some(("next-frame", _)) => {
                                                hidio_capnp::node::pixel_setting::FrameArg::NextFrame
                                            }
                                            _ => todo!(),
                                        };
                                        request.get().get_command().unwrap().set_frame(arg);
                                        hidio_capnp::node::pixel_setting::Command::Frame
                                    }
                                    Some(("reset", submatches)) => {
                                        let arg = match submatches.subcommand() {
                                            Some(("hard-reset", _)) => {
                                                hidio_capnp::node::pixel_setting::ResetArg::HardReset
                                            }
                                            Some(("soft-reset", _)) => {
                                                hidio_capnp::node::pixel_setting::ResetArg::SoftReset
                                            }
                                            _ => todo!(),
                                        };
                                        request.get().get_command().unwrap().set_reset(arg);
                                        hidio_capnp::node::pixel_setting::Command::Reset
                                    }
                                    _ => todo!(),
                                };
                                request.get().get_command().unwrap().set_command(command);

                                match request.send().promise.await {
                                    Ok(response) => response,
                                    Err(e) => {
                                        eprintln!("Setting command request failed: {}", e);
                                        ::std::process::exit(1);
                                    }
                                }
                            };
                            if setting_resp
                                .get()
                                .unwrap()
                                .get_status()
                                .unwrap()
                                .has_success()
                            {
                                println!("Setting command successful");
                            }
                        }
                    }
                    Some(("direct", submatches)) => {
                        if let Ok(hid_io_core::common_capnp::destination::node::Which::Keyboard(
                            node,
                        )) = device.get_node().which()
                        {
                            let node = node?;

                            // Retrieve arguments
                            let start_address: u16 = u16::try_from(
                                *submatches
                                    .get_one::<u64>("START_ADDRESS")
                                    .expect("Required"),
                            )
                            .unwrap();
                            let data: Vec<u8> = submatches
                                .get_many::<u64>("DATA")
                                .into_iter()
                                .flatten()
                                .map(|val| u8::try_from(*val).unwrap())
                                .collect::<Vec<_>>();

                            let direct_resp = {
                                // Cast/transform keyboard node to a hidio node
                                let mut request = hidio_capnp::node::Client {
                                    client: node.client,
                                }
                                .pixel_set_request();

                                request
                                    .get()
                                    .get_command()
                                    .unwrap()
                                    .set_type(hidio_capnp::node::pixel_set::Type::DirectSet);
                                request
                                    .get()
                                    .get_command()
                                    .unwrap()
                                    .set_start_address(start_address);
                                request
                                    .get()
                                    .get_command()
                                    .unwrap()
                                    .set_direct_set_data(&data);

                                // Send command
                                match request.send().promise.await {
                                    Ok(response) => response,
                                    Err(e) => {
                                        eprintln!("Pixel Set request failed: {}", e);
                                        ::std::process::exit(1);
                                    }
                                }
                            };
                            if direct_resp
                                .get()
                                .unwrap()
                                .get_status()
                                .unwrap()
                                .has_success()
                            {
                                println!("Pixel Set: {}:{:?}", start_address, data);
                            } else {
                                println!("NAK: Pixel Set: {}:{:?} - FAILED", start_address, data);
                            }
                        }
                    }
                    _ => todo!(),
                }
            }
            Some(("sleep", _)) => {
                // Sleep mode command
                if let Ok(hid_io_core::common_capnp::destination::node::Which::Keyboard(node)) =
                    device.get_node().which()
                {
                    let node = node?;

                    let sleep_mode_resp = {
                        // Cast/transform keyboard node to a hidio node
                        let request = hidio_capnp::node::Client {
                            client: node.client,
                        }
                        .sleep_mode_request();
                        match request.send().promise.await {
                            Ok(response) => response,
                            Err(e) => {
                                eprintln!("Sleep Mode request failed: {}", e);
                                ::std::process::exit(1);
                            }
                        }
                    };
                    // TODO Fully implement flash mode sequence
                    if sleep_mode_resp
                        .get()
                        .unwrap()
                        .get_status()
                        .unwrap()
                        .has_success()
                    {
                        println!("Sleep mode set");
                    }
                }
            }
            Some(("test", submatches)) => {
                if let Ok(hid_io_core::common_capnp::destination::node::Which::Keyboard(node)) =
                    device.get_node().which()
                {
                    let node = node?;

                    let data_cmd = submatches.value_of("data").unwrap().as_bytes();

                    let test_resp = {
                        // Cast/transform keyboard node to a hidio node
                        let mut request = hidio_capnp::node::Client {
                            client: node.client,
                        }
                        .test_request();
                        request.get().set_data(data_cmd);
                        match request.send().promise.await {
                            Ok(response) => response,
                            Err(e) => {
                                eprintln!("Info request failed: {}", e);
                                ::std::process::exit(1);
                            }
                        }
                    };

                    let data_ack = test_resp.get().unwrap().get_data().unwrap();

                    println!("Sent: {:?}", data_cmd);
                    println!("Recv: {:?}", data_ack);
                    println!("Sent (str): '{}'", String::from_utf8_lossy(data_cmd));
                    println!("Recv (str): '{}'", String::from_utf8_lossy(data_ack));
                    assert_eq!(data_cmd, data_ack, "Sent does not equal received!");

                    // Wait for any Manufacturing Test Data packets
                    // TODO - Only wait if argument is set
                    // - Build subscription for Manufacturing Test Data packets
                    // - Wait for Manufacturing Test Data packets
                }
            }
            _ => {
                println!("No command specified");
            }
        }

        return Ok(());
    }
    /*
    _rpc_disconnector.await?;
    Ok(())
    */
}
