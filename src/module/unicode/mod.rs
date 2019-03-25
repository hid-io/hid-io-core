#[cfg(target_os = "linux")]
/// Xorg impementation
pub mod x11;

#[cfg(target_os = "windows")]
/// Winapi impementation
pub mod winapi;

#[cfg(target_os = "macos")]
/// Osx quartz impementation
pub mod osx;

/// Functions that can be called in a cross platform manner
pub trait UnicodeOutput {
    fn get_layout(&self) -> String;
    fn set_layout(&self, layout: &str);
    fn type_string(&mut self, string: &str);
    fn press_symbol(&mut self, c: char, state: bool);
    fn get_held(&mut self) -> Vec<char>;
    fn set_held(&mut self, string: &str);
}

#[derive(Default)]
/// Dummy impementation for unsupported platforms
pub struct StubOutput {}

impl StubOutput {
    pub fn new() -> StubOutput {
        StubOutput {}
    }
}

impl UnicodeOutput for StubOutput {
    fn get_layout(&self) -> String {
        warn!("Unimplimented");
        "".into()
    }
    fn set_layout(&self, _layout: &str) {
        warn!("Unimplimented");
    }
    fn type_string(&mut self, _string: &str) {
        warn!("Unimplimented");
    }
    fn press_symbol(&mut self, _c: char, _state: bool) {
        warn!("Unimplimented");
    }
    fn get_held(&mut self) -> Vec<char> {
        warn!("Unimplimented");
        vec![]
    }
    fn set_held(&mut self, _string: &str) {
        warn!("Unimplimented");
    }
}
