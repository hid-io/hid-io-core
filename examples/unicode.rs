extern crate hid_io;
extern crate x11;

use hid_io::module::unicode::x11::*;
use hid_io::module::unicode::UnicodeOutput;

pub fn main() {
    let connection = XConnection::new();
    connection.type_string("💣💩🔥");
}
