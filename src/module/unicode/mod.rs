pub mod x11;

pub trait UnicodeOutput {
    fn type_string(&mut self, string: &str);
    fn press_symbol(&mut self, c: char, state: bool);
    fn get_held(&mut self) -> Vec<char>;
    fn set_held(&mut self, string: &str);
}
