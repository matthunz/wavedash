use bevy_ecs::system::Resource;
use bevy_reflect::TypePath;
use serde::de::DeserializeOwned;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fmt;
use wavedash_core::{Request, Response};

#[link(wasm_import_module = "__wavedash__")]
extern "C" {
    fn __wavedash_log(ptr: i32, len: i32) -> i32;

    fn __wavedash_request(ptr: i32, len: i32) -> i32;
}

thread_local! {
    static STORAGE: RefCell<HashMap<i32, Vec<u8>>> = RefCell::new(HashMap::new());
}

#[no_mangle]
extern "C" fn __wavedash_alloc(len: i32) -> i32 {
    STORAGE
        .try_with(|storage| {
            let mut storage = storage.borrow_mut();
            let buf = vec![0; len as usize];
            let ptr = buf.as_ptr() as _;
            storage.insert(ptr, buf);
            ptr
        })
        .unwrap()
}

fn request(req: &Request) -> Response {
    let s = CString::new(serde_json::to_string(req).unwrap()).unwrap();
    let ptr = unsafe { __wavedash_request(s.as_ptr() as _, s.as_bytes().len() as i32) };

    let s = unsafe { CStr::from_ptr(ptr as _) }.to_str().unwrap();
    let response = serde_json::from_str(s).unwrap();

    STORAGE
        .try_with(|storage| {
            storage.borrow_mut().remove(&ptr).unwrap();
        })
        .unwrap();

    response
}

pub fn dbg(s: impl fmt::Debug) {
    request(&Request::Log(format!("{:?}", s)));
}

pub fn log(s: impl fmt::Display) {
    request(&Request::Log(s.to_string()));
}

pub fn resource<R>() -> R
where
    R: Resource + TypePath + DeserializeOwned,
{
    let type_path = R::type_path().to_string();
    let res = request(&Request::GetResource { type_path });
    match res {
        Response::Resource(json) => {
            log(&json);
            serde_json::from_value(json).unwrap()
        }
        _ => unimplemented!(),
    }
}
