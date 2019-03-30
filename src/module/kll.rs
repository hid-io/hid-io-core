use libc::c_int;
use std::ffi::CStr;

#[no_mangle]
pub extern "C" fn hello_world(x: i32) -> i32 {
    println!("Starting test");
    test_ffi();
    println!("Ending test");
    x * 2
}

#[no_mangle]
pub extern "C" fn my_callback(command: *const libc::c_char, args: *const libc::c_char) {
    let command: &CStr = unsafe { CStr::from_ptr(command) };
    let args: &CStr = unsafe { CStr::from_ptr(args) };
    match command.to_str().unwrap() {
        "serial_write" => println!("[kiibohd] {}", args.to_str().unwrap()),
        _ => println!("callback {:?} ({:?})", command, args),
    };
}

#[link(name = "kiibohd")]
extern {
    fn Host_register_callback(func: extern fn(*const libc::c_char, args: *const libc::c_char)) -> c_int;
    fn Host_callback_test() -> c_int;
    fn Host_init() -> c_int;
}

pub fn test_ffi() {
    unsafe {
        Host_register_callback(my_callback);
        Host_callback_test();
        Host_init();
    }
}

