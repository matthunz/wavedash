use std::ffi::{CStr, CString};

#[link(wasm_import_module = "wavedash")]
extern "C" {
    fn _log(ptr: u32, len: u32);

    fn _world_resource(ptr: u32, len: u32) -> u32;
}

pub fn log(msg: impl AsRef<str>) {
    let s = CString::new(msg.as_ref()).unwrap();
    unsafe { _log(s.as_ptr() as _, s.as_bytes().len() as _) };
}

pub fn world_resource(msg: impl AsRef<str>) {
    let s = CString::new(msg.as_ref()).unwrap();

    unsafe {
        let ptr = _world_resource(s.as_ptr() as _, s.count_bytes() as _);
        let cstr = CStr::from_ptr(ptr as _);
        log(cstr.to_str().unwrap());
    };

    log("B");
}
