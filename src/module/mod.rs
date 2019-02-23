/* Copyright (C) 2017 by Jacob Alexander
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

//use std::thread;

pub mod unicode;

/// DeviceInfo struct
/*
pub struct DeviceInfo {
    serial  : &'static str, // Device serial number
    address : &'static str, // Device hardware address
    name    : &'static str, // Device name
}
*/

use crate::RUNNING;
use crate::protocol::hidio::*;
use crate::device::hidusb::*;
use crate::module::unicode::x11::*;
use crate::module::unicode::UnicodeOutput;

use std::thread;
use std::io::Write;
use std::time::Duration;
use std::sync::mpsc::channel;
use std::sync::atomic::Ordering;

const PROCESS_DELAY: u64 = 1;

// Move me
const SUPPORTED_IDS: &[HIDIOCommandID] = &[
    HIDIOCommandID::SupportedIDs,
    HIDIOCommandID::GetProperties,
    HIDIOCommandID::UnicodeText,
    HIDIOCommandID::UnicodeKey,
    HIDIOCommandID::OpenURL,
    HIDIOCommandID::Terminal,
];

fn as_u8_slice(v: &[u16]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            v.as_ptr() as *const u8,
            v.len() * std::mem::size_of::<u16>(),
        )
    }
}

struct HIDIOHandler {
    mailbox: HIDIOMailbox,
    xorg: XConnection, 
    //xorg_tx: std::sync::mpsc::Sender<(usize, String)>,
    //vt_rx: std::sync::mpsc::Receiver<u8>,
}

impl HIDIOHandler {
    fn new(mailbox: HIDIOMailbox) -> HIDIOHandler {
        use std::thread;
	let connection = XConnection::new();
        /*let (xorg_tx, xorg_rx) = channel::<(usize, String)>();
	thread::Builder::new().name("Xorg".to_string()).spawn(move|| {
            let mut connection = XConnection::new();
            loop {
		if !running.load(Ordering::SeqCst) { break; }
                match xorg_rx.recv_timeout(Duration::from_millis(1)) {
		    Ok((t, s)) => {
			match t {
			    0 => { connection.type_string(&s); },
			    1 => { connection.set_held(&s); },
			    _ => {}
			}
		    },
		    Err(RecvTimeoutError::Timeout) => {},
		    Err(RecvTimeoutError::Disconnected) => {
                        println!("Lost socket. Terminating thread");
		    }
		}
            }
	    println!("XORG THREAD DONE!!!!!!");
        }).unwrap();*/

        /*let (vt_tx, vt_rx) = channel::<u8>();
	thread::Builder::new().name("VT".to_string()).spawn(move|| {
            use std::io::Read;
            loop {
                for byte in std::io::stdin().lock().bytes() {
                    if let Ok(b) = byte {
                        vt_tx.send(b).unwrap();
                    } else {
                            warn!("Lost stdin");
                            break;
                    }
                }
            }
        }).unwrap();*/

        HIDIOHandler {
            mailbox,
            //xorg_tx,
            xorg: connection,
            //vt_rx,
        }
    }

    /// hidusb device processing
    fn process(&mut self) { 
        /*let (packet_tx, packet_rx) = channel::<HIDIOPacketBuffer>();
        let (response_tx, response_rx) = channel::<HIDIOPacketBuffer>();
        device.send_sync();*/

        //let mut device = HIDIOQueue::new(packet_rx, response_tx);

        let mailbox = &self.mailbox;
        //let mut received = device.create_buffer();

        //for message in mailbox.iter() {
        loop {
	    if !RUNNING.load(Ordering::SeqCst) { break; }
	    let message = mailbox.recv_psuedoblocking();
	    if !message.is_some() {
		continue;
	    }
	    let message = message.unwrap();

            /*match device.recv_chunk(&mut received) {
                Ok(recv) => {
                    if (recv > 0) {
                            last_sync = Instant::now();

                            let len = received.data.len();
                            //println!("[{}..{}]", prev_len, len);
                            info!("<{:?}>", &received.data[prev_len..len].iter().map(|x| *x as char).collect::<Vec<char>>());
                            prev_len = received.data.len();

                            match &received.ptype {
                                HIDIOPacketType::Sync => {
                                    received = device.create_buffer();
                                }
                                HIDIOPacketType::ACK => {
                                    // Don't ack an ack
                                },
                                HIDIOPacketType::NAK => {
                                    println!("NACK");
                                    break;
                                }
                                HIDIOPacketType::Continued
                                | HIDIOPacketType::Data => {
                                }
                            }

                            if !received.done {
                                device.send_ack(received.id, vec![]);
                            }
                    }
                },
                Err(e) => {
                    //::std::process::exit(1);
                    break;
                }
            };*/

            let (device, received) = (message.device, message.message);
            
            //if received.done {

                if received.ptype != HIDIOPacketType::Data {
			continue;
		}

		let id: HIDIOCommandID = unsafe { std::mem::transmute(received.id as u16) };
		let mydata = received.data.clone();
		//println!("Processing command: {:?}", id);
		match id {
		    HIDIOCommandID::SupportedIDs => {
			let ids = SUPPORTED_IDS.iter().map(|x| unsafe { std::mem::transmute(*x) }).collect::<Vec<u16>>();
			mailbox.send_ack(device, received.id, as_u8_slice(&ids).to_vec());
		    },
		    HIDIOCommandID::GetProperties => {
			use crate::built_info;
			let property: HIDIOPropertyID = unsafe { std::mem::transmute(mydata[0]) };
			println!("Get prop {:?}", property);
			match property {
			    HIDIOPropertyID::HIDIOMajor => {
				let v = built_info::PKG_VERSION_MAJOR.parse::<u16>().unwrap();
				mailbox.send_ack(device, received.id, as_u8_slice(&[v]).to_vec());
			    },
			    HIDIOPropertyID::HIDIOMinor => {
				let v = built_info::PKG_VERSION_MINOR.parse::<u16>().unwrap();
				mailbox.send_ack(device, received.id, as_u8_slice(&[v]).to_vec());
			    },
			    HIDIOPropertyID::HIDIOPatch => {
				let v = built_info::PKG_VERSION_PATCH.parse::<u16>().unwrap();
				mailbox.send_ack(device, received.id, as_u8_slice(&[v]).to_vec());
			    },
			    HIDIOPropertyID::HostOS => {
				let os = match built_info::CFG_OS {
				    "windows" => HostOSID::Windows,
				    "macos" => HostOSID::Mac,
				    "ios" => HostOSID::IOS,
				    "linux" => HostOSID::Linux,
				    "android" => HostOSID::Android,
				    "freebsd" | "openbsd" | "netbsd" => HostOSID::Linux,
				    _ => HostOSID::Unknown,
				};
				mailbox.send_ack(device, received.id, vec![os as u8]);
			    },
			    HIDIOPropertyID::OSVersion => {
				let version = "1.0"; // TODO: Retreive in cross platform way
				mailbox.send_ack(device, received.id, version.as_bytes().to_vec());
			    },
			    HIDIOPropertyID::HostName => {
				let name = built_info::PKG_NAME;
				mailbox.send_ack(device, received.id, name.as_bytes().to_vec());
			    },
			    HIDIOPropertyID::InputLayout => {
				let layout = XConnection::get_layout();
				println!("Current layout: {}", layout);
				mailbox.send_ack(device, received.id, layout.as_bytes().to_vec());
			    },
			    _ => {
				warn!("Unknown property: {:?}", &property);
				mailbox.send_nack(device, received.id, vec![mydata[0]]);
			    }
			};
		    }
		    HIDIOCommandID::UnicodeText => {
			let s = String::from_utf8(mydata).unwrap();
			//self.xorg_tx.send((0, s)).unwrap();
			self.xorg.type_string(&s);
			mailbox.send_ack(device, received.id, vec![]);
		    },
		    HIDIOCommandID::UnicodeKey => {
			let s = String::from_utf8(mydata).unwrap();
			//self.xorg_tx.send((1, s)).unwrap();
			self.xorg.set_held(&s);
			mailbox.send_ack(device, received.id, vec![]);
		    },
		    HIDIOCommandID::HostMacro => {
			warn!("TODO");
			mailbox.send_ack(device, received.id, vec![]);
		    },
		    HIDIOCommandID::KLLState => {
			warn!("TODO");
			mailbox.send_ack(device, received.id, vec![]);
		    },
		    HIDIOCommandID::OpenURL => {
			let s = String::from_utf8(mydata).unwrap();
			println!("Open url: {}", s);
			open::that(s).unwrap();
			mailbox.send_ack(device, received.id, vec![]);
		    },
		    HIDIOCommandID::Terminal => {
			mailbox.send_ack(device, received.id, vec![]);
			/*std::io::stdout().write_all(&mydata).unwrap();
			std::io::stdout().flush().unwrap();*/
		    },
		    HIDIOCommandID::InputLayout => {
			let s = String::from_utf8(mydata).unwrap();
			info!("Setting language to {}", s);
			match XConnection::set_layout(&s) {
			    Ok(_) => mailbox.send_ack(device, received.id, vec![]),
			    Err(_) => mailbox.send_nack(device, received.id, vec![]),
			}
		    },
		    _ => {
			warn!("Unknown command ID: {:?}", &received.id);
			mailbox.send_nack(device, received.id, vec![]);
		    }
		}
	    //}
	    //_ => {} // Handle elsewhere
	//}

                //received = device.create_buffer();
            //}

            /*let mut vt_buf = vec![];
            loop {
                match self.vt_rx.try_recv() {
                        Ok(c) => {
                            vt_buf.push(c);
                        },
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                                break;
                        }
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                println!("Lost socket");
                                ::std::process::exit(1);
                        },
                }
            }

            if !vt_buf.is_empty() {
                // FIXME
                // device.exec_call(HIDIOCommandID::Terminal, vt_buf);
            }*/

            thread::sleep(Duration::from_millis(PROCESS_DELAY));
        }
    }
}


/// Device initialization
/// Sets up a scanning thread per Device type.
/// Each scanning thread will create a new thread per device found.
/// The scanning thread is required in case devices are plugged/unplugged while running.
/// If a device is unplugged, the Device thread will exit.
pub fn initialize(mailbox: HIDIOMailbox) -> std::thread::JoinHandle<()> {
    info!("Initializing modules...");

    thread::Builder::new().name("Command Handler".to_string()).spawn(move|| {
	let mut handler = HIDIOHandler::new(mailbox);
        handler.process();
    }).unwrap()
}
