#[cfg(target_os = "linux")]
use hid_io::module::unicode::x11::*;
use hid_io::module::unicode::UnicodeOutput;

pub fn main() {
    #[cfg(target_os = "linux")]
    let mut connection = XConnection::new();
    #[cfg(target_os = "linux")]
    connection.type_string("💣💩🔥");
}
