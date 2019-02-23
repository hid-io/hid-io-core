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

/// http://www.signal11.us/oss/hidapi/hidapi/doxygen/html/group__API.html#ga135931e04d48078a9fb7aebf663676f9
/// http://www.signal11.us/oss/hidapi/hidapi/doxygen/html/structhid__device__info.html
/// https://docs.rs/hidapi/0.5.0/hidapi/
/// https://www.kernel.org/doc/html/latest/driver-api/usb/usb.html?highlight=usb_co#c.usb_control_msg

use hidapi;
use crate::protocol::hidio::*;

//use std::string;
use std::thread;
//use std::thread::sleep;
use std::sync::mpsc::channel;
use std::time::Instant;
use std::time::Duration;
use std::collections::HashMap;

// TODO (HaaTa) remove this constants when hidapi supports better matching
const DEV_VID: u16 = 0x308f;
const DEV_PID: u16 = 0x0011;
const INTERFACE_NUMBER: i32 = 6;

const USB_FULLSPEED_PACKET_SIZE: usize = 64;
const ENUMERATE_DELAY: u64 = 1000;
const POLL_DELAY: u64 = 1;

/// HIDUSBDevice Struct
///
/// Contains HIDUSB device thread information
/// Required to communicate with device thread

use std::io::{Read, Write};
trait HIDIOTransport: Read + Write { }

struct HIDUSBDevice {
    device_info: hidapi::HidDeviceInfo,
    device: hidapi::HidDevice,
}

impl HIDUSBDevice {
	fn new(device_info: hidapi::HidDeviceInfo, device: hidapi::HidDevice) -> HIDUSBDevice {
		device.set_blocking_mode(false).unwrap();
		HIDUSBDevice {
			device_info,
			device,
		}
	}
}

impl std::io::Read for HIDUSBDevice {
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            match self.device.read(buf) {
                Ok(len) => {
		    if len > 0 {
			trace!("Received {} bytes", len);
			trace!("{:x?}", &buf[0..len]);
		    }
		    Ok(len)
                },
                Err(e) => {
                    warn!("{:?}", e);
		    Err(std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e)))
                }
            }
	}
}
impl std::io::Write for HIDUSBDevice {
	fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            let buf = {
		if _buf[0] == 0x00 {
		    let mut new_buf = vec![0];
		    new_buf.extend(_buf);
		    new_buf
		} else {
		    _buf.to_vec() 
		}
	    };

            match self.device.write(&buf) {
                Ok(len) => {
                    trace!("Sent {} bytes", len);
		    trace!("{:x?}", &buf[0..len]);
		    Ok(len)
                },
                Err(e) => {
                    warn!("{:?}", e);
		    Err(std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e)))
                }
            }
	}

	fn flush(&mut self) -> std::io::Result<()> {
		Ok(())
	}
}

impl HIDIOTransport for HIDUSBDevice { }

struct HIDIOEndpoint {
    socket: Box<HIDIOTransport>,
    max_packet_len: u32,
}

impl HIDIOEndpoint {
	fn new(socket: Box<HIDIOTransport>, max_packet_len: u32) -> HIDIOEndpoint {
		HIDIOEndpoint { socket, max_packet_len }
	}

	fn recv_chunk(&mut self, buffer: &mut HIDIOPacketBuffer) -> Result<usize, std::io::Error> {
	    use std::io::Read;
	    let mut rbuf = [0; 1024]; //self.PACKET_SIZE];
	    match self.socket.read(&mut rbuf) {
		Ok(len) => {
		    if len > 0 {
			let slice = &rbuf[0..len];
			let ret = buffer.decode_packet(&mut slice.to_vec());
                        if let Err(e) = ret {
                            error!("recv_chunk({}) {:?}", len, e);
                            println!("received: {:?}", slice);
                            println!("current state: {:?}", buffer);
                            std::process::exit(2);
                        } else {
                            info!("R{} {:x?}", buffer.data.len(), buffer);
                        }
		    }

		    Ok(len)
		},
		Err(e) => {
		    Err(e)
		}
	    }
	}

	fn create_buffer(&self) -> HIDIOPacketBuffer {
	    let mut buffer = HIDIOPacketBuffer::new();
	    buffer.max_len = self.max_packet_len;
	    buffer
	}

	fn recv_packet(&mut self) -> HIDIOPacketBuffer {
	    let mut deserialized = self.create_buffer();

	    while !deserialized.done {
		if let Ok(len) = self.recv_chunk(&mut deserialized) {
                    if len > 0 {
                        match &deserialized.ptype {
                            HIDIOPacketType::Sync => {
                                deserialized = self.create_buffer();
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
                                self.send_ack(deserialized.id, vec![]);
                            }
                        }
                    }
                }
	    }

	    //info!("Received {:x?}", deserialized);
	    deserialized
	}

    fn send_packet(&mut self, mut packet: HIDIOPacketBuffer) -> Result<(), std::io::Error> {
        use std::io::Write;
        info!("Sending {:x?}", packet);
        let buf: Vec<u8> = packet.serialize_buffer().unwrap();
        for chunk in buf.chunks(self.max_packet_len as usize).collect::<Vec<&[u8]>>().iter() {
            self.socket.write(chunk)?;
        }
	Ok(())
    }

    fn send_sync(&mut self) {
        self.send_packet(HIDIOPacketBuffer {
                ptype:   HIDIOPacketType::Sync,
                id:      0,
                max_len: 64, //..Defaults
                data:    vec![],
                done:    true,
        }).unwrap();
    }

    fn send_ack(&mut self, _id: u32, data: Vec<u8>) {
        self.send_packet(HIDIOPacketBuffer {
                ptype:   HIDIOPacketType::ACK,
                id:      0, // id,
                max_len: 64, //..Defaults
                data,
                done:    true,
        }).unwrap();
    }
}

struct HIDIOController {
    device: HIDIOEndpoint,
    received: HIDIOPacketBuffer,
    last_sync: Instant,
    message_queue: std::sync::mpsc::Sender<HIDIOPacketBuffer>,
    response_queue: std::sync::mpsc::Receiver<HIDIOPacketBuffer>,
}

impl HIDIOController {
    fn new(device: HIDIOEndpoint, message_queue: std::sync::mpsc::Sender<HIDIOPacketBuffer>, response_queue: std::sync::mpsc::Receiver<HIDIOPacketBuffer>) -> HIDIOController {
        let received = device.create_buffer();
        let last_sync = Instant::now();
        //let mut prev_len = 0;
        HIDIOController { device, received, last_sync, message_queue, response_queue }
    }

    fn process(&mut self) -> Result<(), std::io::Error> {
        match self.device.recv_chunk(&mut self.received) {
            Ok(recv) => {
                if recv > 0 {
                        self.last_sync = Instant::now();

                        /*let len = received.data.len();
                        //println!("[{}..{}]", prev_len, len);
                        info!("<{:?}>", &received.data[prev_len..len].iter().map(|x| *x as char).collect::<Vec<char>>());
                        prev_len = received.data.len();*/

                        match &self.received.ptype {
                            HIDIOPacketType::Sync => {
                                self.received = self.device.create_buffer();
                            }
                            HIDIOPacketType::ACK => {
                                // Don't ack an ack
                            },
                            HIDIOPacketType::NAK => {
                                println!("NACK. Resetting buffer");
                                self.received = self.device.create_buffer();
                            }
                            HIDIOPacketType::Continued
                            | HIDIOPacketType::Data => {
                            }
                        }

                        if !self.received.done {
                            self.device.send_ack(self.received.id, vec![]);
                        }
                }
            },
            Err(e) => {
		return Err(e);
                //::std::process::exit(1);
            }
        };
        
        if self.received.done {
            self.message_queue.send(self.received.clone()).unwrap();
            self.received = self.device.create_buffer();
            //prev_len = 0;
        }

        if self.last_sync.elapsed().as_secs() >= 5 {
	    self.device.send_sync();
	    self.received = self.device.create_buffer();
            self.last_sync = Instant::now();
            return Ok(());
        }
        
        match self.response_queue.try_recv() {
                Ok(mut p) => {
                    p.max_len = self.device.max_packet_len;
                    self.device.send_packet(p.clone())?;

                    if p.ptype == HIDIOPacketType::Sync {
                        self.received = self.device.create_buffer();
                    }
                },
                Err(std::sync::mpsc::TryRecvError::Empty) => { }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
			return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, ""));
                        //::std::process::exit(1);
                },
        }

	return Ok(());
    }
}

use std::sync::mpsc;
pub struct HIDIOQueue {
    message_queue: std::sync::mpsc::Receiver<HIDIOPacketBuffer>,
    response_queue: std::sync::mpsc::Sender<HIDIOPacketBuffer>,
}

impl HIDIOQueue {
    fn new(message_queue: std::sync::mpsc::Receiver<HIDIOPacketBuffer>, response_queue: std::sync::mpsc::Sender<HIDIOPacketBuffer>) -> HIDIOQueue {
        HIDIOQueue { message_queue, response_queue }
    }

    fn send_packet(&self, packet: HIDIOPacketBuffer) -> Result<(), mpsc::SendError<HIDIOPacketBuffer>> {
        self.response_queue.send(packet)
    }

    fn recv_packet(&mut self) -> HIDIOPacketBuffer {
        self.message_queue.recv().unwrap()
    }

    fn messages(&mut self) -> mpsc::TryIter<HIDIOPacketBuffer> {
        // TODO: Detect error (other side disconnected)
        self.message_queue.try_iter()
    }

    /*fn send_sync(&mut self) {
        self.send_packet(HIDIOPacketBuffer {
                ptype:   HIDIOPacketType::Sync,
                id:      0,
                max_len: 64, //..Defaults
                data:    vec![],
                done:    true,
        });
    }

    fn send_ack(&mut self, id: u32, data: Vec<u8>) {
        self.send_packet(HIDIOPacketBuffer {
                ptype:   HIDIOPacketType::ACK,
                id:      0, // id,
                max_len: 64, //..Defaults
                data:    data, done:    true,
        });
    }

    fn send_nack(&mut self, id: u32, data: Vec<u8>) {
        self.send_packet(HIDIOPacketBuffer {
                ptype:   HIDIOPacketType::NAK,
                id:      id,
                max_len: 64, //..Defaults
                data:    data,
                done:    true,
        });
    }

    fn send_command(&mut self, id: HIDIOCommandID, data: Vec<u8>) {
        self.send_packet(HIDIOPacketBuffer {
                ptype:   HIDIOPacketType::Data,
                id:      id as u32,
                max_len: 64, //..Defaults
                data:    data,
                done:    true,
        });
    }*/

    /*fn exec_call(&mut self, id: HIDIOCommandID, data: Vec<u8>) -> HIDIOPacketBuffer {
            self.send_command(id, data);
            let result = self.recv_packet();
            return result;
    }

    fn get_supported_ids(&mut self, id: HIDIOPropertyID) -> Vec<u8> {
            let result = self.exec_call(HIDIOCommandID::GetProperties, vec![]);
            return result.data;
    }

    fn get_property(&mut self, id: HIDIOPropertyID) -> Vec<u8> {
        let result = self.exec_call(HIDIOCommandID::GetProperties, vec![id as u8]);
        return result.data;
    }*/
}

#[derive(Debug, Clone)]
pub struct HIDIOMessage {
    pub device: String,
    pub message: HIDIOPacketBuffer,
}

pub struct HIDIOMailer {
    devices: HashMap<String, HIDIOQueue>,
    incoming: std::sync::mpsc::Receiver<HIDIOMessage>,
    outgoing: Vec<std::sync::mpsc::Sender<HIDIOMessage>>,
}

impl HIDIOMailer {
    pub fn new(incoming: std::sync::mpsc::Receiver<HIDIOMessage>) -> HIDIOMailer {
        let devices = HashMap::new();
	let outgoing = vec![];
        HIDIOMailer { devices, incoming, outgoing }
    }

    pub fn register_device(&mut self, device: HIDIOQueue) {
        //println!("Registering device");
        self.devices.insert("device".to_string(), device);
    }

    pub fn register_listener(&mut self, sink: std::sync::mpsc::Sender<HIDIOMessage>) {
	self.outgoing.push(sink);
    }

    /*fn devices(&self) -> Iterator {
        self.devices.keys().into()
    }/

    fn borrow_device(&self, device: &str) -> Option<&HIDIOQueue> {
        self.devices.get(device)
    }

    fn recv_from(&mut self, device: &str) -> HIDIOPacketBuffer {
        let device = self.devices.get(device).unwrap();
        device.recv_packet()
    }

    fn send_to(&mut self, device: &str, packet: HIDIOPacketBuffer) {
        if let Some(device) = self.devices.get(device) {
            device.send_packet(packet);
        }
    }*/

    pub fn process(&mut self) {
        for (device, queue) in self.devices.iter_mut() {
            for message in queue.messages() {
		let m = HIDIOMessage {
                    device: device.to_string(),
                    message
                };
                //println!("Sending message to device");
		for sink in self.outgoing.iter() {
		    //println!("Sending to {:?}", i);
		    sink.send(m.clone()).unwrap();
		}
            }
        }
        
        for message in self.incoming.try_iter() {
            // self.send_to(&message.device, message.message);
            //println!("Adding new incoming message to mailbox");
            let device = &self.devices[&message.device];
            let ret = device.send_packet(message.message);
            if ret.is_err() {
		println!("Device queue disconnected. Unregistering.");
                self.devices.remove(&message.device);
            }
        }
    }
}

pub struct HIDIOMailbox {
    incoming: std::sync::mpsc::Receiver<HIDIOMessage>,
    outgoing: std::sync::mpsc::Sender<HIDIOMessage>,
}

impl HIDIOMailbox {
    pub fn new(incoming: std::sync::mpsc::Receiver<HIDIOMessage>, outgoing: std::sync::mpsc::Sender<HIDIOMessage>) -> HIDIOMailbox {
        HIDIOMailbox { incoming, outgoing }
    }

    pub fn from_sender(dest: mpsc::Sender<HIDIOMessage>) -> (mpsc::Sender<HIDIOMessage>, HIDIOMailbox) {
	let (writer, reader) = channel::<HIDIOMessage>();
	let mailbox = HIDIOMailbox::new(reader, dest);
	(writer, mailbox)
    }

    pub fn send_packet(&self, device: String, packet: HIDIOPacketBuffer) {
        let result = self.outgoing.send(HIDIOMessage {
            device,
            message: packet
        });
        if let Err(e) = result {
            error!("send_packet {}", e);
        }
    }

    fn recv(&self) -> HIDIOMessage {
        self.incoming.recv().unwrap()
    }

    pub fn recv_psuedoblocking(&self) -> Option<HIDIOMessage> {
        match self.incoming.recv_timeout(Duration::from_millis(1)) {
	    Ok(m) => { Some(m) },
	    Err(mpsc::RecvTimeoutError::Timeout) => { None },
	    Err(mpsc::RecvTimeoutError::Disconnected) => {
		warn!("Lost socket"); // TODO: pass warning down
                std::process::exit(1);
		None
	    }
	}
    }
    
    pub fn iter(&self) -> mpsc::Iter<HIDIOMessage> {
	self.incoming.iter()
    }

    pub fn send_sync(&self, device: String) {
        self.send_packet(device, HIDIOPacketBuffer {
                ptype:   HIDIOPacketType::Sync,
                id:      0,
                max_len: 64, //..Defaults
                data:    vec![],
                done:    true,
        });
    }

    pub fn send_ack(&self, device: String, _id: u32, data: Vec<u8>) {
        self.send_packet(device, HIDIOPacketBuffer {
                ptype:   HIDIOPacketType::ACK,
                id:      0, // id,
                max_len: 64, //..Defaults
                data,
                done:    true,
        });
    }

    pub fn send_nack(&self, device: String, id: u32, data: Vec<u8>) {
        self.send_packet(device, HIDIOPacketBuffer {
                ptype:   HIDIOPacketType::NAK,
                id,
                max_len: 64, //..Defaults
                data,
                done:    true,
        });
    }

    pub fn send_command(&self, device: String, id: HIDIOCommandID, data: Vec<u8>) {
        self.send_packet(device, HIDIOPacketBuffer {
                ptype:   HIDIOPacketType::Data,
                id:      id as u32,
                max_len: 64, //..Defaults
                data,
                done:    true,
        });
    }
}

fn device_name(device_info: &hidapi::HidDeviceInfo) -> String {
	let mut string = format!("[{:04x}:{:04x}] ", device_info.vendor_id, device_info.product_id);
	if let Some(m) = &device_info.manufacturer_string {
		string += &m;
	}
	if let Some(p) = &device_info.product_string {
		string += &format!(" {}", p);
	}
	if let Some(s) = &device_info.serial_number {
		string += &format!(" ({})", s);
	}
	string
}

/// hidusb processing
///
/// This thread periodically refreshes the USB device list to see if a new device needs to be attached
/// The thread also handles reading/writing from connected interfaces
///
/// XXX (HaaTa) hidapi is not thread-safe on all platforms, so don't try to create a thread per device
fn processing(mut mailer: HIDIOMailer) {
    info!("Spawning hidusb spawning thread...");

    // Initialize HID interface
    let mut api = hidapi::HidApi::new().expect("HID API object creation failed");

    let mut devices: Vec<HIDIOController> = vec![];

        let mut last_scan = Instant::now();
	let mut enumerate = true;

    // Loop infinitely, the watcher only exits if the daemon is quit
    loop {
    while enumerate {
	last_scan = Instant::now();

        // Refresh devices list
        api.refresh_devices().unwrap();

        // Iterate over found USB interfaces and select usable ones
        info!("Scanning for devices");
        for device_info in api.devices() {
            debug!("{:#x?}", device_info);

            // TODO (HaaTa) Do not use vid, pid + interface number to do match
            // Instead use:
            // 1) bInterfaceClass 0x03 (HID) + bInterfaceSubClass 0x00 (None) + bInterfaceProtocol 0x00 (None)
            // 2) 2 endpoints, EP IN + EP OUT (both Interrupt)
            // 3) iInterface, RawIO API Interface
            if !(device_info.vendor_id == DEV_VID
                && device_info.product_id == DEV_PID
                && device_info.interface_number == INTERFACE_NUMBER)
            {
                continue;
            }

            // Add device
            info!("Connecting to {:#?}", device_info);

            // Add to connected list
            let path = device_info.path.clone();

            // Connect to device
            match api.open_path(&path) {
                Ok(device) => {
                    // Process device
                    println!("Connected to {}", device_name(device_info));
		    let device = HIDUSBDevice::new(device_info.clone(), device);
                    let mut device = HIDIOEndpoint::new(Box::new(device), USB_FULLSPEED_PACKET_SIZE as u32);

                    let (message_tx, message_rx) = channel::<HIDIOPacketBuffer>();
                    let (response_tx, response_rx) = channel::<HIDIOPacketBuffer>();
                    device.send_sync();

                    let master = HIDIOController::new(device, message_tx, response_rx);
                    devices.push(master);

                    let device = HIDIOQueue::new(message_rx, response_tx);
                    mailer.register_device(device);

                    /*match process_device(device) {
                        Ok(_result) => {}
                        Err(e) => {
                            // Remove problematic devices, will be re-added on the next loop if available
                            warn!("{} {:#x?}", e, device_info);
                            break;
                        }
                    };*/
                }
                Err(e) => {
                    // Could not open device (likely removed, or in use)
                    warn!("{}", e);
                    break;
                }
            };
	}

	if !devices.is_empty() {
	    info!("Enumeration finished");
	    enumerate = false;
	    break;
	}

	// Sleep so we don't starve the CPU
	// TODO (HaaTa) - There should be a better way to watch the ports, but still be responsive
	thread::sleep(Duration::from_millis(ENUMERATE_DELAY));
    }

    loop {
        // TODO: Handle device disconnect
	if devices.is_empty() {
		info!("No connected devices. Forcing scan");
		enumerate = true;
		break;
	}

        if last_scan.elapsed().as_secs() >= 60 {
	    info!("Been a while. Checking for new devices");
	    enumerate = true;
	    break;
	}

        devices = devices.drain_filter(|dev| {
            let ret = dev.process();
		if ret.is_err() {
			info!("Device disconnected. No loneger polling it");
		}
	    !ret.is_err()
        }).collect::<Vec<_>>();

        mailer.process();
        
        // TODO (HaaTa) - If there was any IO, on any of the devices, do not sleep, only sleep when all devices are idle
	thread::sleep(Duration::from_millis(POLL_DELAY));
    }
    }
}

/// hidusb initialization
///
/// Sets up a processing thread for hidusb.
pub fn initialize(mailer: HIDIOMailer) {
    info!("Initializing device/hidusb...");

    //processing(mailer);

    // Spawn watcher thread
    thread::Builder::new().name("hidusb".to_string()).spawn(|| {
	processing(mailer)
    }).unwrap();
}
