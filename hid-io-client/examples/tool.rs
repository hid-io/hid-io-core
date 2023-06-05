/* Copyright (C) 2020-2023 by Jacob Alexander
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 * THE SOFTWARE.
 */

extern crate tokio;

use clap::{arg, Arg, Command};
use hid_io_core::built_info;
use hid_io_core::common_capnp::NodeType;
use hid_io_core::hidio_capnp;
use hid_io_core::logging::setup_logging_lite;
use rand::Rng;
use std::io::Write;

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
                    .arg(arg!(<START_ADDRESS> "16-bit starting address for data").value_parser(clap::value_parser!(u64).range(0..=0xFFFF)))
                    .arg(arg!(<DATA> ... "Channel data as 8 bit data (hex or int)").value_parser(clap::value_parser!(u64).range(0..=0xFF)))
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

    // Prepare hid-io-core connection
    let mut hidio_conn = hid_io_client::HidioConnection::new().unwrap();
    let mut rng = rand::thread_rng();
    // Connect and authenticate with hid-io-core
    let (hidio_auth, _hidio_server) = hidio_conn
        .connect(
            hid_io_client::AuthType::Priviledged,
            NodeType::HidioApi,
            "Device tool".to_string(),
            format!("{:x} - pid:{}", rng.gen::<u64>(), std::process::id()),
            true,
            std::time::Duration::from_millis(1000),
        )
        .await?;
    let hidio_auth = hidio_auth.expect("Could not authenticate to hid-io-core");

    let nodes_resp = {
        let request = hidio_auth.nodes_request();
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
            println!(" * {} - {}", n.get_id(), hid_io_client::format_node(n));
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
                println!("Registering to {}", hid_io_client::format_node(n));
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
                println!("Registering to {}", hid_io_client::format_node(n));
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
                    println!("Registering to {}", hid_io_client::format_node(n));
                    id = n.get_id();
                // Otherwise display a list of keyboard nodes
                } else {
                    println!();
                    for n in keyboards {
                        println!(" * {} - {}", n.get_id(), hid_io_client::format_node(n));
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
                                    0x11 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn1Toggle,
                                    0x12 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn2Toggle,
                                    0x13 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn3Toggle,
                                    0x14 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn4Toggle,
                                    0x15 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn5Toggle,
                                    0x16 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn6Toggle,
                                    0x17 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn7Toggle,
                                    0x18 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn8Toggle,
                                    0x19 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn9Toggle,
                                    0x1A => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn10Toggle,
                                    0x1B => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn11Toggle,
                                    0x1C => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn12Toggle,
                                    0x1D => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn13Toggle,
                                    0x1E => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn14Toggle,
                                    0x1F => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn15Toggle,
                                    0x20 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn16Toggle,
                                    0x21 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn17Toggle,
                                    0x22 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn18Toggle,
                                    0x23 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn19Toggle,
                                    0x24 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn20Toggle,
                                    0x25 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn21Toggle,
                                    0x26 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckColumn22Toggle,
                                    0x100 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::ModeSetNormal,
                                    0x101 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::ModeSetLowLatency,
                                    0x102 => hidio_capnp::node::manufacturing::HallEffectSensorTestArg::ModeSetTest,
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
                    if let Ok(hid_io_core::common_capnp::destination::node::Which::Keyboard(node)) =
                        device.get_node().which()
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
                    if let Ok(hid_io_core::common_capnp::destination::node::Which::Keyboard(node)) =
                        device.get_node().which()
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

    Ok(())
}
