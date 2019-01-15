pub mod x11;

pub trait UnicodeOutput {
    fn press_symbol(&self, c: char, state: bool);
    fn type_string(&self, string: &str);
}

