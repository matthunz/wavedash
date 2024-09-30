use serde::de::DeserializeOwned;
use serde_json::Value;
use std::ffi::{CStr, CString};

pub use wavedash_core::Named;

#[link(wasm_import_module = "wavedash")]
extern "C" {
    fn _log(ptr: u32, len: u32);

    fn _world_resource(ptr: u32, len: u32) -> u32;
}

pub fn log(msg: impl AsRef<str>) {
    let s = CString::new(msg.as_ref()).unwrap();
    unsafe { _log(s.as_ptr() as _, s.as_bytes().len() as _) };
}

pub struct App {
    world: World,
}

impl App {
    pub fn current() -> Self {
        Self {
            world: World { _priv: () },
        }
    }
}

impl App {
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }
}

pub struct World {
    _priv: (),
}

impl World {
    pub fn resource<T: Named + DeserializeOwned>(&self) -> T {
        T::deserialize(self.resource_by_id(T::name())).unwrap()
    }

    pub fn resource_by_id(&self, name: impl AsRef<str>) -> Value {
        let s = CString::new(name.as_ref()).unwrap();

        let cstr = unsafe {
            let ptr = _world_resource(s.as_ptr() as _, s.count_bytes() as _);
            CStr::from_ptr(ptr as _)
        };

        let s = cstr.to_str().unwrap();
        serde_json::from_str(s).unwrap()
    }
}
