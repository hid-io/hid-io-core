/* Copyright (C) 2019-2023 by Jacob Alexander
 * Copyright (C) 2019 by Rowan Decker
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

use hid_io_client::capnp;
use hid_io_client::capnp::capability::Promise;
use hid_io_client::capnp_rpc;
use hid_io_client::capnp_rpc::pry;
use hid_io_client::common_capnp::NodeType;
use hid_io_client::hidio_capnp;
use hid_io_client::keyboard_capnp;
use hid_io_client::setup_logging_lite;
use rand::Rng;
use std::io::Read;
use std::io::Write;

#[derive(Default)]
pub struct KeyboardSubscriberImpl {
    /// Switch matrix (raw, calibration offset)
    pub hall_effect_switch_data: Vec<Vec<(u16, i16)>>,
    pub hall_effect_switch_data_cur_strobe: u8,
}

impl keyboard_capnp::keyboard::subscriber::Server for KeyboardSubscriberImpl {
    fn update(
        &mut self,
        params: keyboard_capnp::keyboard::subscriber::UpdateParams,
        _results: keyboard_capnp::keyboard::subscriber::UpdateResults,
    ) -> Promise<(), ::capnp::Error> {
        let signal = pry!(pry!(params.get()).get_signal());

        // Only read cli messages
        if let Ok(signaltype) = signal.get_data().which() {
            match signaltype {
                hid_io_client::keyboard_capnp::keyboard::signal::data::Which::Cli(cli) => {
                    let cli = cli.unwrap();
                    print!("{}", cli.get_output().unwrap());
                    std::io::stdout().flush().unwrap();
                }
                hid_io_client::keyboard_capnp::keyboard::signal::data::Which::Manufacturing(
                    res,
                ) => {
                    let res = res.unwrap();
                    match res.get_cmd().unwrap() {
                        keyboard_capnp::keyboard::signal::manufacturing_result::Command::LedTestSequence => match res.get_arg() {
                            2 | 3 => {
                                println!("{:?}:{} => ", res.get_cmd(), res.get_arg());
                                // LED short/open test
                                let mut pos: usize = 0;
                                let data = res.get_data().unwrap();
                                loop {
                                    let chipid = data.get(pos as u32);
                                    pos += 1;
                                    let buffer_len = data.get(pos as u32);
                                    pos += 1;
                                    let buffer =
                                        &data.as_slice().unwrap()[pos..pos + buffer_len as usize];
                                    pos += buffer_len as usize;
                                    println!("ChipId: {} {}", chipid, buffer_len);
                                    for byte in buffer {
                                        print!("{:02x} ", byte);
                                    }
                                    println!();
                                    if pos >= data.len().try_into().unwrap() {
                                        break;
                                    }
                                }
                            }
                            _ => {
                                println!("Manufacturing command {} not implemented", res.get_arg())
                            }
                        },
                        keyboard_capnp::keyboard::signal::manufacturing_result::Command::HallEffectSensorTest => match res.get_arg() {
                            2 => {
                                let mut tmp = vec![];
                                let mut pos = 0;
                                for (i, byte) in res.get_data().unwrap().iter().enumerate() {
                                    if i == 0 || i == 1 {
                                        continue;
                                    }

                                    tmp.push(byte);
                                    if tmp.len() == 4 {
                                        // Check if header (strobe) or sense data
                                        if pos % 7 == 0 {
                                            let strobe = tmp[0];
                                            // Print on full strobe
                                            if strobe == 0 {
                                                println!("{:?}:{} => ", res.get_cmd(), res.get_arg());
                                                for (i, data) in self.hall_effect_switch_data.iter().enumerate() {
                                                print!("{:>4} ", i);
                                                // Print raw
                                                for elem in data {
                                                    print!("{:>4} ", elem.0);
                                                }
                                                // Print offset
                                                for elem in data {
                                                    print!("{:>4} ", elem.1);
                                                }
                                                println!();
                                                }
                                            }

                                            self.hall_effect_switch_data_cur_strobe = strobe;
                                            if self.hall_effect_switch_data.len() <= strobe as usize {
                                                self.hall_effect_switch_data
                                                    .resize(strobe as usize + 1, vec![]);
                                            }
                                            self.hall_effect_switch_data[strobe as usize] = vec![];
                                        } else {
                                            let data = u16::from_le_bytes([tmp[0], tmp[1]]);
                                            let offset = i16::from_le_bytes([tmp[2], tmp[3]]);
                                            self.hall_effect_switch_data[self.hall_effect_switch_data_cur_strobe as usize].push((data, offset));
                                        }
                                        tmp.clear();
                                        pos += 1;
                                    }
                                }
                            }
                            _ => {
                                println!("{:?}:{} => ", res.get_cmd(), res.get_arg());
                                for byte in res.get_data().unwrap() {
                                    print!("{} ", byte);
                                }
                                println!();
                            }
                        },
                        _ => {
                            println!("{:?}:{} => ", res.get_cmd(), res.get_arg());
                            for byte in res.get_data().unwrap() {
                                print!("{} ", byte);
                            }
                            println!();
                        }
                    }
                    std::io::stdout().flush().unwrap();
                }
                _ => {}
            }
        }

        Promise::ok(())
    }
}

#[tokio::main]
pub async fn main() -> Result<(), capnp::Error> {
    setup_logging_lite().ok();
    tokio::task::LocalSet::new().run_until(try_main()).await
}

async fn try_main() -> Result<(), capnp::Error> {
    // Prepare hid-io-core connection
    let mut hidio_conn = hid_io_client::HidioConnection::new().unwrap();
    let mut rng = rand::thread_rng();

    // Serial is used for automatic reconnection if hid-io goes away and comes back
    let mut serial = "".to_string();

    loop {
        // Connect and authenticate with hid-io-core
        let (hidio_auth, _hidio_server) = hidio_conn
            .connect(
                hid_io_client::AuthType::Priviledged,
                NodeType::HidioApi,
                "RPC Test".to_string(),
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

        let args: Vec<_> = std::env::args().collect();
        let nid = match args.get(1) {
            Some(n) => n.parse().unwrap(),
            None => {
                let id;

                let serial_matched: Vec<_> = nodes
                    .iter()
                    .filter(|n| n.get_serial().unwrap() == serial)
                    .collect();
                // First attempt to match serial number
                if !serial.is_empty() && serial_matched.len() == 1 {
                    let n = serial_matched[0];
                    println!("Re-registering to {}", hid_io_client::format_node(n));
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
        serial = device.get_serial().unwrap().to_string();

        // Build subscription callback
        let subscription = capnp_rpc::new_client(KeyboardSubscriberImpl::default());

        // Subscribe to cli messages
        let subscribe_req = {
            let node = match device.get_node().which().unwrap() {
                hid_io_client::common_capnp::destination::node::Which::Keyboard(n) => n.unwrap(),
                hid_io_client::common_capnp::destination::node::Which::Daemon(_) => {
                    std::process::exit(1);
                }
            };
            let mut request = node.subscribe_request();
            let mut params = request.get();
            params.set_subscriber(subscription);

            // Build list of options
            let mut options = params.init_options(1);
            let mut cli_option = options.reborrow().get(0);
            cli_option.set_type(keyboard_capnp::keyboard::SubscriptionOptionType::CliOutput);
            request
        };
        let _callback = subscribe_req.send().promise.await.unwrap();

        println!("READY");
        let (vt_tx, mut vt_rx) = tokio::sync::mpsc::channel::<u8>(100);
        std::thread::spawn(move || loop {
            #[allow(clippy::significant_drop_in_scrutinee)]
            for byte in std::io::stdin().lock().bytes() {
                if let Ok(b) = byte {
                    if let Err(e) = vt_tx.blocking_send(b) {
                        println!("Restarting stdin loop: {}", e);
                        return;
                    }
                } else {
                    println!("Lost stdin");
                    std::process::exit(2);
                }
            }
        });

        loop {
            let mut vt_buf = vec![];
            // Await the first byte
            match vt_rx.recv().await {
                Some(c) => {
                    vt_buf.push(c);
                }
                None => {
                    println!("Lost socket");
                    ::std::process::exit(1);
                }
            }
            // Loop over the rest of the buffer
            loop {
                match vt_rx.try_recv() {
                    Ok(c) => {
                        vt_buf.push(c);
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                        // Done, can begin sending cli message to device
                        break;
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        println!("Lost socket (buffer)");
                        ::std::process::exit(1);
                    }
                }
            }

            if let Ok(nodetype) = device.get_node().which() {
                match nodetype {
                    hid_io_client::common_capnp::destination::node::Which::Keyboard(node) => {
                        let node = node?;
                        let _command_resp = {
                            // Cast/transform keyboard node to a hidio node
                            let mut request = hidio_capnp::node::Client {
                                client: node.client,
                            }
                            .cli_command_request();
                            request.get().set_command(&String::from_utf8(vt_buf)?);
                            match request.send().promise.await {
                                Ok(response) => response,
                                Err(e) => {
                                    println!("Dead: {}", e);
                                    break;
                                }
                            }
                        };
                    }
                    hid_io_client::common_capnp::destination::node::Which::Daemon(_node) => {}
                }
            }
        }
    }
}
