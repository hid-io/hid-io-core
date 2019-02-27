#[cfg(target_os = "linux")]
pub mod x11;

pub trait UnicodeOutput {
    fn type_string(&mut self, string: &str);
    fn press_symbol(&mut self, c: char, state: bool);
    fn get_held(&mut self) -> Vec<char>;
    fn set_held(&mut self, string: &str);
}

pub struct StubOutput {
}

impl StubOutput {
    pub fn new() -> StubOutput {
        StubOutput { }
    }
}

impl UnicodeOutput for StubOutput {
    fn type_string(&mut self, string: &str) {
        warn!("Unimplimented");
    }
    fn press_symbol(&mut self, c: char, state: bool) {
        warn!("Unimplimented");
    }
    fn get_held(&mut self) -> Vec<char> {
        warn!("Unimplimented");
        vec![]
    }
    fn set_held(&mut self, string: &str) {
        warn!("Unimplimented");
    }
}
