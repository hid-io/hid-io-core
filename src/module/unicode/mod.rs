#[cfg(target_os = "linux")]
pub mod x11;

#[cfg(target_os = "windows")]
pub mod winapi;

#[cfg(target_os = "macos")]
pub mod osx;

pub trait UnicodeOutput {
    fn get_layout(&self) -> String;
    fn set_layout(&self, layout: &str);
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
    fn get_layout(&self) -> String {
        warn!("Unimplimented");
        "".into()
    }
    fn set_layout(&self, layout: &str) {
        warn!("Unimplimented");
    }
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
