use hid_io::module::unicode::x11::*;
use hid_io::module::unicode::UnicodeOutput;

pub fn main() {
    let mut connection = XConnection::new();
    connection.type_string("💣💩🔥");
}
